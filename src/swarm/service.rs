//! Swarm service management

use super::task::{Task, TaskState};
use crate::error::{Result, RuneError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Swarm service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    /// Service ID
    pub id: String,
    /// Service specification
    pub spec: ServiceSpec,
    /// Previous spec (for rollback)
    pub previous_spec: Option<ServiceSpec>,
    /// Service endpoint
    pub endpoint: Endpoint,
    /// Update status
    pub update_status: Option<UpdateStatus>,
    /// Service version
    pub version: ServiceVersion,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl Service {
    /// Create a new service
    pub fn new(spec: ServiceSpec) -> Self {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        Self {
            id,
            spec,
            previous_spec: None,
            endpoint: Endpoint::default(),
            update_status: None,
            version: ServiceVersion { index: 1 },
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the service specification
    pub fn update(&mut self, new_spec: ServiceSpec) {
        self.previous_spec = Some(self.spec.clone());
        self.spec = new_spec;
        self.version.index += 1;
        self.updated_at = Utc::now();
        self.update_status = Some(UpdateStatus {
            state: "updating".to_string(),
            started_at: Some(Utc::now()),
            completed_at: None,
            message: String::new(),
        });
    }

    /// Rollback to previous specification
    pub fn rollback(&mut self) -> Result<()> {
        if let Some(prev) = self.previous_spec.take() {
            self.previous_spec = Some(self.spec.clone());
            self.spec = prev;
            self.version.index += 1;
            self.updated_at = Utc::now();
            self.update_status = Some(UpdateStatus {
                state: "rollback_started".to_string(),
                started_at: Some(Utc::now()),
                completed_at: None,
                message: String::new(),
            });
            Ok(())
        } else {
            Err(RuneError::Service(
                "No previous specification to rollback to".to_string(),
            ))
        }
    }

    /// Scale the service
    pub fn scale(&mut self, replicas: u64) {
        if let Some(ref mut mode) = self.spec.mode {
            if let ServiceMode::Replicated {
                replicas: ref mut r,
            } = mode
            {
                *r = replicas;
            }
        }
        self.version.index += 1;
        self.updated_at = Utc::now();
    }

    /// Get replica count
    pub fn replicas(&self) -> u64 {
        self.spec
            .mode
            .as_ref()
            .map(|m| match m {
                ServiceMode::Replicated { replicas } => *replicas,
                ServiceMode::Global => 0,
                ServiceMode::ReplicatedJob {
                    max_concurrent,
                    total_completions,
                } => *total_completions,
                ServiceMode::GlobalJob => 0,
            })
            .unwrap_or(1)
    }
}

/// Service specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSpec {
    /// Service name
    pub name: String,
    /// Labels
    #[serde(default)]
    pub labels: HashMap<String, String>,
    /// Task template
    pub task_template: TaskSpec,
    /// Service mode
    pub mode: Option<ServiceMode>,
    /// Update config
    pub update_config: Option<UpdateConfig>,
    /// Rollback config
    pub rollback_config: Option<UpdateConfig>,
    /// Networks
    #[serde(default)]
    pub networks: Vec<NetworkAttachmentConfig>,
    /// Endpoint specification
    pub endpoint_spec: Option<EndpointSpec>,
}

impl Default for ServiceSpec {
    fn default() -> Self {
        Self {
            name: String::new(),
            labels: HashMap::new(),
            task_template: TaskSpec::default(),
            mode: Some(ServiceMode::Replicated { replicas: 1 }),
            update_config: None,
            rollback_config: None,
            networks: Vec::new(),
            endpoint_spec: None,
        }
    }
}

/// Task specification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskSpec {
    /// Plugin spec (for plugin tasks)
    pub plugin_spec: Option<PluginSpec>,
    /// Container spec
    pub container_spec: Option<ContainerSpec>,
    /// Network attachment spec
    pub network_attachment_spec: Option<NetworkAttachmentSpec>,
    /// Resources
    pub resources: Option<ResourceRequirements>,
    /// Restart policy
    pub restart_policy: Option<RestartPolicy>,
    /// Placement
    pub placement: Option<Placement>,
    /// Force update
    pub force_update: Option<u64>,
    /// Runtime
    pub runtime: Option<String>,
    /// Networks (deprecated, use ServiceSpec.networks)
    #[serde(default)]
    pub networks: Vec<NetworkAttachmentConfig>,
    /// Log driver
    pub log_driver: Option<LogDriver>,
}

/// Plugin specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSpec {
    /// Plugin name
    pub name: String,
    /// Plugin remote
    pub remote: String,
    /// Disabled
    pub disabled: bool,
    /// Plugin privileges
    pub plugin_privilege: Vec<PluginPrivilege>,
}

