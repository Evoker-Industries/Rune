//! Image store - manages local container images

use crate::error::{Result, RuneError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Container image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    /// Image ID (sha256 hash)
    pub id: String,
    /// Repository tags (e.g., ["nginx:latest", "nginx:1.21"])
    pub repo_tags: Vec<String>,
    /// Repository digests
    pub repo_digests: Vec<String>,
    /// Parent image ID
    pub parent: String,
    /// Comment
    pub comment: String,
    /// Created timestamp
    pub created: DateTime<Utc>,
    /// Container ID used to create this image
    pub container: String,
    /// Docker version used to create this image
    pub docker_version: String,
    /// Author
    pub author: String,
    /// Image configuration
    pub config: ImageConfig,
    /// Architecture
    pub architecture: String,
    /// Operating system
    pub os: String,
    /// OS version
    pub os_version: Option<String>,
    /// Image size in bytes
    pub size: u64,
    /// Virtual size in bytes
    pub virtual_size: u64,
    /// Image layers
    pub layers: Vec<String>,
}

impl Default for Image {
    fn default() -> Self {
        Self {
            id: String::new(),
            repo_tags: Vec::new(),
            repo_digests: Vec::new(),
            parent: String::new(),
            comment: String::new(),
            created: Utc::now(),
            container: String::new(),
            docker_version: env!("CARGO_PKG_VERSION").to_string(),
            author: String::new(),
            config: ImageConfig::default(),
            architecture: std::env::consts::ARCH.to_string(),
            os: std::env::consts::OS.to_string(),
            os_version: None,
            size: 0,
            virtual_size: 0,
            layers: Vec::new(),
        }
    }
}

/// Image configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageConfig {
    /// Hostname
    pub hostname: String,
    /// Domain name
    pub domainname: String,
    /// User
    pub user: String,
    /// Attach stdin
    pub attach_stdin: bool,
    /// Attach stdout
    pub attach_stdout: bool,
    /// Attach stderr
    pub attach_stderr: bool,
    /// Exposed ports
    pub exposed_ports: HashMap<String, HashMap<String, String>>,
    /// TTY
    pub tty: bool,
    /// Open stdin
    pub open_stdin: bool,
    /// Stdin once
    pub stdin_once: bool,
    /// Environment variables
    pub env: Vec<String>,
    /// Command
    pub cmd: Vec<String>,
    /// Healthcheck
    pub healthcheck: Option<HealthConfig>,
    /// Args escaped (Windows)
    pub args_escaped: bool,
    /// Image hash
    pub image: String,
    /// Volumes
    pub volumes: HashMap<String, HashMap<String, String>>,
    /// Working directory
    pub working_dir: String,
    /// Entrypoint
    pub entrypoint: Vec<String>,
    /// Network disabled
    pub network_disabled: bool,
    /// Mac address
    pub mac_address: String,
    /// On build triggers
    pub on_build: Vec<String>,
    /// Labels
    pub labels: HashMap<String, String>,
    /// Stop signal
    pub stop_signal: String,
    /// Stop timeout
    pub stop_timeout: Option<u32>,
    /// Shell
    pub shell: Vec<String>,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Test command
    pub test: Vec<String>,
    /// Interval between checks
    pub interval: u64,
    /// Timeout for check
    pub timeout: u64,
    /// Start period
    pub start_period: u64,
    /// Number of retries
    pub retries: u32,
}

/// Image store for managing local images
pub struct ImageStore {
    /// Images indexed by ID
    images: Arc<RwLock<HashMap<String, Image>>>,
    /// Tag to ID mapping
    tags: Arc<RwLock<HashMap<String, String>>>,
    /// Storage path
    storage_path: PathBuf,
}

impl ImageStore {
    /// Create a new image store
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;
        std::fs::create_dir_all(storage_path.join("layers"))?;
        std::fs::create_dir_all(storage_path.join("manifests"))?;

        Ok(Self {
            images: Arc::new(RwLock::new(HashMap::new())),
            tags: Arc::new(RwLock::new(HashMap::new())),
            storage_path,
        })
    }

    /// Store an image
    pub fn store(&self, image: Image) -> Result<()> {
        let mut images = self
            .images
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;
        let mut tags = self
            .tags
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        // Update tag mappings
        for tag in &image.repo_tags {
            tags.insert(tag.clone(), image.id.clone());
        }

        images.insert(image.id.clone(), image);
        Ok(())
    }

    /// Get image by ID or tag
    pub fn get(&self, reference: &str) -> Result<Image> {
        let images = self
            .images
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;
        let tags = self
            .tags
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        // Try direct ID lookup
        if let Some(image) = images.get(reference) {
            return Ok(image.clone());
        }

        // Try tag lookup
        if let Some(id) = tags.get(reference) {
            if let Some(image) = images.get(id) {
                return Ok(image.clone());
            }
        }

        // Try partial ID match
        for (id, image) in images.iter() {
            if id.starts_with(reference) {
                return Ok(image.clone());
            }
        }

        Err(RuneError::ImageNotFound(reference.to_string()))
    }

    /// List all images
    pub fn list(&self) -> Result<Vec<Image>> {
        let images = self
            .images
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        Ok(images.values().cloned().collect())
    }

    /// Remove an image
    pub fn remove(&self, reference: &str, force: bool) -> Result<()> {
        let mut images = self
            .images
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;
        let mut tags = self
            .tags
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        // Find the image
        let id = if images.contains_key(reference) {
            reference.to_string()
        } else if let Some(id) = tags.get(reference) {
            id.clone()
        } else {
            // Try partial ID match
            let mut found = None;
            for id in images.keys() {
                if id.starts_with(reference) {
                    found = Some(id.clone());
                    break;
                }
            }
            found.ok_or_else(|| RuneError::ImageNotFound(reference.to_string()))?
        };

        let image = images
            .get(&id)
            .ok_or_else(|| RuneError::ImageNotFound(reference.to_string()))?;

        // Remove tag mappings
        for tag in &image.repo_tags {
            tags.remove(tag);
        }

        // Remove image
        images.remove(&id);

        // Clean up storage
        let image_path = self.storage_path.join(&id);
        if image_path.exists() && force {
            std::fs::remove_dir_all(image_path)?;
        }

        Ok(())
    }

    /// Tag an image
    pub fn tag(&self, source: &str, target: &str) -> Result<()> {
        let mut images = self
            .images
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;
        let mut tags = self
            .tags
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        // Find source image
        let id = if images.contains_key(source) {
            source.to_string()
        } else if let Some(id) = tags.get(source) {
            id.clone()
        } else {
            return Err(RuneError::ImageNotFound(source.to_string()));
        };

        // Add new tag
        tags.insert(target.to_string(), id.clone());

        // Update image repo_tags
        if let Some(image) = images.get_mut(&id) {
            if !image.repo_tags.contains(&target.to_string()) {
                image.repo_tags.push(target.to_string());
            }
        }

        Ok(())
    }

    /// Get storage path
    pub fn storage_path(&self) -> &PathBuf {
        &self.storage_path
    }

    /// Prune unused images
    pub fn prune(&self) -> Result<Vec<String>> {
        let images = self
            .images
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        // Find dangling images (no tags)
        let dangling: Vec<String> = images
            .iter()
            .filter(|(_, img)| img.repo_tags.is_empty())
            .map(|(id, _)| id.clone())
            .collect();

        drop(images);

        // Remove dangling images
        for id in &dangling {
            self.remove(id, true)?;
        }

        Ok(dangling)
    }
}
