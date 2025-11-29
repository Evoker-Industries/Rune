//! Container process management
//!
//! Provides functionality for creating and managing container processes
//! with proper namespace isolation.

use crate::error::{Result, RuneError};
use super::namespace::{NamespaceType, NamespaceManager};
use super::cgroup::{CgroupManager, CgroupConfig};
use super::mount::MountManager;
use super::syscall;
use std::collections::HashMap;
use std::path::PathBuf;

/// Process configuration for a container
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    /// Arguments (first is the executable)
    pub args: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Working directory
    pub cwd: String,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Additional groups
    pub groups: Vec<u32>,
    /// Terminal
    pub terminal: bool,
    /// Capabilities to add
    pub capabilities_add: Vec<String>,
    /// Capabilities to drop
    pub capabilities_drop: Vec<String>,
    /// No new privileges flag
    pub no_new_privileges: bool,
    /// OOM score adjustment
    pub oom_score_adj: Option<i32>,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            args: Vec::new(),
            env: HashMap::new(),
            cwd: "/".to_string(),
            uid: 0,
            gid: 0,
            groups: Vec::new(),
            terminal: false,
            capabilities_add: Vec::new(),
            capabilities_drop: Vec::new(),
            no_new_privileges: true,
            oom_score_adj: None,
        }
    }
}

impl ProcessConfig {
    /// Create a new process config with the given command
    pub fn new(args: Vec<String>) -> Self {
        Self {
            args,
            ..Default::default()
        }
    }

    /// Set the working directory
    pub fn cwd(mut self, cwd: &str) -> Self {
        self.cwd = cwd.to_string();
        self
    }

    /// Set the user ID
    pub fn uid(mut self, uid: u32) -> Self {
        self.uid = uid;
        self
    }

    /// Set the group ID
    pub fn gid(mut self, gid: u32) -> Self {
        self.gid = gid;
        self
    }

    /// Add an environment variable
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }

    /// Set all environment variables
    pub fn envs(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Enable terminal
    pub fn terminal(mut self, terminal: bool) -> Self {
        self.terminal = terminal;
        self
    }
}

/// Container process state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// Process is being created
    Creating,
    /// Process is created but not started
    Created,
    /// Process is running
    Running,
    /// Process has stopped
    Stopped,
    /// Process has exited
    Exited,
}

/// Represents a container process
pub struct ContainerProcess {
    /// Process configuration
    config: ProcessConfig,
    /// Namespaces to create
    namespaces: Vec<NamespaceType>,
    /// Process ID (set after fork)
    pid: Option<u32>,
    /// Exit code (set after process exits)
    exit_code: Option<i32>,
    /// Current state
    state: ProcessState,
    /// Root filesystem path
    rootfs: Option<PathBuf>,
    /// Container ID
    container_id: Option<String>,
}

impl ContainerProcess {
    /// Create a new container process
    pub fn new(config: ProcessConfig, namespaces: Vec<NamespaceType>) -> Result<Self> {
        Ok(Self {
            config,
            namespaces,
            pid: None,
            exit_code: None,
            state: ProcessState::Creating,
            rootfs: None,
            container_id: None,
        })
    }

    /// Set the root filesystem
    pub fn set_rootfs(&mut self, rootfs: PathBuf) {
        self.rootfs = Some(rootfs);
    }

    /// Set the container ID
    pub fn set_container_id(&mut self, id: String) {
        self.container_id = Some(id);
    }

    /// Get the process ID
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    /// Get the exit code
    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    /// Get the current state
    pub fn state(&self) -> ProcessState {
        self.state
    }

    /// Get the process configuration
    pub fn config(&self) -> &ProcessConfig {
        &self.config
    }