/// Plugin privilege
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPrivilege {
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Value
    pub value: Vec<String>,
}

/// Container specification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContainerSpec {
    /// Image
    pub image: String,
    /// Labels
    #[serde(default)]
    pub labels: HashMap<String, String>,
    /// Command
    #[serde(default)]
    pub command: Vec<String>,
    /// Args
    #[serde(default)]
    pub args: Vec<String>,
    /// Hostname
    pub hostname: Option<String>,
    /// Environment variables
    #[serde(default)]
    pub env: Vec<String>,
    /// Working directory
    pub dir: Option<String>,
    /// User
    pub user: Option<String>,
    /// Groups
    #[serde(default)]
    pub groups: Vec<String>,
    /// Privileges
    pub privileges: Option<Privileges>,
    /// TTY
    pub tty: Option<bool>,
    /// Open stdin
    pub open_stdin: Option<bool>,
    /// Read only
    pub read_only: Option<bool>,
    /// Mounts
    #[serde(default)]
    pub mounts: Vec<Mount>,
    /// Stop signal
    pub stop_signal: Option<String>,
    /// Stop grace period
    pub stop_grace_period: Option<i64>,
    /// Health check
    pub health_check: Option<HealthConfig>,
    /// Hosts
    #[serde(default)]
    pub hosts: Vec<String>,
    /// DNS config
    pub dns_config: Option<DnsConfig>,
    /// Secrets
    #[serde(default)]
    pub secrets: Vec<SecretReference>,
    /// Configs
    #[serde(default)]
    pub configs: Vec<ConfigReference>,
    /// Isolation
    pub isolation: Option<String>,
    /// Init
    pub init: Option<bool>,
    /// Sysctls
    #[serde(default)]
    pub sysctls: HashMap<String, String>,
    /// Capability add
    #[serde(default)]
    pub cap_add: Vec<String>,
    /// Capability drop
    #[serde(default)]
    pub cap_drop: Vec<String>,
    /// Ulimits
    #[serde(default)]
    pub ulimits: Vec<Ulimit>,
}

/// Privileges configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Privileges {
    /// Credential spec
    pub credential_spec: Option<CredentialSpec>,
    /// SELinux context
    pub selinux_context: Option<SelinuxContext>,
}

/// Credential specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialSpec {
    /// Config
    pub config: Option<String>,
    /// File
    pub file: Option<String>,
    /// Registry
    pub registry: Option<String>,
}

/// SELinux context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelinuxContext {
    /// Disable
    pub disable: bool,
    /// User
    pub user: Option<String>,
    /// Role
    pub role: Option<String>,
    /// Type
    #[serde(rename = "type")]
    pub selinux_type: Option<String>,
    /// Level
    pub level: Option<String>,
}

/// Mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mount {
    /// Target
    pub target: String,
    /// Source
    pub source: Option<String>,
    /// Type
    #[serde(rename = "type")]
    pub mount_type: String,
    /// Read only
    pub read_only: Option<bool>,
    /// Consistency
    pub consistency: Option<String>,
    /// Bind options
    pub bind_options: Option<BindOptions>,
    /// Volume options
    pub volume_options: Option<VolumeOptions>,
    /// Tmpfs options
    pub tmpfs_options: Option<TmpfsOptions>,
}

/// Bind options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BindOptions {
    /// Propagation
    pub propagation: Option<String>,
    /// Non recursive
    pub non_recursive: Option<bool>,
}

