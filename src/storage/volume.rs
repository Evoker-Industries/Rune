//! Volume management

use crate::error::{Result, RuneError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Volume driver types
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VolumeDriver {
    /// Local filesystem driver
    #[default]
    Local,
    /// NFS driver
    Nfs,
    /// Custom driver
    Custom(String),
}

impl std::fmt::Display for VolumeDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VolumeDriver::Local => write!(f, "local"),
            VolumeDriver::Nfs => write!(f, "nfs"),
            VolumeDriver::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Volume scope
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VolumeScope {
    /// Local to this node
    #[default]
    Local,
    /// Global (swarm)
    Global,
}

/// Volume configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    /// Volume name
    pub name: String,
    /// Volume driver
    pub driver: VolumeDriver,
    /// Mount point on host
    pub mountpoint: PathBuf,
    /// Volume scope
    pub scope: VolumeScope,
    /// Driver options
    pub options: HashMap<String, String>,
    /// Volume labels
    pub labels: HashMap<String, String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Usage data
    pub usage_data: Option<VolumeUsageData>,
    /// Status
    pub status: HashMap<String, String>,
}

impl Volume {
    /// Create a new volume
    pub fn new(name: &str, base_path: &Path) -> Self {
        Self {
            name: name.to_string(),
            driver: VolumeDriver::Local,
            mountpoint: base_path.join(name),
            scope: VolumeScope::Local,
            options: HashMap::new(),
            labels: HashMap::new(),
            created_at: Utc::now(),
            usage_data: None,
            status: HashMap::new(),
        }
    }

    /// Set driver
    pub fn driver(mut self, driver: VolumeDriver) -> Self {
        self.driver = driver;
        self
    }

    /// Add option
    pub fn option(mut self, key: &str, value: &str) -> Self {
        self.options.insert(key.to_string(), value.to_string());
        self
    }

    /// Add label
    pub fn label(mut self, key: &str, value: &str) -> Self {
        self.labels.insert(key.to_string(), value.to_string());
        self
    }

    /// Get size in bytes
    pub fn size(&self) -> Result<u64> {
        if !self.mountpoint.exists() {
            return Ok(0);
        }

        let mut total = 0u64;
        for entry in walkdir::WalkDir::new(&self.mountpoint)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Ok(metadata) = entry.metadata() {
                total += metadata.len();
            }
        }

        Ok(total)
    }
}

/// Volume usage data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeUsageData {
    /// Size in bytes
    pub size: i64,
    /// Reference count (number of containers using this volume)
    pub ref_count: i64,
}

/// Volume manager
pub struct VolumeManager {
    /// Volumes indexed by name
    volumes: Arc<RwLock<HashMap<String, Volume>>>,
    /// Base path for volume storage
    base_path: PathBuf,
}

impl VolumeManager {
    /// Create a new volume manager
    pub fn new(base_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&base_path)?;

