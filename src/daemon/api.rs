//! REST API Handler for Rune Daemon
//!
//! Implements Docker-compatible REST API endpoints.

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerCreateResponse {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Warnings")]
    pub warnings: Vec<String>,
}

/// Version info response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct VersionResponse {
    version: String,
    api_version: String,
    min_api_version: String,
    go_version: String,
    os: String,
    arch: String,
}

/// System info response
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
    #[serde(rename = "NCPU")]
    ncpu: usize,
    mem_total: i64,
    docker_root_dir: String,
    name: String,
    server_version: String,
    default_runtime: String,
    #[serde(rename = "OSType")]
    os_type: String,
    architecture: String,
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
}

/// Container inspect response
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

/// API Handler for processing requests
#[derive(Clone)]
pub struct ApiHandler {
    container_manager: Arc<ContainerManager>,
}

impl ApiHandler {
    /// Create a new API handler
    pub fn new(container_manager: Arc<ContainerManager>) -> Self {
        Self { container_manager }
    }

    /// Handle an incoming API request
    pub fn handle_request(&self, method: &str, path: &str, body: &str) -> Result<String> {
        debug!("API request: {} {} body={}", method, path, body.len());

        let path_parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        match (method, path_parts.as_slice()) {
            // Version and info
            ("GET", ["version"]) | ("GET", ["v1.43", "version"]) => self.get_version(),
            ("GET", ["info"]) | ("GET", ["v1.43", "info"]) => self.get_info(),
            ("GET", ["_ping"]) | ("GET", ["v1.43", "_ping"]) => Ok("OK".to_string()),
            ("HEAD", ["_ping"]) | ("HEAD", ["v1.43", "_ping"]) => Ok("".to_string()),

            // Containers
            ("GET", ["containers", "json"]) | ("GET", ["v1.43", "containers", "json"]) => {
                self.list_containers(path)
            }
            ("POST", ["containers", "create"]) | ("POST", ["v1.43", "containers", "create"]) => {
                self.create_container(body, path)
            }
            ("GET", ["containers", id, "json"]) | ("GET", ["v1.43", "containers", id, "json"]) => {
                self.inspect_container(id)
            }
            ("POST", ["containers", id, "start"]) | ("POST", ["v1.43", "containers", id, "start"]) => {
                self.start_container(id)
            }
            ("POST", ["containers", id, "stop"]) | ("POST", ["v1.43", "containers", id, "stop"]) => {
                self.stop_container(id)
            }
            ("POST", ["containers", id, "restart"]) | ("POST", ["v1.43", "containers", id, "restart"]) => {
                self.restart_container(id)
            }
            ("POST", ["containers", id, "kill"]) | ("POST", ["v1.43", "containers", id, "kill"]) => {
                self.stop_container(id)
            }
            ("DELETE", ["containers", id]) | ("DELETE", ["v1.43", "containers", id]) => {
                self.remove_container(id, path)
            }
            ("GET", ["containers", id, "logs"]) | ("GET", ["v1.43", "containers", id, "logs"]) => {
                Ok("".to_string())
            }

            // Images
            ("GET", ["images", "json"]) | ("GET", ["v1.43", "images", "json"]) => {
                Ok("[]".to_string())
            }

            // Networks
            ("GET", ["networks"]) | ("GET", ["v1.43", "networks"]) => self.list_networks(),

            // Volumes
            ("GET", ["volumes"]) | ("GET", ["v1.43", "volumes"]) => {
                Ok(r#"{"Volumes":[],"Warnings":[]}"#.to_string())
            }

            // Default
            _ => Err(RuneError::Api(format!("Unknown endpoint: {} {}", method, path))),
        }
    }

    fn get_version(&self) -> Result<String> {
        let response = VersionResponse {
            version: env!("CARGO_PKG_VERSION").to_string(),
            api_version: "1.43".to_string(),
            min_api_version: "1.12".to_string(),
            go_version: "N/A (Rust)".to_string(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
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

        let response = InfoResponse {
            id: uuid::Uuid::new_v4().to_string(),
            containers,
            containers_running: running,
            containers_paused: 0,
            containers_stopped: containers - running,
            images: 0,
            driver: "overlay2".to_string(),
            ncpu: num_cpus::get(),
            mem_total: 0,
            docker_root_dir: "/var/lib/rune".to_string(),
            name: gethostname::gethostname().to_string_lossy().to_string(),
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            default_runtime: "rune".to_string(),
            os_type: "linux".to_string(),
            architecture: std::env::consts::ARCH.to_string(),
        };
        Ok(serde_json::to_string(&response)?)
    }

    fn list_containers(&self, path: &str) -> Result<String> {
        let all = path.contains("all=true") || path.contains("all=1");
        let containers = self.container_manager.list(all)?;
        
        let response: Vec<ContainerListItem> = containers
            .iter()
            .map(|c| ContainerListItem {
                id: c.id.clone(),
                names: vec![format!("/{}", c.name)],
                image: c.image.clone(),
                image_id: "".to_string(),
                command: "".to_string(),
                created: c.created_at.timestamp(),
                state: c.status.to_string().to_lowercase(),
                status: c.status.to_string(),
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

        if let Some(cmd) = request.cmd {
            config.cmd = cmd;
        }
        if let Some(env) = request.env {
            for e in env {
                if let Some((key, value)) = e.split_once('=') {
                    config.env.insert(key.to_string(), value.to_string());
                }
            }
        }
        if let Some(wd) = request.working_dir {
            config.working_dir = wd;
        }
        if let Some(hostname) = request.hostname {
            config.hostname = hostname;
        }
        if let Some(user) = request.user {
            config.user = user;
        }

        let id = self.container_manager.create(config)?;
        let response = ContainerCreateResponse { id, warnings: vec![] };
        Ok(serde_json::to_string(&response)?)
    }

    fn inspect_container(&self, id: &str) -> Result<String> {
        let container = self.container_manager.get(id)?;

        let response = ContainerInspect {
            id: container.id.clone(),
            created: container.created_at.to_rfc3339(),
            path: "".to_string(),
            args: vec![],
            state: ContainerState {
                status: container.status.to_string().to_lowercase(),
                running: container.status.to_string() == "Running",
                paused: container.status.to_string() == "Paused",
                restarting: false,
                oom_killed: false,
                dead: false,
                pid: 0,
                exit_code: 0,
                error: "".to_string(),
                started_at: "".to_string(),
                finished_at: "".to_string(),
            },
            image: container.image.clone(),
            name: format!("/{}", container.name),
            restart_count: 0,
            driver: "overlay2".to_string(),
            platform: "linux".to_string(),
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
            {"Name": "bridge", "Id": "bridge", "Driver": "bridge", "Scope": "local"},
            {"Name": "host", "Id": "host", "Driver": "host", "Scope": "local"},
            {"Name": "none", "Id": "none", "Driver": "null", "Scope": "local"}
        ]);
        Ok(response.to_string())
    }
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
