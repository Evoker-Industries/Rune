//! REST API Handler for Rune Daemon
//!
//! Implements Docker Engine API v1.24+ compatible endpoints.
//! This API is compatible with Portainer and other Docker management tools.

use std::sync::Arc;
use crate::container::{ContainerManager, ContainerConfig};
use crate::error::{Result, RuneError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::debug;

/// API request/response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerCreateRequest {
    #[serde(rename = "Image")]
    pub image: String,
    #[serde(rename = "Cmd")]
    pub cmd: Option<Vec<String>>,
    #[serde(rename = "Env")]
    pub env: Option<Vec<String>>,
    #[serde(rename = "WorkingDir")]
    pub working_dir: Option<String>,
    #[serde(rename = "Hostname")]
    pub hostname: Option<String>,
    #[serde(rename = "User")]
    pub user: Option<String>,
    #[serde(rename = "Tty")]
    pub tty: Option<bool>,
    #[serde(rename = "ExposedPorts")]
    pub exposed_ports: Option<std::collections::HashMap<String, Value>>,
    #[serde(rename = "HostConfig")]
    pub host_config: Option<HostConfig>,
    #[serde(rename = "NetworkingConfig")]
    pub networking_config: Option<NetworkingConfig>,
    #[serde(rename = "Labels")]
    pub labels: Option<std::collections::HashMap<String, String>>,
}

/// Host configuration for container
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct HostConfig {
    pub binds: Option<Vec<String>>,
    pub port_bindings: Option<std::collections::HashMap<String, Vec<PortBinding>>>,
    pub network_mode: Option<String>,
    pub restart_policy: Option<RestartPolicy>,
    pub memory: Option<i64>,
    pub memory_swap: Option<i64>,
    pub cpu_shares: Option<i64>,
    pub cpu_period: Option<i64>,
    pub cpu_quota: Option<i64>,
    pub privileged: Option<bool>,
    pub publish_all_ports: Option<bool>,
    pub auto_remove: Option<bool>,
}

/// Port binding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PortBinding {
    pub host_ip: Option<String>,
    pub host_port: Option<String>,
}

/// Restart policy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RestartPolicy {
    pub name: String,
    pub maximum_retry_count: Option<i32>,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkingConfig {
    pub endpoints_config: Option<std::collections::HashMap<String, EndpointConfig>>,
}

/// Endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EndpointConfig {
    pub ipam_config: Option<IpamConfig>,
    pub aliases: Option<Vec<String>>,
}

/// IPAM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IpamConfig {
    #[serde(rename = "IPv4Address")]
    pub ipv4_address: Option<String>,
    #[serde(rename = "IPv6Address")]
    pub ipv6_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerCreateResponse {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Warnings")]
    pub warnings: Vec<String>,
}

/// Version info response - Portainer compatible
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct VersionResponse {
    version: String,
    api_version: String,
    min_api_version: String,
    go_version: String,
    git_commit: String,
    built: String,
    os: String,
    arch: String,
    kernel_version: String,
    experimental: bool,
    build_time: String,
}

/// System info response - Portainer compatible
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct InfoResponse {
    #[serde(rename = "ID")]
    id: String,
    containers: i64,
    containers_running: i64,
    containers_paused: i64,
    containers_stopped: i64,
    images: i64,
    driver: String,
    driver_status: Vec<Vec<String>>,
    #[serde(rename = "NCPU")]
    ncpu: usize,
    mem_total: i64,
    docker_root_dir: String,
    name: String,
    server_version: String,
    default_runtime: String,
    #[serde(rename = "OSType")]
    os_type: String,
    operating_system: String,
    architecture: String,
    kernel_version: String,
    experimental_build: bool,
    live_restore_enabled: bool,
    swarm: SwarmInfo,
    runtimes: std::collections::HashMap<String, RuntimeInfo>,
    security_options: Vec<String>,
    plugins: PluginsInfo,
    registries: Vec<RegistryConfig>,
}

/// Swarm info for system info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct SwarmInfo {
    #[serde(rename = "NodeID")]
    node_id: String,
    node_addr: String,
    local_node_state: String,
    control_available: bool,
    error: String,
    remote_managers: Option<Vec<RemoteManager>>,
}

/// Remote manager info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RemoteManager {
    #[serde(rename = "NodeID")]
    node_id: String,
    addr: String,
}

/// Runtime info
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeInfo {
    path: String,
    #[serde(rename = "runtimeArgs")]
    runtime_args: Option<Vec<String>>,
}

/// Plugins info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct PluginsInfo {
    volume: Vec<String>,
    network: Vec<String>,
    authorization: Option<Vec<String>>,
    log: Vec<String>,
}

/// Registry config
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryConfig {
    name: String,
    mirrors: Vec<String>,
}

/// Container list item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ContainerListItem {
    id: String,
    names: Vec<String>,
    image: String,
    #[serde(rename = "ImageID")]
    image_id: String,
    command: String,
    created: i64,
    state: String,
    status: String,
    ports: Vec<PortInfo>,
    labels: std::collections::HashMap<String, String>,
    network_settings: NetworkSettingsSummary,
    mounts: Vec<MountPoint>,
}

/// Port info for container list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PortInfo {
    #[serde(rename = "IP")]
    ip: Option<String>,
    private_port: u16,
    public_port: Option<u16>,
    #[serde(rename = "Type")]
    port_type: String,
}

/// Network settings summary for container list
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct NetworkSettingsSummary {
    networks: std::collections::HashMap<String, EndpointSettings>,
}

/// Endpoint settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct EndpointSettings {
    #[serde(rename = "IPAMConfig")]
    ipam_config: Option<Value>,
    links: Option<Vec<String>>,
    aliases: Option<Vec<String>>,
    #[serde(rename = "NetworkID")]
    network_id: String,
    #[serde(rename = "EndpointID")]
    endpoint_id: String,
    gateway: String,
    #[serde(rename = "IPAddress")]
    ip_address: String,
    #[serde(rename = "IPPrefixLen")]
    ip_prefix_len: i32,
    #[serde(rename = "IPv6Gateway")]
    ipv6_gateway: String,
    #[serde(rename = "GlobalIPv6Address")]
    global_ipv6_address: String,
    #[serde(rename = "GlobalIPv6PrefixLen")]
    global_ipv6_prefix_len: i32,
    mac_address: String,
}

/// Mount point
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MountPoint {
    #[serde(rename = "Type")]
    mount_type: String,
    name: Option<String>,
    source: String,
    destination: String,
    driver: Option<String>,
    mode: String,
    #[serde(rename = "RW")]
    rw: bool,
    propagation: String,
}

/// Container inspect response - Full Docker API compatible
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ContainerInspect {
    id: String,
    created: String,
    path: String,
    args: Vec<String>,
    state: ContainerState,
    image: String,
    name: String,
    restart_count: i32,
    driver: String,
    platform: String,
    config: ContainerConfigResponse,
    host_config: HostConfigResponse,
    network_settings: NetworkSettingsResponse,
    mounts: Vec<MountPoint>,
}

/// Container config in inspect response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ContainerConfigResponse {
    hostname: String,
    domainname: String,
    user: String,
    attach_stdin: bool,
    attach_stdout: bool,
    attach_stderr: bool,
    exposed_ports: Option<std::collections::HashMap<String, Value>>,
    tty: bool,
    open_stdin: bool,
    stdin_once: bool,
    env: Vec<String>,
    cmd: Vec<String>,
    image: String,
    volumes: Option<std::collections::HashMap<String, Value>>,
    working_dir: String,
    entrypoint: Option<Vec<String>>,
    labels: std::collections::HashMap<String, String>,
}

