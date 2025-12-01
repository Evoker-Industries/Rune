//! Linux namespace management
//!
//! Provides functionality for creating and managing Linux namespaces
//! for container isolation.

use super::syscall::{clone_flags, unshare};
use crate::error::{Result, RuneError};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

/// Types of Linux namespaces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceType {
    /// Mount namespace - isolates filesystem mount points
    Mount,
    /// UTS namespace - isolates hostname and domain name
    Uts,
    /// IPC namespace - isolates System V IPC and POSIX message queues
    Ipc,
    /// Network namespace - isolates network devices, ports, etc.
    Net,
    /// PID namespace - isolates process IDs
    Pid,
    /// User namespace - isolates user and group IDs
    User,
    /// Cgroup namespace - isolates cgroup root directory
    Cgroup,
}

impl NamespaceType {
    /// Get the clone flag for this namespace type
    pub fn clone_flag(&self) -> i32 {
        match self {
            NamespaceType::Mount => clone_flags::CLONE_NEWNS,
            NamespaceType::Uts => clone_flags::CLONE_NEWUTS,
            NamespaceType::Ipc => clone_flags::CLONE_NEWIPC,
            NamespaceType::Net => clone_flags::CLONE_NEWNET,
            NamespaceType::Pid => clone_flags::CLONE_NEWPID,
            NamespaceType::User => clone_flags::CLONE_NEWUSER,
            NamespaceType::Cgroup => clone_flags::CLONE_NEWCGROUP,
        }
    }

    /// Get the namespace file name in /proc/[pid]/ns/
    pub fn proc_name(&self) -> &'static str {
        match self {
            NamespaceType::Mount => "mnt",
            NamespaceType::Uts => "uts",
            NamespaceType::Ipc => "ipc",
            NamespaceType::Net => "net",
            NamespaceType::Pid => "pid",
            NamespaceType::User => "user",
            NamespaceType::Cgroup => "cgroup",
        }
    }

    /// Get all namespace types
    pub fn all() -> Vec<NamespaceType> {
        vec![
            NamespaceType::User,
            NamespaceType::Mount,
            NamespaceType::Uts,
            NamespaceType::Ipc,
            NamespaceType::Net,
            NamespaceType::Pid,
            NamespaceType::Cgroup,
        ]
    }
}

/// Represents a Linux namespace
#[derive(Debug)]
pub struct Namespace {
    /// Type of namespace
    ns_type: NamespaceType,
    /// Process ID this namespace is associated with (if any)
    pid: Option<u32>,
}

impl Namespace {
    /// Create a new namespace reference
    pub fn new(ns_type: NamespaceType) -> Self {
        Self { ns_type, pid: None }
    }

    /// Create a namespace reference for a specific process
    pub fn for_process(ns_type: NamespaceType, pid: u32) -> Self {
        Self {
            ns_type,
            pid: Some(pid),
        }
    }

    /// Get the namespace type
    pub fn ns_type(&self) -> NamespaceType {
        self.ns_type
    }

    /// Get the path to the namespace file
    pub fn path(&self) -> String {
        let pid = self
            .pid
            .map(|p| p.to_string())
            .unwrap_or_else(|| "self".to_string());
        format!("/proc/{}/ns/{}", pid, self.ns_type.proc_name())
    }

    /// Check if the namespace exists
    pub fn exists(&self) -> bool {
        Path::new(&self.path()).exists()
    }

    /// Get the namespace inode number (unique identifier)
    pub fn inode(&self) -> Result<u64> {
        let path = self.path();
        let metadata = std::fs::metadata(&path)
            .map_err(|e| RuneError::Runtime(format!("Failed to get namespace metadata: {}", e)))?;

        use std::os::unix::fs::MetadataExt;
        Ok(metadata.ino())
    }
}

/// Namespace manager for creating and managing container namespaces
pub struct NamespaceManager {
    /// Container ID
    container_id: String,
}

impl NamespaceManager {
    /// Create a new namespace manager
    pub fn new(container_id: &str) -> Self {
        Self {
            container_id: container_id.to_string(),
        }
    }

    /// Create new namespaces by unsharing from the current process
    pub fn unshare(&self, namespaces: &[NamespaceType]) -> Result<()> {
        let mut flags = 0i32;
        for ns_type in namespaces {
            flags |= ns_type.clone_flag();
        }

        unshare(flags)
            .map_err(|e| RuneError::Runtime(format!("Failed to unshare namespaces: {}", e)))
    }

