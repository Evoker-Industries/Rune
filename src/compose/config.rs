//! Docker Compose configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Docker Compose file configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeConfig {
    /// Compose file version
    #[serde(default)]
    pub version: Option<String>,
    /// Service name (for top-level name)
    #[serde(default)]
    pub name: Option<String>,
    /// Services
    #[serde(default)]
    pub services: HashMap<String, ServiceConfig>,
    /// Networks
    #[serde(default)]
    pub networks: HashMap<String, NetworkConfig>,
    /// Volumes
    #[serde(default)]
    pub volumes: HashMap<String, VolumeConfig>,
    /// Secrets
    #[serde(default)]
    pub secrets: HashMap<String, SecretConfig>,
    /// Configs
    #[serde(default)]
    pub configs: HashMap<String, ConfigConfig>,
}

impl Default for ComposeConfig {
    fn default() -> Self {
        Self {
            version: Some("3.8".to_string()),
            name: None,
            services: HashMap::new(),
            networks: HashMap::new(),
            volumes: HashMap::new(),
            secrets: HashMap::new(),
            configs: HashMap::new(),
        }
    }
}

/// Service configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Image name
    #[serde(default)]
    pub image: Option<String>,
    /// Build configuration
    #[serde(default)]
    pub build: Option<BuildConfig>,
    /// Command to run
    #[serde(default)]
    pub command: Option<CommandConfig>,
    /// Entrypoint
    #[serde(default)]
    pub entrypoint: Option<CommandConfig>,
    /// Container name
    #[serde(default)]
    pub container_name: Option<String>,
    /// Hostname
    #[serde(default)]
    pub hostname: Option<String>,
    /// Domain name
    #[serde(default)]
    pub domainname: Option<String>,
    /// Environment variables
    #[serde(default)]
    pub environment: Option<EnvironmentConfig>,
    /// Environment file
    #[serde(default)]
    pub env_file: Option<EnvFileConfig>,
    /// Exposed ports
    #[serde(default)]
    pub expose: Option<Vec<String>>,
    /// Port mappings
    #[serde(default)]
    pub ports: Option<Vec<PortConfig>>,
    /// Volume mounts
    #[serde(default)]
    pub volumes: Option<Vec<VolumeMount>>,
    /// Networks to connect to
    #[serde(default)]
    pub networks: Option<NetworksConfig>,
    /// Service dependencies
    #[serde(default)]
    pub depends_on: Option<DependsOnConfig>,
    /// Deploy configuration
    #[serde(default)]
    pub deploy: Option<DeployConfig>,
    /// Healthcheck configuration
    #[serde(default)]
    pub healthcheck: Option<HealthcheckConfig>,
    /// Labels
    #[serde(default)]
    pub labels: Option<LabelsConfig>,
    /// Logging configuration
    #[serde(default)]
    pub logging: Option<LoggingConfig>,
    /// Restart policy
    #[serde(default)]
    pub restart: Option<String>,
    /// Working directory
    #[serde(default)]
    pub working_dir: Option<String>,
    /// User
    #[serde(default)]
    pub user: Option<String>,
    /// Privileged mode
    #[serde(default)]
    pub privileged: Option<bool>,
    /// Read only root filesystem
    #[serde(default)]
    pub read_only: Option<bool>,
    /// Stdin open
    #[serde(default)]
    pub stdin_open: Option<bool>,
    /// TTY
    #[serde(default)]
    pub tty: Option<bool>,
    /// Stop signal
    #[serde(default)]
    pub stop_signal: Option<String>,
    /// Stop grace period
    #[serde(default)]
    pub stop_grace_period: Option<String>,
    /// Sysctls
    #[serde(default)]
    pub sysctls: Option<HashMap<String, String>>,
    /// Ulimits
    #[serde(default)]
    pub ulimits: Option<HashMap<String, UlimitConfig>>,
    /// Extra hosts
    #[serde(default)]
    pub extra_hosts: Option<Vec<String>>,
    /// DNS servers
    #[serde(default)]
    pub dns: Option<Vec<String>>,
    /// DNS search domains
    #[serde(default)]
    pub dns_search: Option<Vec<String>>,
    /// Capabilities to add
    #[serde(default)]
    pub cap_add: Option<Vec<String>>,
    /// Capabilities to drop
    #[serde(default)]
    pub cap_drop: Option<Vec<String>>,
    /// Security options
    #[serde(default)]
    pub security_opt: Option<Vec<String>>,
    /// Secrets
    #[serde(default)]
    pub secrets: Option<Vec<SecretRef>>,
    /// Configs
    #[serde(default)]
    pub configs: Option<Vec<ConfigRef>>,
    /// Devices
    #[serde(default)]
    pub devices: Option<Vec<String>>,
    /// Init process
    #[serde(default)]
    pub init: Option<bool>,
    /// IPC mode
    #[serde(default)]
    pub ipc: Option<String>,
    /// PID mode
    #[serde(default)]
    pub pid: Option<String>,
    /// Network mode
    #[serde(default)]
    pub network_mode: Option<String>,
    /// Profiles
    #[serde(default)]
    pub profiles: Option<Vec<String>>,
    /// Pull policy
    #[serde(default)]
    pub pull_policy: Option<String>,
    /// Platform
    #[serde(default)]
    pub platform: Option<String>,
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum BuildConfig {
    /// Simple context path
    Simple(String),
    /// Full build configuration
    Full(BuildConfigFull),
}

