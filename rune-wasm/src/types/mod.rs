//! Container and Docker API types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Container state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[wasm_bindgen]
pub enum ContainerState {
    Created,
    Running,
    Paused,
    Restarting,
    Removing,
    Exited,
    Dead,
}

/// Container information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Container {
    pub id: String,
    pub names: Vec<String>,
    pub image: String,
    pub image_id: String,
    pub command: String,
    pub created: i64,
    pub state: String,
    pub status: String,
    pub ports: Vec<PortBinding>,
    pub labels: HashMap<String, String>,
}

/// Port binding
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PortBinding {
    #[serde(rename = "IP")]
    pub ip: Option<String>,
    pub private_port: u16,
    pub public_port: Option<u16>,
    #[serde(rename = "Type")]
    pub port_type: String,
}

/// Container creation options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerCreateOptions {
    pub image: String,
    pub name: Option<String>,
    pub cmd: Option<Vec<String>>,
    pub env: Option<Vec<String>>,
    pub labels: Option<HashMap<String, String>>,
    pub working_dir: Option<String>,
    pub user: Option<String>,
    pub exposed_ports: Option<HashMap<String, serde_json::Value>>,
    pub host_config: Option<HostConfig>,
}

/// Host configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HostConfig {
    pub binds: Option<Vec<String>>,
    pub port_bindings: Option<HashMap<String, Vec<PortMap>>>,
    pub memory: Option<i64>,
    pub memory_swap: Option<i64>,
    pub cpu_shares: Option<i64>,
    pub privileged: Option<bool>,
    pub network_mode: Option<String>,
    pub restart_policy: Option<RestartPolicy>,
}

/// Port mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PortMap {
    pub host_ip: Option<String>,
    pub host_port: String,
}

/// Restart policy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RestartPolicy {
    pub name: String,
    pub maximum_retry_count: Option<i32>,
}

/// Image information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Image {
    pub id: String,
    pub parent_id: String,
    pub repo_tags: Vec<String>,
    pub repo_digests: Vec<String>,
    pub created: i64,
    pub size: i64,
    pub virtual_size: i64,
    pub labels: HashMap<String, String>,
}

/// Network information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Network {
    pub name: String,
    pub id: String,
    pub created: String,
    pub scope: String,
    pub driver: String,
    #[serde(rename = "IPAM")]
    pub ipam: IpamConfig,
    pub internal: bool,
    pub attachable: bool,
    pub ingress: bool,
    pub labels: HashMap<String, String>,
}

/// IPAM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IpamConfig {
    pub driver: String,
    pub config: Vec<IpamPoolConfig>,
}

/// IPAM pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IpamPoolConfig {
    pub subnet: Option<String>,
    pub gateway: Option<String>,
}

/// Volume information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Volume {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub created_at: String,
    pub labels: HashMap<String, String>,
    pub scope: String,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SystemInfo {
    pub id: String,
    pub containers: i32,
    pub containers_running: i32,
    pub containers_paused: i32,
    pub containers_stopped: i32,
    pub images: i32,
    pub driver: String,
    pub memory_limit: bool,
    pub swap_limit: bool,
    pub kernel_version: String,
    pub operating_system: String,
    pub os_type: String,
    pub architecture: String,
    #[serde(rename = "NCPU")]
    pub ncpu: i32,
    pub mem_total: i64,
    pub name: String,
    pub server_version: String,
}

/// Version information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Version {
    pub version: String,
    pub api_version: String,
    pub min_api_version: String,
    pub git_commit: String,
    pub go_version: String,
    pub os: String,
    pub arch: String,
    pub kernel_version: String,
    pub build_time: String,
}
