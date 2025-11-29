//! Rune's native container runtime
//!
//! A custom container runtime implementation without external dependencies.
//! Provides Linux namespace isolation, cgroup resource management, and
//! process execution for containers.

pub mod namespace;
pub mod cgroup;
pub mod process;
pub mod syscall;
pub mod mount;

pub use namespace::{Namespace, NamespaceType};
pub use cgroup::{CgroupManager, CgroupConfig};
pub use process::{ContainerProcess, ProcessConfig};
pub use mount::MountManager;

use crate::error::{Result, RuneError};

/// Container runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Enable user namespace
    pub user_namespace: bool,
    /// Enable PID namespace
    pub pid_namespace: bool,
    /// Enable network namespace
    pub network_namespace: bool,
    /// Enable mount namespace
    pub mount_namespace: bool,
    /// Enable UTS namespace
    pub uts_namespace: bool,
    /// Enable IPC namespace
    pub ipc_namespace: bool,
    /// Enable cgroup namespace
    pub cgroup_namespace: bool,
    /// Root filesystem path
    pub rootfs: String,
    /// Hostname for the container
    pub hostname: String,
    /// Cgroup configuration
    pub cgroup: Option<CgroupConfig>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            user_namespace: true,
            pid_namespace: true,
            network_namespace: true,
            mount_namespace: true,
            uts_namespace: true,
            ipc_namespace: true,
            cgroup_namespace: true,
            rootfs: String::new(),
            hostname: String::from("rune-container"),
            cgroup: None,
        }
    }
}

/// The main container runtime
pub struct Runtime {
    /// Runtime configuration
    config: RuntimeConfig,
    /// Cgroup manager
    cgroup_manager: CgroupManager,
    /// Mount manager
    mount_manager: MountManager,
}

impl Runtime {
    /// Create a new runtime with the given configuration
    pub fn new(config: RuntimeConfig) -> Result<Self> {
        Ok(Self {
            cgroup_manager: CgroupManager::new()?,
            mount_manager: MountManager::new(),
            config,
        })
    }

    /// Create a new container process
    pub fn create_container(&self, process_config: ProcessConfig) -> Result<ContainerProcess> {
        // Build namespace flags based on configuration
        let mut namespaces = Vec::new();

        if self.config.pid_namespace {
            namespaces.push(NamespaceType::Pid);
        }
        if self.config.network_namespace {
            namespaces.push(NamespaceType::Net);
        }
        if self.config.mount_namespace {
            namespaces.push(NamespaceType::Mount);
        }
        if self.config.uts_namespace {
            namespaces.push(NamespaceType::Uts);
        }
        if self.config.ipc_namespace {
            namespaces.push(NamespaceType::Ipc);
        }
        if self.config.user_namespace {
            namespaces.push(NamespaceType::User);
        }
        if self.config.cgroup_namespace {
            namespaces.push(NamespaceType::Cgroup);
        }

        ContainerProcess::new(process_config, namespaces)
    }

    /// Setup cgroup for container
    pub fn setup_cgroup(&self, container_id: &str, config: &CgroupConfig) -> Result<()> {
        self.cgroup_manager.create(container_id, config)
    }

    /// Add process to cgroup
    pub fn add_to_cgroup(&self, container_id: &str, pid: u32) -> Result<()> {
        self.cgroup_manager.add_process(container_id, pid)
    }

    /// Remove cgroup
    pub fn remove_cgroup(&self, container_id: &str) -> Result<()> {
        self.cgroup_manager.remove(container_id)
    }

    /// Setup root filesystem
    pub fn setup_rootfs(&self, rootfs: &str) -> Result<()> {
        self.mount_manager.setup_rootfs(rootfs)
    }

    /// Pivot root to new filesystem
    pub fn pivot_root(&self, new_root: &str, put_old: &str) -> Result<()> {
        self.mount_manager.pivot_root(new_root, put_old)
    }

    /// Get runtime configuration
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let config = RuntimeConfig::default();
        let runtime = Runtime::new(config);
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_default_config() {
        let config = RuntimeConfig::default();
        assert!(config.pid_namespace);
        assert!(config.network_namespace);
        assert!(config.mount_namespace);
        assert!(config.uts_namespace);
        assert!(config.ipc_namespace);
        assert!(config.user_namespace);
        assert!(config.cgroup_namespace);
        assert_eq!(config.hostname, "rune-container");
    }
}