/// Full build configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildConfigFull {
    /// Build context
    pub context: Option<String>,
    /// Dockerfile path (or Runefile)
    pub dockerfile: Option<String>,
    /// Build arguments
    #[serde(default)]
    pub args: Option<HashMap<String, String>>,
    /// Target stage
    pub target: Option<String>,
    /// Cache from images
    #[serde(default)]
    pub cache_from: Option<Vec<String>>,
    /// Cache to
    pub cache_to: Option<String>,
    /// Extra hosts
    #[serde(default)]
    pub extra_hosts: Option<Vec<String>>,
    /// Labels
    #[serde(default)]
    pub labels: Option<HashMap<String, String>>,
    /// Network
    pub network: Option<String>,
    /// SSH
    #[serde(default)]
    pub ssh: Option<Vec<String>>,
    /// Secrets
    #[serde(default)]
    pub secrets: Option<Vec<String>>,
    /// Tags
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Platforms
    #[serde(default)]
    pub platforms: Option<Vec<String>>,
    /// Privileged
    pub privileged: Option<bool>,
    /// No cache
    pub no_cache: Option<bool>,
    /// Pull
    pub pull: Option<bool>,
}

/// Command configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CommandConfig {
    /// Shell command string
    Shell(String),
    /// Exec form array
    Exec(Vec<String>),
}

/// Environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EnvironmentConfig {
    /// Array of KEY=value strings
    Array(Vec<String>),
    /// Map of key to value
    Map(HashMap<String, Option<String>>),
}

/// Env file configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EnvFileConfig {
    /// Single file
    Single(String),
    /// Multiple files
    Multiple(Vec<String>),
}

/// Port configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PortConfig {
    /// Short syntax: "8080:80"
    Short(String),
    /// Long syntax
    Long(PortConfigLong),
}

/// Long port configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PortConfigLong {
    /// Target port in container
    pub target: u16,
    /// Published port on host
    pub published: Option<String>,
    /// Host IP to bind to
    pub host_ip: Option<String>,
    /// Protocol (tcp/udp)
    pub protocol: Option<String>,
    /// Mode (host/ingress)
    pub mode: Option<String>,
}

/// Volume mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VolumeMount {
    /// Short syntax: "host:container:mode"
    Short(String),
    /// Long syntax
    Long(VolumeMountLong),
}

/// Long volume mount configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumeMountLong {
    /// Mount type (volume, bind, tmpfs, npipe)
    #[serde(rename = "type")]
    pub mount_type: Option<String>,
    /// Source path or volume name
    pub source: Option<String>,
    /// Target path in container
    pub target: String,
    /// Read only
    pub read_only: Option<bool>,
    /// Bind options
    pub bind: Option<BindOptions>,
    /// Volume options
    pub volume: Option<VolumeOptions>,
    /// Tmpfs options
    pub tmpfs: Option<TmpfsOptions>,
    /// Consistency
    pub consistency: Option<String>,
}

