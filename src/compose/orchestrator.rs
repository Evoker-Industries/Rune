//! Docker Compose orchestrator

use super::config::{ComposeConfig, ServiceConfig, DependsOnConfig};
use crate::container::{ContainerConfig, ContainerManager, ContainerStatus};
use crate::error::{Result, RuneError};
use crate::image::builder::{BuildContext, ImageBuilder};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

/// Compose project state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectState {
    /// Project is not running
    Stopped,
    /// Project is starting
    Starting,
    /// Project is running
    Running,
    /// Project is stopping
    Stopping,
    /// Project has errors
    Error,
}

/// Service state
#[derive(Debug, Clone)]
pub struct ServiceState {
    /// Service name
    pub name: String,
    /// Container IDs for this service
    pub container_ids: Vec<String>,
    /// Desired replica count
    pub replicas: u32,
    /// Current state
    pub state: ContainerStatus,
}

/// Compose orchestrator
pub struct ComposeOrchestrator {
    /// Project name
    project_name: String,
    /// Compose configuration
    config: ComposeConfig,
    /// Container manager
    container_manager: Arc<ContainerManager>,
    /// Service states
    service_states: HashMap<String, ServiceState>,
    /// Project working directory
    working_dir: PathBuf,
}

impl ComposeOrchestrator {
    /// Create a new orchestrator
    pub fn new(
        project_name: &str,
        config: ComposeConfig,
        container_manager: Arc<ContainerManager>,
        working_dir: PathBuf,
    ) -> Self {
        Self {
            project_name: project_name.to_string(),
            config,
            container_manager,
            service_states: HashMap::new(),
            working_dir,
        }
    }

    /// Start the compose project
    pub async fn up(&mut self, detach: bool, build: bool) -> Result<()> {
        tracing::info!("Starting compose project: {}", self.project_name);

        // Build images if requested
        if build {
            self.build_services().await?;
        }

        // Get service start order
        let order = self.get_start_order()?;

        // Start services in order
        for service_name in order {
            self.start_service(&service_name).await?;
        }

        if !detach {
            // In non-detached mode, we would attach to logs here
            tracing::info!("Project {} is running (attached mode not implemented)", self.project_name);
        }

        Ok(())
    }

    /// Stop the compose project
    pub async fn down(&mut self, remove_volumes: bool) -> Result<()> {
        tracing::info!("Stopping compose project: {}", self.project_name);

        // Stop services in reverse order
        let order = self.get_start_order()?;
        for service_name in order.into_iter().rev() {
            self.stop_service(&service_name).await?;
        }

        // Remove volumes if requested
        if remove_volumes {
            // Volume removal would go here
        }

        Ok(())
    }

    /// Start a specific service
    pub async fn start_service(&mut self, service_name: &str) -> Result<()> {
        let service = self.config.services.get(service_name)
            .ok_or_else(|| RuneError::ServiceNotFound(service_name.to_string()))?
            .clone();

        let replicas = service.deploy
            .as_ref()
            .and_then(|d| d.replicas)
            .unwrap_or(1);

        tracing::info!("Starting service {} with {} replicas", service_name, replicas);

        let mut container_ids = Vec::new();

        for i in 0..replicas {
            let container_name = if replicas == 1 {
                format!("{}-{}-1", self.project_name, service_name)
            } else {
                format!("{}-{}-{}", self.project_name, service_name, i + 1)
            };

            let container_config = self.service_to_container_config(service_name, &service, &container_name)?;

            let id = self.container_manager.create(container_config)?;
            self.container_manager.start(&id)?;
            container_ids.push(id);
        }

        self.service_states.insert(service_name.to_string(), ServiceState {
            name: service_name.to_string(),
            container_ids,
            replicas,
            state: ContainerStatus::Running,
        });

        Ok(())
    }

