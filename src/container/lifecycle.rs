//! Container lifecycle management

use super::config::{ContainerConfig, ContainerStatus};
use super::runtime::Container;
use crate::error::{Result, RuneError};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Container manager for handling container lifecycle
pub struct ContainerManager {
    /// All containers indexed by ID
    containers: Arc<RwLock<HashMap<String, Container>>>,
    /// Base path for container storage
    base_path: PathBuf,
}

impl ContainerManager {
    /// Create a new container manager
    pub fn new(base_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&base_path)?;
        
        Ok(Self {
            containers: Arc::new(RwLock::new(HashMap::new())),
            base_path,
        })
    }

    /// Create a new container
    pub fn create(&self, config: ContainerConfig) -> Result<String> {
        let container = Container::new(config, &self.base_path)?;
        let id = container.id().to_string();
        
        let mut containers = self.containers.write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;
        
        if containers.contains_key(&id) {
            return Err(RuneError::ContainerExists(id));
        }
        
        containers.insert(id.clone(), container);
        Ok(id)
    }

    /// Start a container
    pub fn start(&self, id: &str) -> Result<()> {
        let mut containers = self.containers.write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;
        
        let container = containers.get_mut(id)
            .ok_or_else(|| RuneError::ContainerNotFound(id.to_string()))?;
        
        container.start()
    }

    /// Stop a container
    pub fn stop(&self, id: &str) -> Result<()> {
        let mut containers = self.containers.write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;
        
        let container = containers.get_mut(id)
            .ok_or_else(|| RuneError::ContainerNotFound(id.to_string()))?;
        
        container.stop()
    }

    /// Pause a container
    pub fn pause(&self, id: &str) -> Result<()> {
        let mut containers = self.containers.write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;
        
        let container = containers.get_mut(id)
            .ok_or_else(|| RuneError::ContainerNotFound(id.to_string()))?;
        
        container.pause()
    }

    /// Unpause a container
    pub fn unpause(&self, id: &str) -> Result<()> {
        let mut containers = self.containers.write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;
        
        let container = containers.get_mut(id)
            .ok_or_else(|| RuneError::ContainerNotFound(id.to_string()))?;
        
        container.unpause()
    }

    /// Kill a container
    pub fn kill(&self, id: &str, signal: Option<i32>) -> Result<()> {
        let mut containers = self.containers.write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;
        
        let container = containers.get_mut(id)
            .ok_or_else(|| RuneError::ContainerNotFound(id.to_string()))?;
        
        container.kill(signal)
    }

    /// Remove a container
    pub fn remove(&self, id: &str, force: bool) -> Result<()> {
        let mut containers = self.containers.write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;
        
        let container = containers.get_mut(id)
            .ok_or_else(|| RuneError::ContainerNotFound(id.to_string()))?;
        
        if force && container.is_running() {
            container.kill(Some(9))?;
        }
        
        container.remove()?;
        containers.remove(id);
        
        Ok(())
    }

    /// Get container by ID
    pub fn get(&self, id: &str) -> Result<ContainerConfig> {
        let containers = self.containers.read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;
        
        containers.get(id)
            .map(|c| c.config.clone())
            .ok_or_else(|| RuneError::ContainerNotFound(id.to_string()))
    }

    /// List all containers
    pub fn list(&self, all: bool) -> Result<Vec<ContainerConfig>> {
        let containers = self.containers.read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;
        
        let result: Vec<ContainerConfig> = containers.values()
            .filter(|c| all || c.config.status == ContainerStatus::Running)
            .map(|c| c.config.clone())
            .collect();
        
        Ok(result)
    }

    /// Find container by name
    pub fn find_by_name(&self, name: &str) -> Result<Option<ContainerConfig>> {
        let containers = self.containers.read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;
        
        let result = containers.values()
            .find(|c| c.config.name == name)
            .map(|c| c.config.clone());
        
        Ok(result)
    }

    /// Get container count
    pub fn count(&self) -> Result<usize> {
        let containers = self.containers.read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;
        
        Ok(containers.len())
    }

    /// Get running container count
    pub fn running_count(&self) -> Result<usize> {
        let containers = self.containers.read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;
        
        let count = containers.values()
            .filter(|c| c.config.status == ContainerStatus::Running)
            .count();
        
        Ok(count)
    }
}