    /// Start the container process
    /// 
    /// This will fork the current process and set up namespaces, cgroups,
    /// and the root filesystem in the child process.
    pub fn start(&mut self) -> Result<u32> {
        // Calculate clone flags from namespaces
        let ns_manager = NamespaceManager::new(
            self.container_id.as_deref().unwrap_or("unknown")
        );
        let clone_flags = ns_manager.get_clone_flags(&self.namespaces);

        // Fork the process with new namespaces
        let pid = self.fork_with_namespaces(clone_flags)?;

        if pid == 0 {
            // Child process
            self.child_process()?;
            std::process::exit(0);
        } else {
            // Parent process
            self.pid = Some(pid);
            self.state = ProcessState::Running;
            
            // Set up user namespace mappings if needed
            if self.namespaces.contains(&NamespaceType::User) {
                // Map root in container to current user
                let uid = unsafe { libc::getuid() };
                let gid = unsafe { libc::getgid() };
                let uid_map = format!("0 {} 1", uid);
                let gid_map = format!("0 {} 1", gid);
                
                // Give the child time to start
                std::thread::sleep(std::time::Duration::from_millis(10));
                
                let _ = ns_manager.setup_user_namespace(pid, &uid_map, &gid_map);
            }

            Ok(pid)
        }
    }

    /// Fork with new namespaces using clone
    fn fork_with_namespaces(&self, flags: i32) -> Result<u32> {
        // Use fork for simplicity; clone would require more setup
        let pid = syscall::fork()
            .map_err(|e| RuneError::Runtime(format!("Failed to fork: {}", e)))?;

        if pid == 0 {
            // Child: unshare namespaces
            syscall::unshare(flags)
                .map_err(|e| RuneError::Runtime(format!("Failed to unshare: {}", e)))?;
        }

        Ok(pid)
    }

    /// Child process setup
    fn child_process(&self) -> Result<()> {
        // Set hostname if UTS namespace is used
        if self.namespaces.contains(&NamespaceType::Uts) {
            let hostname = self.container_id.as_deref().unwrap_or("rune-container");
            let _ = syscall::sethostname(&hostname[..std::cmp::min(64, hostname.len())]);
        }

        // Set up root filesystem if specified
        if let Some(ref rootfs) = self.rootfs {
            let mount_manager = MountManager::new();
            let rootfs_str = rootfs.to_string_lossy();
            
            // Setup rootfs with essential mounts
            mount_manager.setup_rootfs(&rootfs_str)?;
            
            // Create devices
            mount_manager.create_devices(&rootfs_str)?;
            
            // Pivot to new root
            mount_manager.pivot_root(&rootfs_str, "/.pivot_root")?;
        }

        // Change to working directory
        let _ = syscall::chdir(&self.config.cwd);

        // Set UID/GID
        if self.config.gid != 0 {
            let _ = syscall::setgid(self.config.gid);
        }
        if self.config.uid != 0 {
            let _ = syscall::setuid(self.config.uid);
        }

        // Execute the command
        if !self.config.args.is_empty() {
            let args: Vec<&str> = self.config.args.iter().map(|s| s.as_str()).collect();
            let env: Vec<String> = self.config.env.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            let env_refs: Vec<&str> = env.iter().map(|s| s.as_str()).collect();

            syscall::execve(&args[0], &args, &env_refs)
                .map_err(|e| RuneError::Runtime(format!("Failed to exec: {}", e)))?;
        }

        Ok(())
    }

    /// Wait for the process to exit
    pub fn wait(&mut self) -> Result<i32> {
        if let Some(pid) = self.pid {
            let (_, status) = syscall::waitpid(pid as i32, 0)
                .map_err(|e| RuneError::Runtime(format!("Failed to wait: {}", e)))?;
            
            // Extract exit code from status
            let exit_code = if libc::WIFEXITED(status) {
                libc::WEXITSTATUS(status)
            } else if libc::WIFSIGNALED(status) {
                128 + libc::WTERMSIG(status)
            } else {
                -1
            };

            self.exit_code = Some(exit_code);
            self.state = ProcessState::Exited;
            
            Ok(exit_code)
        } else {
            Err(RuneError::Runtime("Process not started".to_string()))
        }
    }

    /// Send a signal to the process
    pub fn kill(&mut self, signal: i32) -> Result<()> {
        if let Some(pid) = self.pid {
            syscall::kill(pid as i32, signal)
                .map_err(|e| RuneError::Runtime(format!("Failed to kill: {}", e)))?;
            
            if signal == libc::SIGKILL || signal == libc::SIGTERM {
                self.state = ProcessState::Stopped;
            }
            
            Ok(())
        } else {
            Err(RuneError::Runtime("Process not started".to_string()))
        }
    }