    /// Get combined clone flags for a list of namespace types
    pub fn get_clone_flags(&self, namespaces: &[NamespaceType]) -> i32 {
        namespaces.iter().fold(0, |acc, ns| acc | ns.clone_flag())
    }

    /// Set up user namespace mappings
    pub fn setup_user_namespace(&self, pid: u32, uid_map: &str, gid_map: &str) -> Result<()> {
        // Write uid_map
        let uid_map_path = format!("/proc/{}/uid_map", pid);
        let mut uid_file = OpenOptions::new()
            .write(true)
            .open(&uid_map_path)
            .map_err(|e| RuneError::Runtime(format!("Failed to open uid_map: {}", e)))?;
        uid_file
            .write_all(uid_map.as_bytes())
            .map_err(|e| RuneError::Runtime(format!("Failed to write uid_map: {}", e)))?;

        // Write setgroups deny (required before gid_map in some cases)
        let setgroups_path = format!("/proc/{}/setgroups", pid);
        if Path::new(&setgroups_path).exists() {
            let mut setgroups_file = OpenOptions::new()
                .write(true)
                .open(&setgroups_path)
                .map_err(|e| RuneError::Runtime(format!("Failed to open setgroups: {}", e)))?;
            setgroups_file
                .write_all(b"deny")
                .map_err(|e| RuneError::Runtime(format!("Failed to write setgroups: {}", e)))?;
        }

        // Write gid_map
        let gid_map_path = format!("/proc/{}/gid_map", pid);
        let mut gid_file = OpenOptions::new()
            .write(true)
            .open(&gid_map_path)
            .map_err(|e| RuneError::Runtime(format!("Failed to open gid_map: {}", e)))?;
        gid_file
            .write_all(gid_map.as_bytes())
            .map_err(|e| RuneError::Runtime(format!("Failed to write gid_map: {}", e)))?;

        Ok(())
    }

    /// Set the hostname for the UTS namespace
    pub fn set_hostname(&self, hostname: &str) -> Result<()> {
        super::syscall::sethostname(hostname)
            .map_err(|e| RuneError::Runtime(format!("Failed to set hostname: {}", e)))
    }

    /// Set the domain name for the UTS namespace
    pub fn set_domainname(&self, domainname: &str) -> Result<()> {
        super::syscall::setdomainname(domainname)
            .map_err(|e| RuneError::Runtime(format!("Failed to set domainname: {}", e)))
    }

    /// Get container ID
    pub fn container_id(&self) -> &str {
        &self.container_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_type_clone_flags() {
        assert_eq!(NamespaceType::Mount.clone_flag(), clone_flags::CLONE_NEWNS);
        assert_eq!(NamespaceType::Uts.clone_flag(), clone_flags::CLONE_NEWUTS);
        assert_eq!(NamespaceType::Pid.clone_flag(), clone_flags::CLONE_NEWPID);
        assert_eq!(NamespaceType::Net.clone_flag(), clone_flags::CLONE_NEWNET);
        assert_eq!(NamespaceType::User.clone_flag(), clone_flags::CLONE_NEWUSER);
    }

    #[test]
    fn test_namespace_proc_name() {
        assert_eq!(NamespaceType::Mount.proc_name(), "mnt");
        assert_eq!(NamespaceType::Uts.proc_name(), "uts");
        assert_eq!(NamespaceType::Pid.proc_name(), "pid");
        assert_eq!(NamespaceType::Net.proc_name(), "net");
        assert_eq!(NamespaceType::User.proc_name(), "user");
    }

    #[test]
    fn test_namespace_path() {
        let ns = Namespace::new(NamespaceType::Pid);
        assert_eq!(ns.path(), "/proc/self/ns/pid");

        let ns = Namespace::for_process(NamespaceType::Net, 1234);
        assert_eq!(ns.path(), "/proc/1234/ns/net");
    }

    #[test]
    fn test_namespace_exists() {
        // /proc/self/ns/pid should exist on Linux
        let ns = Namespace::new(NamespaceType::Pid);
        // This may fail in non-Linux environments, so we just ensure no panic
        let _ = ns.exists();
    }

    #[test]
    fn test_namespace_manager_clone_flags() {
        let manager = NamespaceManager::new("test-container");
        let namespaces = vec![NamespaceType::Pid, NamespaceType::Net, NamespaceType::Mount];
        let flags = manager.get_clone_flags(&namespaces);

        assert!(flags & clone_flags::CLONE_NEWPID != 0);
        assert!(flags & clone_flags::CLONE_NEWNET != 0);
        assert!(flags & clone_flags::CLONE_NEWNS != 0);
    }
}