    /// Stop a specific service
    pub async fn stop_service(&mut self, service_name: &str) -> Result<()> {
        if let Some(state) = self.service_states.get(service_name) {
            for id in &state.container_ids {
                if let Err(e) = self.container_manager.stop(id) {
                    tracing::warn!("Failed to stop container {}: {}", id, e);
                }
            }
        }

        if let Some(state) = self.service_states.get_mut(service_name) {
            state.state = ContainerStatus::Stopped;
        }

        Ok(())
    }

    /// Restart a service
    pub async fn restart_service(&mut self, service_name: &str) -> Result<()> {
        self.stop_service(service_name).await?;
        self.start_service(service_name).await?;
        Ok(())
    }

    /// Scale a service
    pub async fn scale(&mut self, service_name: &str, replicas: u32) -> Result<()> {
        let current = self.service_states.get(service_name)
            .map(|s| s.replicas)
            .unwrap_or(0);

        if replicas > current {
            // Scale up
            let service = self.config.services.get(service_name)
                .ok_or_else(|| RuneError::ServiceNotFound(service_name.to_string()))?
                .clone();

            for i in current..replicas {
                let container_name = format!("{}-{}-{}", self.project_name, service_name, i + 1);
                let container_config = self.service_to_container_config(service_name, &service, &container_name)?;

                let id = self.container_manager.create(container_config)?;
                self.container_manager.start(&id)?;

                if let Some(state) = self.service_states.get_mut(service_name) {
                    state.container_ids.push(id);
                    state.replicas = replicas;
                }
            }
        } else if replicas < current {
            // Scale down
            if let Some(state) = self.service_states.get_mut(service_name) {
                while state.container_ids.len() > replicas as usize {
                    if let Some(id) = state.container_ids.pop() {
                        self.container_manager.stop(&id)?;
                        self.container_manager.remove(&id, true)?;
                    }
                }
                state.replicas = replicas;
            }
        }

        Ok(())
    }

    /// Build service images
    pub async fn build_services(&self) -> Result<()> {
        for (name, service) in &self.config.services {
            if let Some(ref build_config) = service.build {
                tracing::info!("Building image for service: {}", name);

                let context_path = match build_config {
                    super::config::BuildConfig::Simple(path) => self.working_dir.join(path),
                    super::config::BuildConfig::Full(full) => {
                        full.context.as_ref()
                            .map(|p| self.working_dir.join(p))
                            .unwrap_or_else(|| self.working_dir.clone())
                    }
                };

                let build_context = BuildContext::new(context_path)
                    .tag(&format!("{}-{}:latest", self.project_name, name));

                let builder = ImageBuilder::new(build_context);
                builder.build().await?;
            }
        }

        Ok(())
    }

    /// Get service logs
    pub async fn logs(&self, service_name: Option<&str>, follow: bool, tail: Option<usize>) -> Result<Vec<String>> {
        let mut logs = Vec::new();

        let services: Vec<&str> = if let Some(name) = service_name {
            vec![name]
        } else {
            self.config.services.keys().map(|s| s.as_str()).collect()
        };

        for service in services {
            if let Some(state) = self.service_states.get(service) {
                for id in &state.container_ids {
                    // In a real implementation, we would get container logs here
                    logs.push(format!("[{}] Container {} logs...", service, id));
                }
            }
        }

        Ok(logs)
    }

    /// Get project status
    pub fn status(&self) -> HashMap<String, ServiceState> {
        self.service_states.clone()
    }

    /// Get service start order based on dependencies
    fn get_start_order(&self) -> Result<Vec<String>> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        for service_name in self.config.services.keys() {
            self.topological_sort(service_name, &mut visited, &mut visiting, &mut order)?;
        }