        Ok(Self {
            volumes: Arc::new(RwLock::new(HashMap::new())),
            base_path,
        })
    }

    /// Create a new volume
    pub fn create(
        &self,
        name: &str,
        driver: Option<VolumeDriver>,
        options: HashMap<String, String>,
        labels: HashMap<String, String>,
    ) -> Result<Volume> {
        let mut volumes = self
            .volumes
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        if volumes.contains_key(name) {
            return Err(RuneError::Volume(format!("Volume {} already exists", name)));
        }

        // Generate name if not provided
        let volume_name = if name.is_empty() {
            Uuid::new_v4().to_string().replace("-", "")[..12].to_string()
        } else {
            name.to_string()
        };

        let mut volume = Volume::new(&volume_name, &self.base_path);

        if let Some(d) = driver {
            volume.driver = d;
        }
        volume.options = options;
        volume.labels = labels;

        // Create the volume directory
        std::fs::create_dir_all(&volume.mountpoint)?;

        volumes.insert(volume_name.clone(), volume.clone());

        Ok(volume)
    }

    /// Get a volume by name
    pub fn get(&self, name: &str) -> Result<Volume> {
        let volumes = self
            .volumes
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        volumes
            .get(name)
            .cloned()
            .ok_or_else(|| RuneError::VolumeNotFound(name.to_string()))
    }

    /// List all volumes
    pub fn list(&self) -> Result<Vec<Volume>> {
        let volumes = self
            .volumes
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        Ok(volumes.values().cloned().collect())
    }

    /// Remove a volume
    pub fn remove(&self, name: &str, force: bool) -> Result<()> {
        let mut volumes = self
            .volumes
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        let volume = volumes
            .get(name)
            .ok_or_else(|| RuneError::VolumeNotFound(name.to_string()))?;

        // Check if volume is in use
        if let Some(ref usage) = volume.usage_data {
            if usage.ref_count > 0 && !force {
                return Err(RuneError::Volume(format!(
                    "Volume {} is in use by {} container(s)",
                    name, usage.ref_count
                )));
            }
        }

        // Remove the directory
        if volume.mountpoint.exists() {
            std::fs::remove_dir_all(&volume.mountpoint)?;
        }

        volumes.remove(name);

        Ok(())
    }

    /// Prune unused volumes
    pub fn prune(&self) -> Result<Vec<String>> {
        let volumes = self
            .volumes
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        // Find unused volumes
        let to_remove: Vec<String> = volumes
            .iter()
            .filter(|(_, v)| {
                v.usage_data
                    .as_ref()
                    .map(|u| u.ref_count == 0)
                    .unwrap_or(true)
            })
            .map(|(name, _)| name.clone())
            .collect();

        drop(volumes);

        // Remove volumes
        for name in &to_remove {
            self.remove(name, true)?;
        }

        Ok(to_remove)
    }

    /// Increment reference count for a volume
    pub fn add_reference(&self, name: &str) -> Result<()> {
        let mut volumes = self
            .volumes
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        let volume = volumes
            .get_mut(name)
            .ok_or_else(|| RuneError::VolumeNotFound(name.to_string()))?;

        match &mut volume.usage_data {
            Some(usage) => usage.ref_count += 1,
            None => {
                volume.usage_data = Some(VolumeUsageData {
                    size: 0,
                    ref_count: 1,
                });
            }
        }

        Ok(())
    }

    /// Decrement reference count for a volume
    pub fn remove_reference(&self, name: &str) -> Result<()> {
        let mut volumes = self
            .volumes
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        let volume = volumes
            .get_mut(name)
            .ok_or_else(|| RuneError::VolumeNotFound(name.to_string()))?;

        if let Some(ref mut usage) = volume.usage_data {
            usage.ref_count = (usage.ref_count - 1).max(0);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_volume() {
        let temp = tempdir().unwrap();
        let manager = VolumeManager::new(temp.path().to_path_buf()).unwrap();

        let volume = manager
            .create("test-volume", None, HashMap::new(), HashMap::new())
            .unwrap();
        assert_eq!(volume.name, "test-volume");
        assert!(volume.mountpoint.exists());
    }

    #[test]
    fn test_remove_volume() {
        let temp = tempdir().unwrap();
        let manager = VolumeManager::new(temp.path().to_path_buf()).unwrap();

        manager
            .create("test-volume", None, HashMap::new(), HashMap::new())
            .unwrap();
        manager.remove("test-volume", false).unwrap();

        assert!(manager.get("test-volume").is_err());
    }

    #[test]
    fn test_volume_reference_counting() {
        let temp = tempdir().unwrap();
        let manager = VolumeManager::new(temp.path().to_path_buf()).unwrap();

        manager
            .create("test-volume", None, HashMap::new(), HashMap::new())
            .unwrap();

        manager.add_reference("test-volume").unwrap();
        manager.add_reference("test-volume").unwrap();

        let volume = manager.get("test-volume").unwrap();
        assert_eq!(volume.usage_data.unwrap().ref_count, 2);
    }
}