/// Volume options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumeOptions {
    /// No copy
    pub no_copy: Option<bool>,
    /// Labels
    #[serde(default)]
    pub labels: HashMap<String, String>,
    /// Driver config
    pub driver_config: Option<DriverConfig>,
}

/// Driver configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DriverConfig {
    /// Name
    pub name: Option<String>,
    /// Options
    #[serde(default)]
    pub options: HashMap<String, String>,
}

/// Tmpfs options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TmpfsOptions {
    /// Size
    pub size_bytes: Option<i64>,
    /// Mode
    pub mode: Option<u32>,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Test
    pub test: Vec<String>,
    /// Interval
    pub interval: Option<i64>,
    /// Timeout
    pub timeout: Option<i64>,
    /// Retries
    pub retries: Option<i32>,
    /// Start period
    pub start_period: Option<i64>,
}

/// DNS configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DnsConfig {
    /// Nameservers
    #[serde(default)]
    pub nameservers: Vec<String>,
    /// Search
    #[serde(default)]
    pub search: Vec<String>,
    /// Options
    #[serde(default)]
    pub options: Vec<String>,
}

/// Secret reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretReference {
    /// File
    pub file: Option<SecretFile>,
    /// Secret ID
    pub secret_id: String,
    /// Secret name
    pub secret_name: String,
}

/// Secret file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretFile {
    /// Name
    pub name: String,
    /// UID
    pub uid: Option<String>,
    /// GID
    pub gid: Option<String>,
    /// Mode
    pub mode: Option<u32>,
}

/// Config reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigReference {
    /// File
    pub file: Option<ConfigFile>,
    /// Runtime (for runtime configs)
    pub runtime: Option<RuntimeConfig>,
    /// Config ID
    pub config_id: String,
    /// Config name
    pub config_name: String,
}

/// Config file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    /// Name
    pub name: String,
    /// UID
    pub uid: Option<String>,
    /// GID
    pub gid: Option<String>,
    /// Mode
    pub mode: Option<u32>,
}

/// Runtime config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {}

/// Ulimit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ulimit {
    /// Name
    pub name: String,
    /// Soft limit
    pub soft: i64,
    /// Hard limit
    pub hard: i64,
}

/// Network attachment specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAttachmentSpec {
    /// Container ID
    pub container_id: String,
}

/// Resource requirements
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// Limits
    pub limits: Option<ResourceSpec>,
    /// Reservations
    pub reservations: Option<ResourceSpec>,
}

/// Resource specification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceSpec {
    /// CPU limit in nanoCPUs
    pub nano_cpus: Option<i64>,
    /// Memory limit in bytes
    pub memory_bytes: Option<i64>,
    /// Pids limit
    pub pids: Option<i64>,
    /// Generic resources
    #[serde(default)]
    pub generic_resources: Vec<GenericResource>,
}

/// Generic resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericResource {
    /// Named resource spec
    pub named_resource_spec: Option<NamedResourceSpec>,
    /// Discrete resource spec
    pub discrete_resource_spec: Option<DiscreteResourceSpec>,
}

/// Named resource spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedResourceSpec {
    /// Kind
    pub kind: String,
    /// Value
    pub value: String,
}

/// Discrete resource spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscreteResourceSpec {
    /// Kind
    pub kind: String,
    /// Value
    pub value: i64,
}

/// Restart policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartPolicy {
    /// Condition
    pub condition: Option<String>,
    /// Delay
    pub delay: Option<i64>,
    /// Max attempts
    pub max_attempts: Option<u64>,
    /// Window
    pub window: Option<i64>,
}

/// Placement configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Placement {
    /// Constraints
    #[serde(default)]
    pub constraints: Vec<String>,
    /// Preferences
    #[serde(default)]
    pub preferences: Vec<PlacementPreference>,
    /// Max replicas per node
    pub max_replicas: Option<u64>,
    /// Platforms
    #[serde(default)]
    pub platforms: Vec<Platform>,
}