/// Host config in inspect response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HostConfigResponse {
    binds: Option<Vec<String>>,
    network_mode: String,
    port_bindings: Option<std::collections::HashMap<String, Vec<PortBinding>>>,
    restart_policy: RestartPolicyResponse,
    auto_remove: bool,
    privileged: bool,
    publish_all_ports: bool,
    read_only_rootfs: bool,
    memory: i64,
    memory_swap: i64,
    memory_reservation: i64,
    cpu_shares: i64,
    cpu_period: i64,
    cpu_quota: i64,
    cpuset_cpus: String,
    cpuset_mems: String,
    pids_limit: Option<i64>,
}

/// Restart policy in response
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RestartPolicyResponse {
    name: String,
    maximum_retry_count: i32,
}

/// Network settings in inspect response
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct NetworkSettingsResponse {
    bridge: String,
    #[serde(rename = "SandboxID")]
    sandbox_id: String,
    hairpin_mode: bool,
    #[serde(rename = "LinkLocalIPv6Address")]
    link_local_ipv6_address: String,
    #[serde(rename = "LinkLocalIPv6PrefixLen")]
    link_local_ipv6_prefix_len: i32,
    ports: Option<std::collections::HashMap<String, Option<Vec<PortBinding>>>>,
    sandbox_key: String,
    #[serde(rename = "SecondaryIPAddresses")]
    secondary_ip_addresses: Option<Vec<String>>,
    #[serde(rename = "SecondaryIPv6Addresses")]
    secondary_ipv6_addresses: Option<Vec<String>>,
    #[serde(rename = "EndpointID")]
    endpoint_id: String,
    gateway: String,
    #[serde(rename = "GlobalIPv6Address")]
    global_ipv6_address: String,
    #[serde(rename = "GlobalIPv6PrefixLen")]
    global_ipv6_prefix_len: i32,
    #[serde(rename = "IPAddress")]
    ip_address: String,
    #[serde(rename = "IPPrefixLen")]
    ip_prefix_len: i32,
    #[serde(rename = "IPv6Gateway")]
    ipv6_gateway: String,
    mac_address: String,
    networks: std::collections::HashMap<String, EndpointSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ContainerState {
    status: String,
    running: bool,
    paused: bool,
    restarting: bool,
    #[serde(rename = "OOMKilled")]
    oom_killed: bool,
    dead: bool,
    pid: i64,
    exit_code: i32,
    error: String,
    started_at: String,
    finished_at: String,
}

/// Exec create request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ExecCreateRequest {
    pub attach_stdin: Option<bool>,
    pub attach_stdout: Option<bool>,
    pub attach_stderr: Option<bool>,
    pub detach_keys: Option<String>,
    pub tty: Option<bool>,
    pub env: Option<Vec<String>>,
    pub cmd: Option<Vec<String>>,
    pub privileged: Option<bool>,
    pub user: Option<String>,
    pub working_dir: Option<String>,
}

/// Exec start request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ExecStartRequest {
    pub detach: Option<bool>,
    pub tty: Option<bool>,
    pub console_size: Option<Vec<u32>>,
}

/// Exec inspect response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ExecInspectResponse {
    pub can_remove: bool,
    #[serde(rename = "ContainerID")]
    pub container_id: String,
    pub detach_keys: String,
    pub exit_code: Option<i32>,
    #[serde(rename = "ID")]
    pub id: String,
    pub open_stderr: bool,
    pub open_stdin: bool,
    pub open_stdout: bool,
    pub process_config: ExecProcessConfig,
    pub running: bool,
    pub pid: i64,
}

/// Exec process config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecProcessConfig {
    pub arguments: Vec<String>,
    pub entrypoint: String,
    pub privileged: bool,
    pub tty: bool,
    pub user: String,
}

/// Container attach options
#[derive(Debug, Clone)]
pub struct AttachOptions {
    pub stream: bool,
    pub stdin: bool,
    pub stdout: bool,
    pub stderr: bool,
    pub logs: bool,
    pub detach_keys: Option<String>,
}

/// Exec instance stored in memory
#[derive(Debug, Clone)]
pub struct ExecInstance {
    pub id: String,
    pub container_id: String,
    pub cmd: Vec<String>,
    pub env: Vec<String>,
    pub tty: bool,
    pub attach_stdin: bool,
    pub attach_stdout: bool,
    pub attach_stderr: bool,
    pub privileged: bool,
    pub user: String,
    pub working_dir: String,
    pub running: bool,
    pub exit_code: Option<i32>,
    pub pid: Option<i64>,
}

/// API Handler for processing requests
#[derive(Clone)]
pub struct ApiHandler {
    container_manager: Arc<ContainerManager>,
    exec_instances: Arc<std::sync::RwLock<std::collections::HashMap<String, ExecInstance>>>,
}