        Ok(order)
    }

    /// Topological sort for dependency resolution
    fn topological_sort(
        &self,
        service: &str,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) -> Result<()> {
        if visited.contains(service) {
            return Ok(());
        }

        if visiting.contains(service) {
            return Err(RuneError::Compose(format!(
                "Circular dependency detected for service: {}",
                service
            )));
        }

        visiting.insert(service.to_string());

        if let Some(service_config) = self.config.services.get(service) {
            if let Some(ref depends) = service_config.depends_on {
                let deps = match depends {
                    DependsOnConfig::Array(arr) => arr.clone(),
                    DependsOnConfig::Map(map) => map.keys().cloned().collect(),
                };

                for dep in deps {
                    self.topological_sort(&dep, visited, visiting, order)?;
                }
            }
        }

        visiting.remove(service);
        visited.insert(service.to_string());
        order.push(service.to_string());

        Ok(())
    }

    /// Convert service config to container config
    fn service_to_container_config(
        &self,
        service_name: &str,
        service: &ServiceConfig,
        container_name: &str,
    ) -> Result<ContainerConfig> {
        let image = service.image.clone()
            .unwrap_or_else(|| format!("{}-{}:latest", self.project_name, service_name));

        let mut config = ContainerConfig::new(container_name, &image);

        // Set command
        if let Some(ref cmd) = service.command {
            config.cmd = match cmd {
                super::config::CommandConfig::Shell(s) => vec!["/bin/sh".to_string(), "-c".to_string(), s.clone()],
                super::config::CommandConfig::Exec(arr) => arr.clone(),
            };
        }

        // Set entrypoint
        if let Some(ref ep) = service.entrypoint {
            config.entrypoint = match ep {
                super::config::CommandConfig::Shell(s) => vec![s.clone()],
                super::config::CommandConfig::Exec(arr) => arr.clone(),
            };
        }

        // Set environment
        if let Some(ref env) = service.environment {
            match env {
                super::config::EnvironmentConfig::Array(arr) => {
                    for item in arr {
                        if let Some((key, value)) = item.split_once('=') {
                            config.env.insert(key.to_string(), value.to_string());
                        }
                    }
                }
                super::config::EnvironmentConfig::Map(map) => {
                    for (key, value) in map {
                        if let Some(v) = value {
                            config.env.insert(key.clone(), v.clone());
                        }
                    }
                }
            }
        }

        // Set working directory
        if let Some(ref wd) = service.working_dir {
            config.working_dir = wd.clone();
        }

        // Set user
        if let Some(ref user) = service.user {
            config.user = user.clone();
        }

        // Set hostname
        if let Some(ref hostname) = service.hostname {
            config.hostname = hostname.clone();
        }

        // Set privileged
        if let Some(privileged) = service.privileged {
            config.privileged = privileged;
        }

        // Add labels
        config.labels.insert(
            "com.docker.compose.project".to_string(),
            self.project_name.clone(),
        );
        config.labels.insert(
            "com.docker.compose.service".to_string(),
            service_name.to_string(),
        );

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::parser::ComposeParser;
    use tempfile::tempdir;

    #[test]
    fn test_get_start_order() {
        let yaml = r#"
services:
  web:
    image: nginx
    depends_on:
      - api
  api:
    image: node
    depends_on:
      - db
  db:
    image: postgres
"#;

        let config = ComposeParser::parse_str(yaml).unwrap();
        let temp = tempdir().unwrap();
        let manager = Arc::new(ContainerManager::new(temp.path().to_path_buf()).unwrap());
        
        let orchestrator = ComposeOrchestrator::new("test", config, manager, temp.path().to_path_buf());
        let order = orchestrator.get_start_order().unwrap();

        // db should come before api, api before web
        let db_pos = order.iter().position(|s| s == "db").unwrap();
        let api_pos = order.iter().position(|s| s == "api").unwrap();
        let web_pos = order.iter().position(|s| s == "web").unwrap();

        assert!(db_pos < api_pos);
        assert!(api_pos < web_pos);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let yaml = r#"
services:
  a:
    image: nginx
    depends_on:
      - b
  b:
    image: nginx
    depends_on:
      - a
"#;

        let config = ComposeParser::parse_str(yaml).unwrap();
        let temp = tempdir().unwrap();
        let manager = Arc::new(ContainerManager::new(temp.path().to_path_buf()).unwrap());
        
        let orchestrator = ComposeOrchestrator::new("test", config, manager, temp.path().to_path_buf());
        let result = orchestrator.get_start_order();

        assert!(result.is_err());
    }
}
