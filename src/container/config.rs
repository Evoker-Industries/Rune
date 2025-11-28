//! Container configuration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Container status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContainerStatus {
    /// Container is being created
    Creating,
    /// Container is created but not running
    Created,
    /// Container is running
    Running,
    /// Container is paused
    Paused,
    /// Container has stopped
    Stopped,
    /// Container has exited
    Exited,
    /// Container is being removed
    Removing,
    /// Container is in an error state
    Dead,
}

impl std::fmt::Display for ContainerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerStatus::Creating => write!(f, "creating"),
            ContainerStatus::Created => write!(f, "created"),
            ContainerStatus::Running => write!(f, "running"),
            ContainerStatus::Paused => write!(f, "paused"),
            ContainerStatus::Stopped => write!(f, "stopped"),
            ContainerStatus::Exited => write!(f, "exited"),
            ContainerStatus::Removing => write!(f, "removing"),
            ContainerStatus::Dead => write!(f, "dead"),
        }
    }
}

/// Container configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Unique container ID
    pub id: String,
    /// Container name
    pub name: String,
    /// Image name/tag
    pub image: String,
    /// Command to run
    pub cmd: Vec<String>,
    /// Entry point
    pub entrypoint: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Working directory
    pub working_dir: String,
    /// User to run as
    pub user: String,
    /// Exposed ports
    pub exposed_ports: Vec<PortMapping>,
    /// Volume mounts
    pub volumes: Vec<VolumeMount>,
    /// Container labels
    pub labels: HashMap<String, String>,
    /// Hostname
    pub hostname: String,
    /// Domain name
    pub domainname: String,
    /// Network mode
    pub network_mode: String,
    /// Privileged mode
    pub privileged: bool,
    /// Read-only root filesystem
    pub read_only_rootfs: bool,
    /// Resource limits
    pub resources: ResourceLimits,
    /// Current status
    pub status: ContainerStatus,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Start time
    pub started_at: Option<DateTime<Utc>>,
    /// Stop time
    pub finished_at: Option<DateTime<Utc>>,
    /// Exit code
    pub exit_code: Option<i32>,
    /// Process ID
    pub pid: Option<u32>,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string().replace("-", "")[..12].to_string(),
            name: String::new(),
            image: String::new(),
            cmd: Vec::new(),
            entrypoint: Vec::new(),
            env: HashMap::new(),
            working_dir: "/".to_string(),
            user: String::new(),
            exposed_ports: Vec::new(),
            volumes: Vec::new(),
            labels: HashMap::new(),
            hostname: String::new(),
            domainname: String::new(),
            network_mode: "bridge".to_string(),
            privileged: false,
            read_only_rootfs: false,
            resources: ResourceLimits::default(),
            status: ContainerStatus::Creating,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            exit_code: None,
            pid: None,
        }
    }
}

impl ContainerConfig {
    /// Create a new container configuration
    pub fn new(name: &str, image: &str) -> Self {
        let mut config = Self::default();
        config.name = name.to_string();
        config.image = image.to_string();
        config.hostname = name.to_string();
        config
    }

    /// Set command to run
    pub fn cmd(mut self, cmd: Vec<String>) -> Self {
        self.cmd = cmd;
        self
    }

    /// Add environment variable
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }

    /// Set working directory
    pub fn working_dir(mut self, dir: &str) -> Self {
        self.working_dir = dir.to_string();
        self
    }

    /// Add port mapping
    pub fn port(mut self, host_port: u16, container_port: u16) -> Self {
        self.exposed_ports.push(PortMapping {
            host_port,
            container_port,
            protocol: Protocol::Tcp,
        });
        self
    }

    /// Add volume mount
    pub fn volume(mut self, host_path: &str, container_path: &str) -> Self {
        self.volumes.push(VolumeMount {
            host_path: host_path.to_string(),
            container_path: container_path.to_string(),
            read_only: false,
        });
        self
    }
}

/// Port mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: Protocol,
}

/// Network protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
}

/// Volume mount
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
}

/// Resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Memory limit in bytes
    pub memory_limit: Option<u64>,
    /// Memory reservation in bytes
    pub memory_reservation: Option<u64>,
    /// CPU shares
    pub cpu_shares: Option<u64>,
    /// CPU quota
    pub cpu_quota: Option<i64>,
    /// CPU period
    pub cpu_period: Option<u64>,
    /// Number of CPUs
    pub cpus: Option<f64>,
    /// PIDs limit
    pub pids_limit: Option<i64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_limit: None,
            memory_reservation: None,
            cpu_shares: None,
            cpu_quota: None,
            cpu_period: None,
            cpus: None,
            pids_limit: None,
        }
    }
}