/// Bind mount options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BindOptions {
    /// Propagation mode
    pub propagation: Option<String>,
    /// Create host path
    pub create_host_path: Option<bool>,
    /// SELinux relabeling
    pub selinux: Option<String>,
}

/// Volume options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumeOptions {
    /// No copy data from container
    pub nocopy: Option<bool>,
}

/// Tmpfs options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TmpfsOptions {
    /// Size in bytes
    pub size: Option<u64>,
    /// Mode
    pub mode: Option<u32>,
}

/// Networks configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NetworksConfig {
    /// Array of network names
    Array(Vec<String>),
    /// Map of network name to config
    Map(HashMap<String, Option<ServiceNetworkConfig>>),
}

/// Service network configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceNetworkConfig {
    /// Aliases
    #[serde(default)]
    pub aliases: Option<Vec<String>>,
    /// IPv4 address
    pub ipv4_address: Option<String>,
    /// IPv6 address
    pub ipv6_address: Option<String>,
    /// Link local IPs
    #[serde(default)]
    pub link_local_ips: Option<Vec<String>>,
    /// Priority
    pub priority: Option<i32>,
}

/// Depends on configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependsOnConfig {
    /// Array of service names
    Array(Vec<String>),
    /// Map of service to condition
    Map(HashMap<String, DependsOnCondition>),
}

/// Depends on condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependsOnCondition {
    /// Condition to wait for
    pub condition: String,
}

/// Deploy configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeployConfig {
    /// Deployment mode (replicated, global)
    pub mode: Option<String>,
    /// Number of replicas
    pub replicas: Option<u32>,
    /// Placement constraints
    pub placement: Option<PlacementConfig>,
    /// Resource limits and reservations
    pub resources: Option<ResourcesConfig>,
    /// Restart policy
    pub restart_policy: Option<RestartPolicyConfig>,
    /// Update configuration
    pub update_config: Option<UpdateConfig>,
    /// Rollback configuration
    pub rollback_config: Option<UpdateConfig>,
    /// Labels
    #[serde(default)]
    pub labels: Option<LabelsConfig>,
    /// Endpoint mode
    pub endpoint_mode: Option<String>,
}

/// Placement configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlacementConfig {
    /// Constraints
    #[serde(default)]
    pub constraints: Option<Vec<String>>,
    /// Preferences
    #[serde(default)]
    pub preferences: Option<Vec<PlacementPreference>>,
    /// Max replicas per node
    pub max_replicas_per_node: Option<u32>,
}

/// Placement preference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementPreference {
    /// Spread strategy
    pub spread: Option<String>,
}

/// Resources configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourcesConfig {
    /// Resource limits
    pub limits: Option<ResourceSpec>,
    /// Resource reservations
    pub reservations: Option<ResourceSpec>,
}

/// Resource specification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceSpec {
    /// CPU limit/reservation
    pub cpus: Option<String>,
    /// Memory limit/reservation
    pub memory: Option<String>,
    /// PIDs limit
    pub pids: Option<i64>,
    /// Generic resources
    #[serde(default)]
    pub devices: Option<Vec<DeviceSpec>>,
}

/// Device specification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceSpec {
    /// Capabilities
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
    /// Driver
    pub driver: Option<String>,
    /// Count
    pub count: Option<i64>,
    /// Device IDs
    #[serde(default)]
    pub device_ids: Option<Vec<String>>,
    /// Options
    #[serde(default)]
    pub options: Option<HashMap<String, String>>,
}

/// Restart policy configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RestartPolicyConfig {
    /// Condition (none, on-failure, any)
    pub condition: Option<String>,
    /// Delay between retries
    pub delay: Option<String>,
    /// Maximum attempts
    pub max_attempts: Option<u32>,
    /// Window for counting retries
    pub window: Option<String>,
}

/// Update/rollback configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Parallelism
    pub parallelism: Option<u32>,
    /// Delay between updates
    pub delay: Option<String>,
    /// Failure action
    pub failure_action: Option<String>,
    /// Monitor duration
    pub monitor: Option<String>,
    /// Max failure ratio
    pub max_failure_ratio: Option<f64>,
    /// Order (start-first, stop-first)
    pub order: Option<String>,
}