    /// Check if the process is running
    pub fn is_running(&self) -> bool {
        if let Some(pid) = self.pid {
            // Check if process exists
            match syscall::kill(pid as i32, 0) {
                Ok(_) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }
}

/// Exec into a running container
pub struct ContainerExec {
    /// Target container PID
    container_pid: u32,
    /// Process configuration
    config: ProcessConfig,
}

impl ContainerExec {
    /// Create a new exec into a container
    pub fn new(container_pid: u32, config: ProcessConfig) -> Self {
        Self {
            container_pid,
            config,
        }
    }

    /// Execute a command in the container's namespaces
    pub fn exec(&self) -> Result<u32> {
        // Fork first
        let pid = syscall::fork()
            .map_err(|e| RuneError::Runtime(format!("Failed to fork: {}", e)))?;

        if pid == 0 {
            // Child process: enter namespaces
            self.enter_namespaces()?;
            
            // Execute the command
            if !self.config.args.is_empty() {
                let args: Vec<&str> = self.config.args.iter().map(|s| s.as_str()).collect();
                let env: Vec<String> = self.config.env.iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect();
                let env_refs: Vec<&str> = env.iter().map(|s| s.as_str()).collect();

                syscall::execve(&args[0], &args, &env_refs)
                    .map_err(|e| RuneError::Runtime(format!("Failed to exec: {}", e)))?;
            }
            
            std::process::exit(0);
        }

        Ok(pid)
    }

    /// Enter the container's namespaces
    fn enter_namespaces(&self) -> Result<()> {
        use std::fs::File;
        use std::os::unix::io::AsRawFd;

        use super::syscall::clone_flags;
        
        let ns_types = [
            ("user", libc::CLONE_NEWUSER),
            ("mnt", libc::CLONE_NEWNS),
            ("uts", libc::CLONE_NEWUTS),
            ("ipc", libc::CLONE_NEWIPC),
            ("net", libc::CLONE_NEWNET),
            ("pid", libc::CLONE_NEWPID),
            ("cgroup", clone_flags::CLONE_NEWCGROUP),
        ];

        for (ns_name, ns_flag) in ns_types {
            let ns_path = format!("/proc/{}/ns/{}", self.container_pid, ns_name);
            
            if let Ok(file) = File::open(&ns_path) {
                let fd = file.as_raw_fd();
                let result = unsafe { libc::setns(fd, ns_flag) };
                if result < 0 {
                    tracing::warn!("Failed to enter {} namespace", ns_name);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_config_creation() {
        let config = ProcessConfig::new(vec!["/bin/sh".to_string()]);
        assert_eq!(config.args, vec!["/bin/sh"]);
        assert_eq!(config.cwd, "/");
        assert_eq!(config.uid, 0);
        assert_eq!(config.gid, 0);
    }

    #[test]
    fn test_process_config_builder() {
        let config = ProcessConfig::new(vec!["/bin/sh".to_string()])
            .cwd("/home")
            .uid(1000)
            .gid(1000)
            .env("PATH", "/usr/bin")
            .terminal(true);

        assert_eq!(config.cwd, "/home");
        assert_eq!(config.uid, 1000);
        assert_eq!(config.gid, 1000);
        assert_eq!(config.env.get("PATH"), Some(&"/usr/bin".to_string()));
        assert!(config.terminal);
    }

    #[test]
    fn test_container_process_creation() {
        let config = ProcessConfig::new(vec!["/bin/sh".to_string()]);
        let namespaces = vec![NamespaceType::Pid, NamespaceType::Mount];
        let process = ContainerProcess::new(config, namespaces);
        
        assert!(process.is_ok());
        let process = process.unwrap();
        assert_eq!(process.state(), ProcessState::Creating);
        assert!(process.pid().is_none());
    }

    #[test]
    fn test_process_state() {
        assert_eq!(ProcessState::Creating, ProcessState::Creating);
        assert_ne!(ProcessState::Running, ProcessState::Stopped);
    }
}