impl ApiHandler {
    /// Create a new API handler
    pub fn new(container_manager: Arc<ContainerManager>) -> Self {
        Self { 
            container_manager,
            exec_instances: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Handle an incoming API request
    /// Supports Docker Engine API v1.24+ for Portainer compatibility
    pub fn handle_request(&self, method: &str, path: &str, body: &str) -> Result<String> {
        debug!("API request: {} {} body={}", method, path, body.len());

        // Strip version prefix and query string for matching
        let path_clean = path.split('?').next().unwrap_or(path);
        let path_parts: Vec<&str> = path_clean.trim_start_matches('/').split('/').collect();
        
        // Skip version prefix if present (e.g., v1.24, v1.43)
        let parts = if !path_parts.is_empty() && path_parts[0].starts_with("v1.") {
            &path_parts[1..]
        } else {
            &path_parts[..]
        };

        match (method, parts) {
            // Version and info - required for Portainer
            ("GET", ["version"]) => self.get_version(),
            ("GET", ["info"]) => self.get_info(),
            ("GET", ["_ping"]) => Ok("OK".to_string()),
            ("HEAD", ["_ping"]) => Ok("".to_string()),

            // Events - required for Portainer
            ("GET", ["events"]) => self.get_events(path),

            // Containers
            ("GET", ["containers", "json"]) => self.list_containers(path),
            ("POST", ["containers", "create"]) => self.create_container(body, path),
            ("GET", ["containers", id, "json"]) => self.inspect_container(id),
            ("GET", ["containers", id, "top"]) => self.container_top(id, path),
            ("GET", ["containers", id, "stats"]) => self.container_stats(id, path),
            ("POST", ["containers", id, "start"]) => self.start_container(id),
            ("POST", ["containers", id, "stop"]) => self.stop_container(id),
            ("POST", ["containers", id, "restart"]) => self.restart_container(id),
            ("POST", ["containers", id, "kill"]) => self.kill_container(id, path),
            ("POST", ["containers", id, "pause"]) => self.pause_container(id),
            ("POST", ["containers", id, "unpause"]) => self.unpause_container(id),
            ("POST", ["containers", id, "rename"]) => self.rename_container(id, path),
            ("POST", ["containers", id, "update"]) => self.update_container(id, body),
            ("DELETE", ["containers", id]) => self.remove_container(id, path),
            ("GET", ["containers", id, "logs"]) => self.container_logs(id, path),
            ("POST", ["containers", id, "wait"]) => self.wait_container(id),
            ("POST", ["containers", "prune"]) => self.prune_containers(path),
            // Attach and console endpoints
            ("POST", ["containers", id, "attach"]) => self.attach_container(id, path),
            ("GET", ["containers", id, "attach", "ws"]) => self.attach_container_websocket(id, path),
            ("POST", ["containers", id, "resize"]) => self.resize_container_tty(id, path),

            // Images - required for Portainer
            ("GET", ["images", "json"]) => self.list_images(path),
            ("GET", ["images", id, "json"]) => self.inspect_image(id),
            ("GET", ["images", id, "history"]) => self.image_history(id),
            ("POST", ["images", "create"]) => self.pull_image(path, body),
            ("POST", ["images", id, "tag"]) => self.tag_image(id, path),
            ("DELETE", ["images", id]) => self.remove_image(id, path),
            ("POST", ["images", "prune"]) => self.prune_images(path),
            ("GET", ["images", "search"]) => self.search_images(path),
            ("POST", ["build"]) => self.build_image(path, body),

            // Networks - required for Portainer
            ("GET", ["networks"]) => self.list_networks(),
            ("GET", ["networks", id]) => self.inspect_network(id),
            ("POST", ["networks", "create"]) => self.create_network(body),
            ("DELETE", ["networks", id]) => self.remove_network(id),
            ("POST", ["networks", id, "connect"]) => self.connect_network(id, body),
            ("POST", ["networks", id, "disconnect"]) => self.disconnect_network(id, body),
            ("POST", ["networks", "prune"]) => self.prune_networks(path),

            // Volumes - required for Portainer
            ("GET", ["volumes"]) => self.list_volumes(path),
            ("GET", ["volumes", name]) => self.inspect_volume(name),
            ("POST", ["volumes", "create"]) => self.create_volume(body),
            ("DELETE", ["volumes", name]) => self.remove_volume(name, path),
            ("POST", ["volumes", "prune"]) => self.prune_volumes(path),

            // System - required for Portainer
            ("GET", ["system", "df"]) => self.system_df(),
            ("POST", ["auth"]) => self.auth(body),

            // Exec - required for Portainer terminal
            ("POST", ["containers", id, "exec"]) => self.create_exec(id, body),
            ("POST", ["exec", id, "start"]) => self.start_exec(id, body),
            ("GET", ["exec", id, "json"]) => self.inspect_exec(id),
            ("POST", ["exec", id, "resize"]) => self.resize_exec(id, path),

            // Swarm - optional for Portainer
            ("GET", ["swarm"]) => self.inspect_swarm(),
            ("POST", ["swarm", "init"]) => self.init_swarm(body),
            ("POST", ["swarm", "join"]) => self.join_swarm(body),
            ("POST", ["swarm", "leave"]) => self.leave_swarm(path),
            ("POST", ["swarm", "update"]) => self.update_swarm(path, body),
            ("GET", ["swarm", "unlockkey"]) => self.get_unlock_key(),

            // Nodes
            ("GET", ["nodes"]) => self.list_nodes(path),
            ("GET", ["nodes", id]) => self.inspect_node(id),
            ("DELETE", ["nodes", id]) => self.remove_node(id, path),
            ("POST", ["nodes", id, "update"]) => self.update_node(id, path, body),

            // Services
            ("GET", ["services"]) => self.list_services(path),
            ("GET", ["services", id]) => self.inspect_service(id),
            ("POST", ["services", "create"]) => self.create_service(body),
            ("DELETE", ["services", id]) => self.remove_service(id),
            ("POST", ["services", id, "update"]) => self.update_service(id, path, body),
            ("GET", ["services", id, "logs"]) => self.service_logs(id, path),

            // Tasks
            ("GET", ["tasks"]) => self.list_tasks(path),
            ("GET", ["tasks", id]) => self.inspect_task(id),

            // Secrets
            ("GET", ["secrets"]) => self.list_secrets(path),
            ("GET", ["secrets", id]) => self.inspect_secret(id),
            ("POST", ["secrets", "create"]) => self.create_secret(body),
            ("DELETE", ["secrets", id]) => self.remove_secret(id),
            ("POST", ["secrets", id, "update"]) => self.update_secret(id, path, body),

            // Configs
            ("GET", ["configs"]) => self.list_configs(path),
            ("GET", ["configs", id]) => self.inspect_config(id),
            ("POST", ["configs", "create"]) => self.create_config(body),
            ("DELETE", ["configs", id]) => self.remove_config(id),
            ("POST", ["configs", id, "update"]) => self.update_config(id, path, body),

            // Plugins
            ("GET", ["plugins"]) => self.list_plugins(path),
            ("GET", ["plugins", name, "json"]) => self.inspect_plugin(name),

            // Distribution
            ("GET", ["distribution", image, "json"]) => self.get_distribution_info(image),

            // Default
            _ => Err(RuneError::Api(format!("Unknown endpoint: {} {}", method, path))),
        }
    }

    fn get_version(&self) -> Result<String> {
        let response = VersionResponse {
            version: env!("CARGO_PKG_VERSION").to_string(),
            api_version: "1.43".to_string(),
            min_api_version: "1.24".to_string(),
            go_version: "N/A (Rust)".to_string(),
            git_commit: "rune".to_string(),
            built: chrono::Utc::now().to_rfc3339(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            kernel_version: get_kernel_version(),
            experimental: false,
            build_time: chrono::Utc::now().to_rfc3339(),
        };
        Ok(serde_json::to_string(&response)?)
    }

    fn get_info(&self) -> Result<String> {
        let containers = self.container_manager.count()
            .map_err(|e| tracing::warn!("Failed to count containers: {}", e))
            .unwrap_or(0) as i64;
        let running = self.container_manager.running_count()
            .map_err(|e| tracing::warn!("Failed to count running containers: {}", e))
            .unwrap_or(0) as i64;

        let mut runtimes = std::collections::HashMap::new();
        runtimes.insert("rune".to_string(), RuntimeInfo {
            path: "/usr/bin/rune".to_string(),
            runtime_args: None,
        });

        let response = InfoResponse {
            id: uuid::Uuid::new_v4().to_string(),
            containers,
            containers_running: running,
            containers_paused: 0,
            containers_stopped: containers - running,
            images: 0,
            driver: "overlay2".to_string(),
            driver_status: vec![
                vec!["Backing Filesystem".to_string(), "extfs".to_string()],
                vec!["Supports d_type".to_string(), "true".to_string()],
            ],
            ncpu: num_cpus::get(),
            mem_total: get_total_memory(),
            docker_root_dir: "/var/lib/rune".to_string(),
            name: gethostname::gethostname().to_string_lossy().to_string(),
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            default_runtime: "rune".to_string(),
            os_type: "linux".to_string(),
            operating_system: get_os_name(),
            architecture: std::env::consts::ARCH.to_string(),
            kernel_version: get_kernel_version(),
            experimental_build: false,
            live_restore_enabled: false,
            swarm: SwarmInfo {
                local_node_state: "inactive".to_string(),
                ..Default::default()
            },
            runtimes,
            security_options: vec![
                "name=seccomp,profile=default".to_string(),
            ],
            plugins: PluginsInfo {
                volume: vec!["local".to_string()],
                network: vec!["bridge".to_string(), "host".to_string(), "overlay".to_string(), "null".to_string()],
                authorization: None,
                log: vec!["json-file".to_string(), "local".to_string()],
            },
            registries: vec![],
        };
        Ok(serde_json::to_string(&response)?)
    }

    fn get_events(&self, _path: &str) -> Result<String> {
        // Return empty events stream for now
        Ok("".to_string())
    }

    fn list_containers(&self, path: &str) -> Result<String> {
        let all = path.contains("all=true") || path.contains("all=1");
        let containers = self.container_manager.list(all)?;
        
        // Parse label filter if present
        let label_filter: Option<Vec<(String, Option<String>)>> = if let Some(pos) = path.find("filters=") {
            let start = pos + 8;
            let end = path[start..].find('&').map(|i| start + i).unwrap_or(path.len());
            let filter_str = &path[start..end];
            // URL decode and parse JSON filter
            if let Ok(decoded) = urlencoding_decode(filter_str) {
                if let Ok(filters) = serde_json::from_str::<Value>(&decoded) {
                    if let Some(labels) = filters.get("label").and_then(|v| v.as_array()) {
                        Some(labels.iter().filter_map(|l| {
                            l.as_str().map(|s| {
                                if let Some((k, v)) = s.split_once('=') {
                                    (k.to_string(), Some(v.to_string()))
                                } else {
                                    (s.to_string(), None)
                                }
                            })
                        }).collect())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        let response: Vec<ContainerListItem> = containers
            .iter()
            .filter(|c| {
                // Apply label filter if present
                if let Some(ref filters) = label_filter {
                    for (key, value) in filters {
                        if let Some(container_value) = c.labels.get(key) {
                            if let Some(expected_value) = value {
                                if container_value != expected_value {
                                    return false;
                                }
                            }
                        } else {
                            return false;
                        }
                    }
                }
                true
            })
            .map(|c| {
                // Convert ports to PortInfo
                let ports: Vec<PortInfo> = c.exposed_ports.iter().map(|p| {
                    PortInfo {
                        ip: Some("0.0.0.0".to_string()),
                        private_port: p.container_port,
                        public_port: Some(p.host_port),
                        port_type: match p.protocol {
                            crate::container::Protocol::Tcp => "tcp".to_string(),
                            crate::container::Protocol::Udp => "udp".to_string(),
                        },
                    }
                }).collect();

                // Convert volumes to mounts
                let mounts: Vec<MountPoint> = c.volumes.iter().map(|v| {
                    MountPoint {
                        mount_type: "bind".to_string(),
                        name: None,
                        source: v.host_path.clone(),
                        destination: v.container_path.clone(),
                        driver: None,
                        mode: if v.read_only { "ro" } else { "rw" }.to_string(),
                        rw: !v.read_only,
                        propagation: "rprivate".to_string(),
                    }
                }).collect();

                // Build network settings
                let mut networks = std::collections::HashMap::new();
                networks.insert(c.network_mode.clone(), EndpointSettings {
                    network_id: c.network_mode.clone(),
                    endpoint_id: format!("{}-ep", c.id),
                    gateway: "172.17.0.1".to_string(),
                    ip_address: "172.17.0.2".to_string(),
                    ip_prefix_len: 16,
                    ..Default::default()
                });

                ContainerListItem {
                    id: c.id.clone(),
                    names: vec![format!("/{}", c.name)],
                    image: c.image.clone(),
                    image_id: format!("sha256:{}", c.image.replace(":", "")),
                    command: c.cmd.join(" "),
                    created: c.created_at.timestamp(),
                    state: c.status.to_string().to_lowercase(),
                    status: format_container_status(&c.status, c.started_at, c.finished_at),
                    ports,
                    labels: c.labels.clone(),
                    network_settings: NetworkSettingsSummary { networks },
                    mounts,
                }
            })
            .collect();

        Ok(serde_json::to_string(&response)?)
    }

    fn create_container(&self, body: &str, path: &str) -> Result<String> {
        let name = if let Some(pos) = path.find("name=") {
            let start = pos + 5;
            let end = path[start..].find('&').map(|i| start + i).unwrap_or(path.len());
            path[start..end].to_string()
        } else {
            format!("rune-{}", &uuid::Uuid::new_v4().to_string()[..8])
        };

        let request: ContainerCreateRequest = serde_json::from_str(body)?;
        let mut config = ContainerConfig::new(&name, &request.image);

        // Set command
        if let Some(cmd) = request.cmd {
            config.cmd = cmd;
        }
        
        // Set environment variables
        if let Some(env) = request.env {
            for e in env {
                if let Some((key, value)) = e.split_once('=') {
                    config.env.insert(key.to_string(), value.to_string());
                }
            }
        }
        
        // Set labels
        if let Some(labels) = request.labels {
            config.labels = labels;
        }
        
        // Set working directory
        if let Some(wd) = request.working_dir {
            config.working_dir = wd;
        }
        
        // Set hostname
        if let Some(hostname) = request.hostname {
            config.hostname = hostname;
        }
        
        // Set user
        if let Some(user) = request.user {
            config.user = user;
        }

        // Handle host config options
        if let Some(host_config) = request.host_config {
            // Set network mode
            if let Some(network_mode) = host_config.network_mode {
                config.network_mode = network_mode;
            }
            
            // Set privileged mode
            if let Some(privileged) = host_config.privileged {
                config.privileged = privileged;
            }
            
            // Set memory limit
            if let Some(memory) = host_config.memory {
                config.resources.memory_limit = Some(memory as u64);
            }
            
            // Set CPU shares
            if let Some(cpu_shares) = host_config.cpu_shares {
                config.resources.cpu_shares = Some(cpu_shares as u64);
            }
            
            // Set CPU period/quota
            if let Some(cpu_period) = host_config.cpu_period {
                config.resources.cpu_period = Some(cpu_period as u64);
            }
            if let Some(cpu_quota) = host_config.cpu_quota {
                config.resources.cpu_quota = Some(cpu_quota);
            }
            
            // Handle volume binds
            if let Some(binds) = host_config.binds {
                for bind in binds {
                    let parts: Vec<&str> = bind.split(':').collect();
                    if parts.len() >= 2 {
                        config.volumes.push(crate::container::VolumeMount {
                            host_path: parts[0].to_string(),
                            container_path: parts[1].to_string(),
                            read_only: parts.get(2).map(|m| *m == "ro").unwrap_or(false),
                        });
                    }
                }
            }
            
            // Handle port bindings
            if let Some(port_bindings) = host_config.port_bindings {
                for (container_port_str, bindings) in port_bindings {
                    // Parse container port (e.g., "80/tcp")
                    let (port, protocol) = parse_port_spec(&container_port_str);
                    for binding in bindings {
                        if let Some(host_port_str) = binding.host_port {
                            if let Ok(host_port) = host_port_str.parse::<u16>() {
                                config.exposed_ports.push(crate::container::PortMapping {
                                    host_port,
                                    container_port: port,
                                    protocol,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Handle exposed ports from config
        if let Some(exposed_ports) = request.exposed_ports {
            for (port_spec, _) in exposed_ports {
                let (port, protocol) = parse_port_spec(&port_spec);
                // Only add if not already bound
                if !config.exposed_ports.iter().any(|p| p.container_port == port) {
                    config.exposed_ports.push(crate::container::PortMapping {
                        host_port: port, // Default to same port
                        container_port: port,
                        protocol,
                    });
                }
            }
        }

        let id = self.container_manager.create(config)?;
        let response = ContainerCreateResponse { id, warnings: vec![] };
        Ok(serde_json::to_string(&response)?)
    }

    fn inspect_container(&self, id: &str) -> Result<String> {
        let container = self.container_manager.get(id)?;

        // Convert environment variables to Docker format
        let env: Vec<String> = container.env.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        // Convert volumes to exposed volumes
        let volumes: Option<std::collections::HashMap<String, Value>> = if container.volumes.is_empty() {
            None
        } else {
            let mut vol_map = std::collections::HashMap::new();
            for v in &container.volumes {
                vol_map.insert(v.container_path.clone(), json!({}));
            }
            Some(vol_map)
        };

        // Build mounts list
        let mounts: Vec<MountPoint> = container.volumes.iter().map(|v| {
            MountPoint {
                mount_type: "bind".to_string(),
                name: None,
                source: v.host_path.clone(),
                destination: v.container_path.clone(),
                driver: None,
                mode: if v.read_only { "ro" } else { "rw" }.to_string(),
                rw: !v.read_only,
                propagation: "rprivate".to_string(),
            }
        }).collect();

        // Build exposed ports map
        let exposed_ports: Option<std::collections::HashMap<String, Value>> = if container.exposed_ports.is_empty() {
            None
        } else {
            let mut ports = std::collections::HashMap::new();
            for p in &container.exposed_ports {
                let protocol = match p.protocol {
                    crate::container::Protocol::Tcp => "tcp",
                    crate::container::Protocol::Udp => "udp",
                };
                ports.insert(format!("{}/{}", p.container_port, protocol), json!({}));
            }
            Some(ports)
        };

        // Build port bindings for host config
        let port_bindings: Option<std::collections::HashMap<String, Vec<PortBinding>>> = if container.exposed_ports.is_empty() {
            None
        } else {
            let mut bindings = std::collections::HashMap::new();
            for p in &container.exposed_ports {
                let protocol = match p.protocol {
                    crate::container::Protocol::Tcp => "tcp",
                    crate::container::Protocol::Udp => "udp",
                };
                bindings.insert(
                    format!("{}/{}", p.container_port, protocol),
                    vec![PortBinding {
                        host_ip: Some("0.0.0.0".to_string()),
                        host_port: Some(p.host_port.to_string()),
                    }]
                );
            }
            Some(bindings)
        };

        // Build network ports map
        let ports: Option<std::collections::HashMap<String, Option<Vec<PortBinding>>>> = if container.exposed_ports.is_empty() {
            None
        } else {
            let mut ports_map = std::collections::HashMap::new();
            for p in &container.exposed_ports {
                let protocol = match p.protocol {
                    crate::container::Protocol::Tcp => "tcp",
                    crate::container::Protocol::Udp => "udp",
                };
                ports_map.insert(
                    format!("{}/{}", p.container_port, protocol),
                    Some(vec![PortBinding {
                        host_ip: Some("0.0.0.0".to_string()),
                        host_port: Some(p.host_port.to_string()),
                    }])
                );
            }
            Some(ports_map)
        };

        // Build volume binds for host config
        let binds: Option<Vec<String>> = if container.volumes.is_empty() {
            None
        } else {
            Some(container.volumes.iter().map(|v| {
                if v.read_only {
                    format!("{}:{}:ro", v.host_path, v.container_path)
                } else {
                    format!("{}:{}", v.host_path, v.container_path)
                }
            }).collect())
        };

        // Build network settings
        let mut networks = std::collections::HashMap::new();
        networks.insert(container.network_mode.clone(), EndpointSettings {
            network_id: container.network_mode.clone(),
            endpoint_id: format!("{}-ep", container.id),
            gateway: "172.17.0.1".to_string(),
            ip_address: "172.17.0.2".to_string(),
            ip_prefix_len: 16,
            ..Default::default()
        });

        let response = ContainerInspect {
            id: container.id.clone(),
            created: container.created_at.to_rfc3339(),
            path: container.entrypoint.first().cloned().unwrap_or_default(),
            args: if container.entrypoint.len() > 1 {
                container.entrypoint[1..].to_vec()
            } else {
                container.cmd.clone()
            },
            state: ContainerState {
                status: container.status.to_string().to_lowercase(),
                running: matches!(container.status, crate::container::ContainerStatus::Running),
                paused: matches!(container.status, crate::container::ContainerStatus::Paused),
                restarting: false,
                oom_killed: false,
                dead: matches!(container.status, crate::container::ContainerStatus::Dead),
                pid: container.pid.unwrap_or(0) as i64,
                exit_code: container.exit_code.unwrap_or(0),
                error: "".to_string(),
                started_at: container.started_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
                finished_at: container.finished_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
            },
            image: container.image.clone(),
            name: format!("/{}", container.name),
            restart_count: 0,
            driver: "overlay2".to_string(),
            platform: "linux".to_string(),
            config: ContainerConfigResponse {
                hostname: container.hostname.clone(),
                domainname: container.domainname.clone(),
                user: container.user.clone(),
                attach_stdin: false,
                attach_stdout: true,
                attach_stderr: true,
                exposed_ports,
                tty: false,
                open_stdin: false,
                stdin_once: false,
                env,
                cmd: container.cmd.clone(),
                image: container.image.clone(),
                volumes,
                working_dir: container.working_dir.clone(),
                entrypoint: if container.entrypoint.is_empty() {
                    None
                } else {
                    Some(container.entrypoint.clone())
                },
                labels: container.labels.clone(),
            },
            host_config: HostConfigResponse {
                binds,
                network_mode: container.network_mode.clone(),
                port_bindings,
                restart_policy: RestartPolicyResponse::default(),
                auto_remove: false,
                privileged: container.privileged,
                publish_all_ports: false,
                read_only_rootfs: container.read_only_rootfs,
                memory: container.resources.memory_limit.unwrap_or(0) as i64,
                memory_swap: 0,
                memory_reservation: container.resources.memory_reservation.unwrap_or(0) as i64,
                cpu_shares: container.resources.cpu_shares.unwrap_or(0) as i64,
                cpu_period: container.resources.cpu_period.unwrap_or(0) as i64,
                cpu_quota: container.resources.cpu_quota.unwrap_or(0),
                cpuset_cpus: "".to_string(),
                cpuset_mems: "".to_string(),
                pids_limit: container.resources.pids_limit,
            },
            network_settings: NetworkSettingsResponse {
                bridge: "".to_string(),
                sandbox_id: format!("{}-sandbox", container.id),
                hairpin_mode: false,
                link_local_ipv6_address: "".to_string(),
                link_local_ipv6_prefix_len: 0,
                ports,
                sandbox_key: format!("/var/run/rune/netns/{}", container.id),
                secondary_ip_addresses: None,
                secondary_ipv6_addresses: None,
                endpoint_id: format!("{}-ep", container.id),
                gateway: "172.17.0.1".to_string(),
                global_ipv6_address: "".to_string(),
                global_ipv6_prefix_len: 0,
                ip_address: "172.17.0.2".to_string(),
                ip_prefix_len: 16,
                ipv6_gateway: "".to_string(),
                mac_address: "02:42:ac:11:00:02".to_string(),
                networks,
            },
            mounts,
        };

        Ok(serde_json::to_string(&response)?)
    }

    fn start_container(&self, id: &str) -> Result<String> {
        self.container_manager.start(id)?;
        Ok("".to_string())
    }

    fn stop_container(&self, id: &str) -> Result<String> {
        self.container_manager.stop(id)?;
        Ok("".to_string())
    }

    fn restart_container(&self, id: &str) -> Result<String> {
        let _ = self.container_manager.stop(id);
        self.container_manager.start(id)?;
        Ok("".to_string())
    }

    fn remove_container(&self, id: &str, path: &str) -> Result<String> {
        let force = path.contains("force=true") || path.contains("force=1");
        self.container_manager.remove(id, force)?;
        Ok("".to_string())
    }

    fn list_networks(&self) -> Result<String> {
        let response = json!([
            {
                "Name": "bridge",
                "Id": "bridge",
                "Created": chrono::Utc::now().to_rfc3339(),
                "Scope": "local",
                "Driver": "bridge",
                "EnableIPv6": false,
                "IPAM": {
                    "Driver": "default",
                    "Config": [{"Subnet": "172.17.0.0/16", "Gateway": "172.17.0.1"}]
                },
                "Internal": false,
                "Attachable": false,
                "Ingress": false,
                "Options": {},
                "Labels": {}
            },
            {
                "Name": "host",
                "Id": "host",
                "Created": chrono::Utc::now().to_rfc3339(),
                "Scope": "local",
                "Driver": "host",
                "EnableIPv6": false,
                "IPAM": {"Driver": "default", "Config": []},
                "Internal": false,
                "Attachable": false,
                "Ingress": false,
                "Options": {},
                "Labels": {}
            },
            {
                "Name": "none",
                "Id": "none",
                "Created": chrono::Utc::now().to_rfc3339(),
                "Scope": "local",
                "Driver": "null",
                "EnableIPv6": false,
                "IPAM": {"Driver": "default", "Config": []},
                "Internal": false,
                "Attachable": false,
                "Ingress": false,
                "Options": {},
                "Labels": {}
            }
        ]);
        Ok(response.to_string())
    }

    // Additional container methods for Portainer compatibility
    fn container_top(&self, _id: &str, _path: &str) -> Result<String> {
        Ok(json!({
            "Titles": ["UID", "PID", "PPID", "C", "STIME", "TTY", "TIME", "CMD"],
            "Processes": []
        }).to_string())
    }

    fn container_stats(&self, _id: &str, _path: &str) -> Result<String> {
        Ok(json!({
            "read": chrono::Utc::now().to_rfc3339(),
            "preread": chrono::Utc::now().to_rfc3339(),
            "pids_stats": {"current": 0},
            "blkio_stats": {},
            "num_procs": 0,
            "storage_stats": {},
            "cpu_stats": {
                "cpu_usage": {"total_usage": 0, "percpu_usage": [], "usage_in_kernelmode": 0, "usage_in_usermode": 0},
                "system_cpu_usage": 0,
                "online_cpus": num_cpus::get(),
                "throttling_data": {"periods": 0, "throttled_periods": 0, "throttled_time": 0}
            },
            "precpu_stats": {},
            "memory_stats": {"usage": 0, "max_usage": 0, "stats": {}, "limit": 0},
            "name": "",
            "id": ""
        }).to_string())
    }

    fn kill_container(&self, id: &str, _path: &str) -> Result<String> {
        self.container_manager.stop(id)?;
        Ok("".to_string())
    }

    fn pause_container(&self, _id: &str) -> Result<String> {
        // Pause not fully implemented yet
        Ok("".to_string())
    }

    fn unpause_container(&self, _id: &str) -> Result<String> {
        // Unpause not fully implemented yet
        Ok("".to_string())
    }

    fn rename_container(&self, _id: &str, _path: &str) -> Result<String> {
        // Rename not fully implemented yet
        Ok("".to_string())
    }

    fn update_container(&self, _id: &str, _body: &str) -> Result<String> {
        Ok(json!({"Warnings": []}).to_string())
    }

    fn container_logs(&self, _id: &str, _path: &str) -> Result<String> {
        Ok("".to_string())
    }

    fn wait_container(&self, _id: &str) -> Result<String> {
        Ok(json!({"StatusCode": 0}).to_string())
    }

    fn prune_containers(&self, _path: &str) -> Result<String> {
        Ok(json!({"ContainersDeleted": [], "SpaceReclaimed": 0}).to_string())
    }

    // Image methods for Portainer compatibility
    fn list_images(&self, _path: &str) -> Result<String> {
        Ok("[]".to_string())
    }

    fn inspect_image(&self, id: &str) -> Result<String> {
        Ok(json!({
            "Id": id,
            "RepoTags": [],
            "RepoDigests": [],
            "Parent": "",
            "Comment": "",
            "Created": chrono::Utc::now().to_rfc3339(),
            "DockerVersion": env!("CARGO_PKG_VERSION"),
            "Author": "",
            "Config": {},
            "Architecture": std::env::consts::ARCH,
            "Os": "linux",
            "Size": 0,
            "VirtualSize": 0,
            "GraphDriver": {"Name": "overlay2", "Data": {}},
            "RootFS": {"Type": "layers", "Layers": []},
            "Metadata": {"LastTagTime": chrono::Utc::now().to_rfc3339()}
        }).to_string())
    }

    fn image_history(&self, _id: &str) -> Result<String> {
        Ok("[]".to_string())
    }

    fn pull_image(&self, _path: &str, _body: &str) -> Result<String> {
        Ok("".to_string())
    }

    fn tag_image(&self, _id: &str, _path: &str) -> Result<String> {
        Ok("".to_string())
    }

    fn remove_image(&self, _id: &str, _path: &str) -> Result<String> {
        Ok("[]".to_string())
    }

    fn prune_images(&self, _path: &str) -> Result<String> {
        Ok(json!({"ImagesDeleted": [], "SpaceReclaimed": 0}).to_string())
    }

    fn search_images(&self, _path: &str) -> Result<String> {
        Ok("[]".to_string())
    }

    fn build_image(&self, _path: &str, _body: &str) -> Result<String> {
        Ok("".to_string())
    }

    // Network methods
    fn inspect_network(&self, id: &str) -> Result<String> {
        let driver = match id {
            "bridge" => "bridge",
            "host" => "host",
            "none" => "null",
            _ => "bridge",
        };
        Ok(json!({
            "Name": id,
            "Id": id,
            "Created": chrono::Utc::now().to_rfc3339(),
            "Scope": "local",
            "Driver": driver,
            "EnableIPv6": false,
            "IPAM": {"Driver": "default", "Config": []},
            "Internal": false,
            "Attachable": false,
            "Ingress": false,
            "Containers": {},
            "Options": {},
            "Labels": {}
        }).to_string())
    }

    fn create_network(&self, body: &str) -> Result<String> {
        let request: Value = serde_json::from_str(body).unwrap_or(json!({}));
        let _name = request.get("Name").and_then(|v| v.as_str()).unwrap_or("network");
        let id = format!("{:064x}", rand::random::<u64>());
        Ok(json!({"Id": id, "Warning": ""}).to_string())
    }

    fn remove_network(&self, _id: &str) -> Result<String> {
        Ok("".to_string())
    }

    fn connect_network(&self, _id: &str, _body: &str) -> Result<String> {
        Ok("".to_string())
    }

    fn disconnect_network(&self, _id: &str, _body: &str) -> Result<String> {
        Ok("".to_string())
    }

    fn prune_networks(&self, _path: &str) -> Result<String> {
        Ok(json!({"NetworksDeleted": []}).to_string())
    }

    // Volume methods
    fn list_volumes(&self, _path: &str) -> Result<String> {
        Ok(json!({"Volumes": [], "Warnings": []}).to_string())
    }

    fn inspect_volume(&self, name: &str) -> Result<String> {
        Ok(json!({
            "Name": name,
            "Driver": "local",
            "Mountpoint": format!("/var/lib/rune/volumes/{}", name),
            "CreatedAt": chrono::Utc::now().to_rfc3339(),
            "Status": {},
            "Labels": {},
            "Scope": "local",
            "Options": {}
        }).to_string())
    }

    fn create_volume(&self, body: &str) -> Result<String> {
        let request: Value = serde_json::from_str(body).unwrap_or(json!({}));
        let default_name = uuid::Uuid::new_v4().to_string();
        let name = request.get("Name").and_then(|v| v.as_str())
            .unwrap_or(&default_name[..12]);
        Ok(json!({
            "Name": name,
            "Driver": "local",
            "Mountpoint": format!("/var/lib/rune/volumes/{}", name),
            "CreatedAt": chrono::Utc::now().to_rfc3339(),
            "Status": {},
            "Labels": {},
            "Scope": "local",
            "Options": {}
        }).to_string())
    }

    fn remove_volume(&self, _name: &str, _path: &str) -> Result<String> {
        Ok("".to_string())
    }

    fn prune_volumes(&self, _path: &str) -> Result<String> {
        Ok(json!({"VolumesDeleted": [], "SpaceReclaimed": 0}).to_string())
    }

    // System methods
    fn system_df(&self) -> Result<String> {
        Ok(json!({
            "LayersSize": 0,
            "Images": [],
            "Containers": [],
            "Volumes": [],
            "BuildCache": []
        }).to_string())
    }

    fn auth(&self, _body: &str) -> Result<String> {
        Ok(json!({"Status": "Login Succeeded", "IdentityToken": ""}).to_string())
    }

    // Exec methods for Portainer terminal
    fn create_exec(&self, container_id: &str, body: &str) -> Result<String> {
        // Verify container exists
        let _container = self.container_manager.get(container_id)?;
        
        let request: ExecCreateRequest = serde_json::from_str(body).unwrap_or(ExecCreateRequest {
            attach_stdin: Some(false),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            detach_keys: None,
            tty: Some(false),
            env: None,
            cmd: Some(vec!["/bin/sh".to_string()]),
            privileged: None,
            user: None,
            working_dir: None,
        });
        
        let exec_id = uuid::Uuid::new_v4().to_string();
        
        // Store exec instance
        let instance = ExecInstance {
            id: exec_id.clone(),
            container_id: container_id.to_string(),
            cmd: request.cmd.unwrap_or_else(|| vec!["/bin/sh".to_string()]),
            env: request.env.unwrap_or_default(),
            tty: request.tty.unwrap_or(false),
            attach_stdin: request.attach_stdin.unwrap_or(false),
            attach_stdout: request.attach_stdout.unwrap_or(true),
            attach_stderr: request.attach_stderr.unwrap_or(true),
            privileged: request.privileged.unwrap_or(false),
            user: request.user.unwrap_or_default(),
            working_dir: request.working_dir.unwrap_or_default(),
            running: false,
            exit_code: None,
            pid: None,
        };
        
        self.exec_instances.write().unwrap().insert(exec_id.clone(), instance);
        
        Ok(json!({"Id": exec_id}).to_string())
    }

    fn start_exec(&self, exec_id: &str, body: &str) -> Result<String> {
        let request: ExecStartRequest = serde_json::from_str(body).unwrap_or(ExecStartRequest {
            detach: Some(false),
            tty: Some(false),
            console_size: None,
        });
        
        // Get and update exec instance
        let instance = {
            let mut instances = self.exec_instances.write().unwrap();
            if let Some(instance) = instances.get_mut(exec_id) {
                instance.running = true;
                instance.pid = Some(std::process::id() as i64);
                instance.clone()
            } else {
                return Err(RuneError::Api(format!("No such exec instance: {}", exec_id)));
            }
        };
        
        // In a real implementation, this would:
        // 1. Attach to the container's namespace
        // 2. Execute the command
        // 3. Stream I/O if not detached
        
        if request.detach.unwrap_or(false) {
            // Detached mode - return immediately
            Ok("".to_string())
        } else {
            // Attached mode - in real implementation, stream data
            // For now, simulate command execution
            
            // Mark as completed
            {
                let mut instances = self.exec_instances.write().unwrap();
                if let Some(instance) = instances.get_mut(exec_id) {
                    instance.running = false;
                    instance.exit_code = Some(0);
                }
            }
            
            Ok("".to_string())
        }
    }

    fn inspect_exec(&self, exec_id: &str) -> Result<String> {
        let instances = self.exec_instances.read().unwrap();
        
        if let Some(instance) = instances.get(exec_id) {
            let response = ExecInspectResponse {
                can_remove: !instance.running,
                container_id: instance.container_id.clone(),
                detach_keys: "".to_string(),
                exit_code: instance.exit_code,
                id: instance.id.clone(),
                open_stderr: instance.attach_stderr,
                open_stdin: instance.attach_stdin,
                open_stdout: instance.attach_stdout,
                process_config: ExecProcessConfig {
                    arguments: if instance.cmd.len() > 1 {
                        instance.cmd[1..].to_vec()
                    } else {
                        vec![]
                    },
                    entrypoint: instance.cmd.first().cloned().unwrap_or_default(),
                    privileged: instance.privileged,
                    tty: instance.tty,
                    user: instance.user.clone(),
                },
                running: instance.running,
                pid: instance.pid.unwrap_or(0),
            };
            Ok(serde_json::to_string(&response)?)
        } else {
            // Return default response for unknown exec
            Ok(json!({
                "ID": exec_id,
                "Running": false,
                "ExitCode": 0,
                "ProcessConfig": {
                    "privileged": false,
                    "user": "",
                    "tty": false,
                    "entrypoint": "/bin/sh",
                    "arguments": ["-c"]
                },
                "OpenStdin": false,
                "OpenStderr": true,
                "OpenStdout": true,
                "CanRemove": true,
                "ContainerID": "",
                "DetachKeys": "",
                "Pid": 0
            }).to_string())
        }
    }

    fn resize_exec(&self, exec_id: &str, path: &str) -> Result<String> {
        // Parse height and width from query params
        let height = parse_query_param(path, "h").unwrap_or(24);
        let width = parse_query_param(path, "w").unwrap_or(80);
        
        // In real implementation, this would resize the PTY
        debug!("Resize exec {} to {}x{}", exec_id, width, height);
        
        Ok("".to_string())
    }

    // Container attach methods
    fn attach_container(&self, container_id: &str, path: &str) -> Result<String> {
        // Verify container exists and is running
        let container = self.container_manager.get(container_id)?;
        
        if !matches!(container.status, crate::container::ContainerStatus::Running) {
            return Err(RuneError::Api("Container is not running".to_string()));
        }
        
        // Parse attach options from query string
        let _options = AttachOptions {
            stream: path.contains("stream=true") || path.contains("stream=1"),
            stdin: path.contains("stdin=true") || path.contains("stdin=1"),
            stdout: path.contains("stdout=true") || path.contains("stdout=1"),
            stderr: path.contains("stderr=true") || path.contains("stderr=1"),
            logs: path.contains("logs=true") || path.contains("logs=1"),
            detach_keys: parse_query_string(path, "detachKeys"),
        };
        
        // In a real implementation, this would:
        // 1. Connect to the container's PTY/stdin/stdout/stderr
        // 2. Set up multiplexed streaming
        // 3. Handle detach keys
        
        // For HTTP, we return success and the actual streaming would happen over the connection
        Ok("".to_string())
    }

    fn attach_container_websocket(&self, container_id: &str, path: &str) -> Result<String> {
        // WebSocket attach endpoint for console access
        // Verify container exists
        let container = self.container_manager.get(container_id)?;
        
        if !matches!(container.status, crate::container::ContainerStatus::Running) {
            return Err(RuneError::Api("Container is not running".to_string()));
        }
        
        // Parse options
        let _stdin = path.contains("stdin=true") || path.contains("stdin=1");
        let _stdout = path.contains("stdout=true") || path.contains("stdout=1");
        let _stderr = path.contains("stderr=true") || path.contains("stderr=1");
        
        // In real implementation, this would upgrade to WebSocket
        // and provide bidirectional communication with the container
        
        Ok("".to_string())
    }

    fn resize_container_tty(&self, container_id: &str, path: &str) -> Result<String> {
        // Verify container exists
        let _container = self.container_manager.get(container_id)?;
        
        // Parse dimensions
        let height = parse_query_param(path, "h").unwrap_or(24);
        let width = parse_query_param(path, "w").unwrap_or(80);
        
        // In real implementation, resize the container's TTY
        debug!("Resize container {} TTY to {}x{}", container_id, width, height);
        
        Ok("".to_string())
    }

    // Swarm methods
    fn inspect_swarm(&self) -> Result<String> {
        Err(RuneError::Api("This node is not a swarm manager".to_string()))
    }

    fn init_swarm(&self, _body: &str) -> Result<String> {
        let node_id = uuid::Uuid::new_v4().to_string();
        Ok(format!("\"{}\"", node_id))
    }

    fn join_swarm(&self, _body: &str) -> Result<String> {
        Ok("".to_string())
    }

    fn leave_swarm(&self, _path: &str) -> Result<String> {
        Ok("".to_string())
    }

    fn update_swarm(&self, _path: &str, _body: &str) -> Result<String> {
        Ok("".to_string())
    }

    fn get_unlock_key(&self) -> Result<String> {
        Ok(json!({"UnlockKey": ""}).to_string())
    }

    // Node methods
    fn list_nodes(&self, _path: &str) -> Result<String> {
        Ok("[]".to_string())
    }

    fn inspect_node(&self, id: &str) -> Result<String> {
        Ok(json!({
            "ID": id,
            "Version": {"Index": 1},
            "CreatedAt": chrono::Utc::now().to_rfc3339(),
            "UpdatedAt": chrono::Utc::now().to_rfc3339(),
            "Spec": {"Role": "manager", "Availability": "active"},
            "Description": {"Hostname": gethostname::gethostname().to_string_lossy()},
            "Status": {"State": "ready", "Addr": "127.0.0.1"},
            "ManagerStatus": {"Leader": true, "Reachability": "reachable", "Addr": "127.0.0.1:2377"}
        }).to_string())
    }

    fn remove_node(&self, _id: &str, _path: &str) -> Result<String> {
        Ok("".to_string())
    }

    fn update_node(&self, _id: &str, _path: &str, _body: &str) -> Result<String> {
        Ok("".to_string())
    }

    // Service methods
    fn list_services(&self, _path: &str) -> Result<String> {
        Ok("[]".to_string())
    }

    fn inspect_service(&self, id: &str) -> Result<String> {
        Ok(json!({
            "ID": id,
            "Version": {"Index": 1},
            "CreatedAt": chrono::Utc::now().to_rfc3339(),
            "UpdatedAt": chrono::Utc::now().to_rfc3339(),
            "Spec": {"Name": id, "TaskTemplate": {}, "Mode": {"Replicated": {"Replicas": 1}}},
            "Endpoint": {"Spec": {}}
        }).to_string())
    }

    fn create_service(&self, _body: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        Ok(json!({"ID": id, "Warning": ""}).to_string())
    }

    fn remove_service(&self, _id: &str) -> Result<String> {
        Ok("".to_string())
    }

    fn update_service(&self, _id: &str, _path: &str, _body: &str) -> Result<String> {
        Ok(json!({"Warning": ""}).to_string())
    }

    fn service_logs(&self, _id: &str, _path: &str) -> Result<String> {
        Ok("".to_string())
    }

    // Task methods
    fn list_tasks(&self, _path: &str) -> Result<String> {
        Ok("[]".to_string())
    }

    fn inspect_task(&self, id: &str) -> Result<String> {
        Ok(json!({
            "ID": id,
            "Version": {"Index": 1},
            "CreatedAt": chrono::Utc::now().to_rfc3339(),
            "UpdatedAt": chrono::Utc::now().to_rfc3339(),
            "Spec": {},
            "ServiceID": "",
            "Slot": 1,
            "NodeID": "",
            "Status": {"Timestamp": chrono::Utc::now().to_rfc3339(), "State": "running"},
            "DesiredState": "running"
        }).to_string())
    }

    // Secret methods
    fn list_secrets(&self, _path: &str) -> Result<String> {
        Ok("[]".to_string())
    }

    fn inspect_secret(&self, id: &str) -> Result<String> {
        Ok(json!({
            "ID": id,
            "Version": {"Index": 1},
            "CreatedAt": chrono::Utc::now().to_rfc3339(),
            "UpdatedAt": chrono::Utc::now().to_rfc3339(),
            "Spec": {"Name": id}
        }).to_string())
    }

    fn create_secret(&self, _body: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        Ok(json!({"ID": id}).to_string())
    }

    fn remove_secret(&self, _id: &str) -> Result<String> {
        Ok("".to_string())
    }

    fn update_secret(&self, _id: &str, _path: &str, _body: &str) -> Result<String> {
        Ok("".to_string())
    }

    // Config methods
    fn list_configs(&self, _path: &str) -> Result<String> {
        Ok("[]".to_string())
    }

    fn inspect_config(&self, id: &str) -> Result<String> {
        Ok(json!({
            "ID": id,
            "Version": {"Index": 1},
            "CreatedAt": chrono::Utc::now().to_rfc3339(),
            "UpdatedAt": chrono::Utc::now().to_rfc3339(),
            "Spec": {"Name": id}
        }).to_string())
    }

    fn create_config(&self, _body: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        Ok(json!({"ID": id}).to_string())
    }

    fn remove_config(&self, _id: &str) -> Result<String> {
        Ok("".to_string())
    }

    fn update_config(&self, _id: &str, _path: &str, _body: &str) -> Result<String> {
        Ok("".to_string())
    }

    // Plugin methods
    fn list_plugins(&self, _path: &str) -> Result<String> {
        Ok("[]".to_string())
    }

    fn inspect_plugin(&self, name: &str) -> Result<String> {
        Ok(json!({
            "Id": name,
            "Name": name,
            "Enabled": true,
            "Settings": {"Mounts": [], "Env": [], "Args": [], "Devices": []},
            "PluginReference": "",
            "Config": {}
        }).to_string())
    }

    // Distribution methods
    fn get_distribution_info(&self, image: &str) -> Result<String> {
        Ok(json!({
            "Descriptor": {
                "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
                "digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                "size": 0
            },
            "Platforms": [{"architecture": std::env::consts::ARCH, "os": "linux"}]
        }).to_string())
    }
}

// Helper functions
fn get_kernel_version() -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/version")
            .ok()
            .and_then(|v| v.split_whitespace().nth(2).map(String::from))
            .unwrap_or_else(|| "unknown".to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        "unknown".to_string()
    }
}

fn get_total_memory() -> i64 {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|content| {
                content.lines()
                    .find(|line| line.starts_with("MemTotal:"))
                    .and_then(|line| {
                        line.split_whitespace()
                            .nth(1)
                            .and_then(|v| v.parse::<i64>().ok())
                            .map(|kb| kb * 1024) // Convert to bytes
                    })
            })
            .unwrap_or(0)
    }
    #[cfg(not(target_os = "linux"))]
    {
        0
    }
}

fn get_os_name() -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                content.lines()
                    .find(|line| line.starts_with("PRETTY_NAME="))
                    .map(|line| line.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
            })
            .unwrap_or_else(|| "Linux".to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        std::env::consts::OS.to_string()
    }
}

/// Simple URL decoding for filter parameters
fn urlencoding_decode(input: &str) -> std::result::Result<String, ()> {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    return Err(());
                }
            } else {
                return Err(());
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    
    Ok(result)
}

/// Parse port specification (e.g., "80/tcp" -> (80, Protocol::Tcp))
fn parse_port_spec(spec: &str) -> (u16, crate::container::Protocol) {
    let parts: Vec<&str> = spec.split('/').collect();
    let port = parts.first()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(0);
    let protocol = parts.get(1)
        .map(|p| if *p == "udp" { crate::container::Protocol::Udp } else { crate::container::Protocol::Tcp })
        .unwrap_or(crate::container::Protocol::Tcp);
    (port, protocol)
}

/// Format container status string like Docker does
fn format_container_status(
    status: &crate::container::ContainerStatus,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    finished_at: Option<chrono::DateTime<chrono::Utc>>,
) -> String {
    use crate::container::ContainerStatus;
    
    match status {
        ContainerStatus::Running => {
            if let Some(started) = started_at {
                let duration = chrono::Utc::now().signed_duration_since(started);
                format!("Up {}", format_duration(duration))
            } else {
                "Up".to_string()
            }
        }
        ContainerStatus::Exited | ContainerStatus::Stopped => {
            if let Some(finished) = finished_at {
                let duration = chrono::Utc::now().signed_duration_since(finished);
                format!("Exited (0) {} ago", format_duration(duration))
            } else {
                "Exited".to_string()
            }
        }
        ContainerStatus::Created => "Created".to_string(),
        ContainerStatus::Paused => "Paused".to_string(),
        ContainerStatus::Dead => "Dead".to_string(),
        _ => status.to_string(),
    }
}

/// Format duration in human-readable format
fn format_duration(duration: chrono::Duration) -> String {
    let seconds = duration.num_seconds();
    if seconds < 60 {
        format!("{} seconds", seconds)
    } else if seconds < 3600 {
        format!("{} minutes", seconds / 60)
    } else if seconds < 86400 {
        format!("{} hours", seconds / 3600)
    } else {
        format!("{} days", seconds / 86400)
    }
}

/// Parse a query parameter as u32
fn parse_query_param(path: &str, param: &str) -> Option<u32> {
    let query = path.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
            if key == param {
                return value.parse().ok();
            }
        }
    }
    None
}

/// Parse a query parameter as string
fn parse_query_string(path: &str, param: &str) -> Option<String> {
    let query = path.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
            if key == param {
                return Some(urlencoding_decode(value).unwrap_or_else(|_| value.to_string()));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_handler() -> ApiHandler {
        let temp_dir = TempDir::new().unwrap();
        let manager = Arc::new(ContainerManager::new(temp_dir.path().to_path_buf()).unwrap());
        ApiHandler::new(manager)
    }

    #[test]
    fn test_get_version() {
        let handler = create_test_handler();
        let result = handler.get_version();
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Version"));
    }

    #[test]
    fn test_get_info() {
        let handler = create_test_handler();
        let result = handler.get_info();
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Containers"));
    }

    #[test]
    fn test_ping() {
        let handler = create_test_handler();
        let result = handler.handle_request("GET", "/_ping", "");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "OK");
    }
}