/// Healthcheck configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthcheckConfig {
    /// Test command
    pub test: Option<HealthcheckTest>,
    /// Interval
    pub interval: Option<String>,
    /// Timeout
    pub timeout: Option<String>,
    /// Retries
    pub retries: Option<u32>,
    /// Start period
    pub start_period: Option<String>,
    /// Disable healthcheck
    pub disable: Option<bool>,
}

/// Healthcheck test
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HealthcheckTest {
    /// Command string
    Command(String),
    /// Command array
    Array(Vec<String>),
}

/// Labels configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LabelsConfig {
    /// Array of "key=value" strings
    Array(Vec<String>),
    /// Map of key to value
    Map(HashMap<String, String>),
}

/// Logging configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Driver
    pub driver: Option<String>,
    /// Options
    #[serde(default)]
    pub options: Option<HashMap<String, String>>,
}

/// Ulimit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UlimitConfig {
    /// Single value (same for soft and hard)
    Single(i64),
    /// Separate soft and hard limits
    SoftHard { soft: i64, hard: i64 },
}

/// Network configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Driver
    pub driver: Option<String>,
    /// Driver options
    #[serde(default)]
    pub driver_opts: Option<HashMap<String, String>>,
    /// IPAM configuration
    pub ipam: Option<IpamConfig>,
    /// External network
    pub external: Option<ExternalConfig>,
    /// Internal network
    pub internal: Option<bool>,
    /// Attachable
    pub attachable: Option<bool>,
    /// Labels
    #[serde(default)]
    pub labels: Option<LabelsConfig>,
    /// Enable IPv6
    pub enable_ipv6: Option<bool>,
    /// Name
    pub name: Option<String>,
}

/// IPAM configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IpamConfig {
    /// Driver
    pub driver: Option<String>,
    /// Config blocks
    #[serde(default)]
    pub config: Option<Vec<IpamPoolConfig>>,
    /// Options
    #[serde(default)]
    pub options: Option<HashMap<String, String>>,
}

/// IPAM pool configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IpamPoolConfig {
    /// Subnet
    pub subnet: Option<String>,
    /// IP range
    pub ip_range: Option<String>,
    /// Gateway
    pub gateway: Option<String>,
    /// Auxiliary addresses
    #[serde(default)]
    pub aux_addresses: Option<HashMap<String, String>>,
}

/// Volume configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumeConfig {
    /// Driver
    pub driver: Option<String>,
    /// Driver options
    #[serde(default)]
    pub driver_opts: Option<HashMap<String, String>>,
    /// External volume
    pub external: Option<ExternalConfig>,
    /// Labels
    #[serde(default)]
    pub labels: Option<LabelsConfig>,
    /// Name
    pub name: Option<String>,
}

/// External resource configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExternalConfig {
    /// Boolean
    Bool(bool),
    /// With name
    Named { name: String },
}

/// Secret configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecretConfig {
    /// File path
    pub file: Option<String>,
    /// Environment variable
    pub environment: Option<String>,
    /// External secret
    pub external: Option<ExternalConfig>,
    /// Name
    pub name: Option<String>,
}

/// Config configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigConfig {
    /// File path
    pub file: Option<String>,
    /// External config
    pub external: Option<ExternalConfig>,
    /// Name
    pub name: Option<String>,
}

/// Secret reference in service
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SecretRef {
    /// Short syntax
    Short(String),
    /// Long syntax
    Long(SecretRefLong),
}

/// Long secret reference
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecretRefLong {
    /// Source secret name
    pub source: String,
    /// Target path in container
    pub target: Option<String>,
    /// UID
    pub uid: Option<String>,
    /// GID
    pub gid: Option<String>,
    /// Mode
    pub mode: Option<u32>,
}

/// Config reference in service
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigRef {
    /// Short syntax
    Short(String),
    /// Long syntax
    Long(ConfigRefLong),
}

/// Long config reference
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigRefLong {
    /// Source config name
    pub source: String,
    /// Target path in container
    pub target: Option<String>,
    /// UID
    pub uid: Option<String>,
    /// GID
    pub gid: Option<String>,
    /// Mode
    pub mode: Option<u32>,
}