/// Placement preference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementPreference {
    /// Spread
    pub spread: Option<SpreadOver>,
}

/// Spread over
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadOver {
    /// Spread descriptor
    pub spread_descriptor: String,
}

/// Platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Platform {
    /// Architecture
    pub architecture: Option<String>,
    /// OS
    pub os: Option<String>,
}

/// Log driver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogDriver {
    /// Name
    pub name: String,
    /// Options
    #[serde(default)]
    pub options: HashMap<String, String>,
}

/// Service mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ServiceMode {
    /// Replicated service
    Replicated { replicas: u64 },
    /// Global service (one per node)
    Global,
    /// Replicated job
    ReplicatedJob {
        max_concurrent: u64,
        total_completions: u64,
    },
    /// Global job
    GlobalJob,
}

/// Update configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Parallelism
    pub parallelism: Option<u64>,
    /// Delay
    pub delay: Option<i64>,
    /// Failure action
    pub failure_action: Option<String>,
    /// Monitor
    pub monitor: Option<i64>,
    /// Max failure ratio
    pub max_failure_ratio: Option<f64>,
    /// Order
    pub order: Option<String>,
}

/// Network attachment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAttachmentConfig {
    /// Target
    pub target: String,
    /// Aliases
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Driver opts
    #[serde(default)]
    pub driver_opts: HashMap<String, String>,
}

/// Endpoint specification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EndpointSpec {
    /// Mode (vip, dnsrr)
    pub mode: Option<String>,
    /// Ports
    #[serde(default)]
    pub ports: Vec<PortConfig>,
}

/// Port configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortConfig {
    /// Name
    pub name: Option<String>,
    /// Protocol
    pub protocol: Option<String>,
    /// Target port
    pub target_port: u16,
    /// Published port
    pub published_port: Option<u16>,
    /// Publish mode
    pub publish_mode: Option<String>,
}

/// Endpoint
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Endpoint {
    /// Spec
    pub spec: Option<EndpointSpec>,
    /// Ports
    #[serde(default)]
    pub ports: Vec<PortConfig>,
    /// Virtual IPs
    #[serde(default)]
    pub virtual_ips: Vec<VirtualIP>,
}

/// Virtual IP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualIP {
    /// Network ID
    pub network_id: String,
    /// Address
    pub addr: String,
}

/// Update status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
    /// State
    pub state: String,
    /// Started at
    pub started_at: Option<DateTime<Utc>>,
    /// Completed at
    pub completed_at: Option<DateTime<Utc>>,
    /// Message
    pub message: String,
}

/// Service version
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceVersion {
    /// Index
    pub index: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_service() {
        let spec = ServiceSpec {
            name: "web".to_string(),
            task_template: TaskSpec {
                container_spec: Some(ContainerSpec {
                    image: "nginx:latest".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let service = Service::new(spec);
        assert_eq!(service.spec.name, "web");
        assert_eq!(service.replicas(), 1);
    }

    #[test]
    fn test_scale_service() {
        let spec = ServiceSpec {
            name: "web".to_string(),
            ..Default::default()
        };

        let mut service = Service::new(spec);
        service.scale(5);

        assert_eq!(service.replicas(), 5);
    }

    #[test]
    fn test_update_rollback() {
        let spec1 = ServiceSpec {
            name: "web".to_string(),
            task_template: TaskSpec {
                container_spec: Some(ContainerSpec {
                    image: "nginx:1.0".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let mut service = Service::new(spec1);

        let spec2 = ServiceSpec {
            name: "web".to_string(),
            task_template: TaskSpec {
                container_spec: Some(ContainerSpec {
                    image: "nginx:2.0".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        service.update(spec2);
        assert_eq!(
            service
                .spec
                .task_template
                .container_spec
                .as_ref()
                .unwrap()
                .image,
            "nginx:2.0"
        );

        service.rollback().unwrap();
        assert_eq!(
            service
                .spec
                .task_template
                .container_spec
                .as_ref()
                .unwrap()
                .image,
            "nginx:1.0"
        );
    }
}
