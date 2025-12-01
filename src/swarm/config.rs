//! Docker Config management for Swarm
//!
//! This module provides Docker Config support for storing and managing
//! non-sensitive configuration data across the swarm cluster.
//! Configs are similar to secrets but designed for non-confidential data
//! like configuration files.

use crate::error::{Result, RuneError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Docker Config specification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConfigSpec {
    /// Config name
    pub name: String,
    /// Labels
    #[serde(default)]
    pub labels: HashMap<String, String>,
    /// Base64-encoded config data
    pub data: String,
    /// Templating configuration
    pub templating: Option<ConfigTemplating>,
}

/// Config templating options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConfigTemplating {
    /// Templating engine name (e.g., "golang")
    pub name: Option<String>,
    /// Templating options
    #[serde(default)]
    pub options: HashMap<String, String>,
}

/// Docker Config object
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Config {
    /// Config ID
    #[serde(rename = "ID")]
    pub id: String,
    /// Config version
    pub version: ConfigVersion,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Config specification
    pub spec: ConfigSpec,
}

impl Config {
    /// Create a new config
    pub fn new(spec: ConfigSpec) -> Self {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        Self {
            id,
            version: ConfigVersion { index: 1 },
            created_at: now,
            updated_at: now,
            spec,
        }
    }

    /// Update the config specification
    pub fn update(&mut self, spec: ConfigSpec) -> Result<()> {
        // Config data cannot be changed after creation
        // Only labels can be updated
        if self.spec.data != spec.data {
            return Err(RuneError::InvalidConfig(
                "Config data cannot be modified after creation".to_string(),
            ));
        }

        self.spec.name = spec.name;
        self.spec.labels = spec.labels;
        self.spec.templating = spec.templating;
        self.version.index += 1;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Get the raw config data (base64 decoded)
    pub fn get_data(&self) -> Result<Vec<u8>> {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(&self.spec.data)
            .map_err(|e| RuneError::InvalidConfig(format!("Invalid base64 data: {}", e)))
    }

    /// Get the config data as a string (if valid UTF-8)
    pub fn get_data_string(&self) -> Result<String> {
        let bytes = self.get_data()?;
        String::from_utf8(bytes)
            .map_err(|e| RuneError::InvalidConfig(format!("Invalid UTF-8 data: {}", e)))
    }
}

/// Config version
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConfigVersion {
    /// Version index
    pub index: u64,
}

/// Config manager for storing and retrieving configs
pub struct ConfigManager {
    /// Stored configs
    configs: Arc<RwLock<HashMap<String, Config>>>,
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigManager {
    /// Create a new config manager
    pub fn new() -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new config
    pub fn create(&self, spec: ConfigSpec) -> Result<String> {
        let mut configs = self
            .configs
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        // Check for duplicate name
        for existing in configs.values() {
            if existing.spec.name == spec.name {
                return Err(RuneError::InvalidConfig(format!(
                    "Config with name '{}' already exists",
                    spec.name
                )));
            }
        }

        let config = Config::new(spec);
        let id = config.id.clone();
        configs.insert(id.clone(), config);
        Ok(id)
    }

    /// Get a config by ID or name
    pub fn get(&self, id_or_name: &str) -> Result<Config> {
        let configs = self
            .configs
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        // Try ID first
        if let Some(config) = configs.get(id_or_name) {
            return Ok(config.clone());
        }

        // Try name
        for config in configs.values() {
            if config.spec.name == id_or_name {
                return Ok(config.clone());
            }
        }

        Err(RuneError::InvalidConfig(format!(
            "Config not found: {}",
            id_or_name
        )))
    }

    /// List all configs
    pub fn list(&self, filters: Option<ConfigFilters>) -> Result<Vec<Config>> {
        let configs = self
            .configs
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        let mut result: Vec<Config> = configs.values().cloned().collect();

        // Apply filters
        if let Some(filters) = filters {
            if let Some(ref ids) = filters.id {
                result.retain(|c| ids.iter().any(|id| c.id.starts_with(id)));
            }
            if let Some(ref names) = filters.name {
                result.retain(|c| names.contains(&c.spec.name));
            }
            if let Some(ref labels) = filters.label {
                result.retain(|c| {
                    labels.iter().all(|label| {
                        if let Some((key, value)) = label.split_once('=') {
                            c.spec.labels.get(key).map(|v| v == value).unwrap_or(false)
                        } else {
                            c.spec.labels.contains_key(label)
                        }
                    })
                });
            }
        }

        Ok(result)
    }

    /// Update a config
    pub fn update(&self, id_or_name: &str, spec: ConfigSpec, version: u64) -> Result<()> {
        let mut configs = self
            .configs
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        // Find config by ID or name
        let config_id = if configs.contains_key(id_or_name) {
            id_or_name.to_string()
        } else {
            configs
                .values()
                .find(|c| c.spec.name == id_or_name)
                .map(|c| c.id.clone())
                .ok_or_else(|| {
                    RuneError::InvalidConfig(format!("Config not found: {}", id_or_name))
                })?
        };

        let config = configs
            .get_mut(&config_id)
            .ok_or_else(|| RuneError::InvalidConfig(format!("Config not found: {}", id_or_name)))?;

        // Check version for optimistic locking
        if config.version.index != version {
            return Err(RuneError::InvalidConfig(format!(
                "Config version mismatch: expected {}, got {}",
                config.version.index, version
            )));
        }

        config.update(spec)?;
        Ok(())
    }

    /// Remove a config
    pub fn remove(&self, id_or_name: &str) -> Result<()> {
        let mut configs = self
            .configs
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        // Try ID first
        if configs.remove(id_or_name).is_some() {
            return Ok(());
        }

        // Try name
        let config_id = configs
            .values()
            .find(|c| c.spec.name == id_or_name)
            .map(|c| c.id.clone());

        if let Some(id) = config_id {
            configs.remove(&id);
            Ok(())
        } else {
            Err(RuneError::InvalidConfig(format!(
                "Config not found: {}",
                id_or_name
            )))
        }
    }

    /// Get the number of configs
    pub fn count(&self) -> Result<usize> {
        let configs = self
            .configs
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;
        Ok(configs.len())
    }
}

/// Filters for listing configs
#[derive(Debug, Clone, Default)]
pub struct ConfigFilters {
    /// Filter by config ID (prefix match)
    pub id: Option<Vec<String>>,
    /// Filter by config name (exact match)
    pub name: Option<Vec<String>>,
    /// Filter by label
    pub label: Option<Vec<String>>,
}

impl ConfigFilters {
    /// Create filters from a JSON filter string
    pub fn from_json(json: &str) -> Result<Self> {
        let filters: HashMap<String, Vec<String>> = serde_json::from_str(json)
            .map_err(|e| RuneError::InvalidConfig(format!("Invalid filter JSON: {}", e)))?;

        Ok(Self {
            id: filters.get("id").cloned(),
            name: filters.get("name").cloned(),
            label: filters.get("label").cloned(),
        })
    }
}

/// Config reference for mounting in containers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConfigReferenceSpec {
    /// File configuration for the config
    pub file: Option<ConfigReferenceFileTarget>,
    /// Runtime configuration
    pub runtime: Option<ConfigReferenceRuntimeTarget>,
    /// Config ID
    #[serde(rename = "ConfigID")]
    pub config_id: String,
    /// Config name
    pub config_name: String,
}

/// File target for config reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConfigReferenceFileTarget {
    /// Target filename in the container
    pub name: String,
    /// UID for the file
    #[serde(rename = "UID")]
    pub uid: Option<String>,
    /// GID for the file
    #[serde(rename = "GID")]
    pub gid: Option<String>,
    /// File mode (permissions)
    pub mode: Option<u32>,
}

impl Default for ConfigReferenceFileTarget {
    fn default() -> Self {
        Self {
            name: String::new(),
            uid: Some("0".to_string()),
            gid: Some("0".to_string()),
            mode: Some(0o444), // Read-only by default
        }
    }
}

/// Runtime target for config reference
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConfigReferenceRuntimeTarget {
    // Empty for now, but can be extended for runtime-specific config injection
}

/// Create request for Docker API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConfigCreateRequest {
    /// Config name
    pub name: String,
    /// Labels
    #[serde(default)]
    pub labels: HashMap<String, String>,
    /// Base64-encoded config data
    pub data: String,
    /// Templating configuration
    pub templating: Option<ConfigTemplating>,
}

impl From<ConfigCreateRequest> for ConfigSpec {
    fn from(req: ConfigCreateRequest) -> Self {
        Self {
            name: req.name,
            labels: req.labels,
            data: req.data,
            templating: req.templating,
        }
    }
}

/// Create response for Docker API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConfigCreateResponse {
    /// Config ID
    #[serde(rename = "ID")]
    pub id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_spec(name: &str, data: &str) -> ConfigSpec {
        use base64::Engine;
        ConfigSpec {
            name: name.to_string(),
            labels: HashMap::new(),
            data: base64::engine::general_purpose::STANDARD.encode(data),
            templating: None,
        }
    }

    #[test]
    fn test_create_config() {
        let manager = ConfigManager::new();
        let spec = create_test_spec("my-config", "key=value\nfoo=bar");

        let id = manager.create(spec).unwrap();
        assert!(!id.is_empty());

        let config = manager.get(&id).unwrap();
        assert_eq!(config.spec.name, "my-config");
        assert_eq!(config.version.index, 1);
    }

    #[test]
    fn test_get_config_by_name() {
        let manager = ConfigManager::new();
        let spec = create_test_spec("test-config", "test data");

        manager.create(spec).unwrap();

        let config = manager.get("test-config").unwrap();
        assert_eq!(config.spec.name, "test-config");
    }

    #[test]
    fn test_get_config_data() {
        let manager = ConfigManager::new();
        let original_data = "server {\n  listen 80;\n}";
        let spec = create_test_spec("nginx-config", original_data);

        let id = manager.create(spec).unwrap();
        let config = manager.get(&id).unwrap();

        let data = config.get_data_string().unwrap();
        assert_eq!(data, original_data);
    }

    #[test]
    fn test_list_configs() {
        let manager = ConfigManager::new();

        manager
            .create(create_test_spec("config-1", "data1"))
            .unwrap();
        manager
            .create(create_test_spec("config-2", "data2"))
            .unwrap();
        manager
            .create(create_test_spec("config-3", "data3"))
            .unwrap();

        let configs = manager.list(None).unwrap();
        assert_eq!(configs.len(), 3);
    }

    #[test]
    fn test_list_configs_with_name_filter() {
        let manager = ConfigManager::new();

        manager
            .create(create_test_spec("app-config", "app"))
            .unwrap();
        manager.create(create_test_spec("db-config", "db")).unwrap();

        let filters = ConfigFilters {
            name: Some(vec!["app-config".to_string()]),
            ..Default::default()
        };

        let configs = manager.list(Some(filters)).unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].spec.name, "app-config");
    }

    #[test]
    fn test_list_configs_with_label_filter() {
        let manager = ConfigManager::new();

        let mut spec1 = create_test_spec("config-a", "data-a");
        spec1.labels.insert("env".to_string(), "prod".to_string());

        let mut spec2 = create_test_spec("config-b", "data-b");
        spec2.labels.insert("env".to_string(), "dev".to_string());

        manager.create(spec1).unwrap();
        manager.create(spec2).unwrap();

        let filters = ConfigFilters {
            label: Some(vec!["env=prod".to_string()]),
            ..Default::default()
        };

        let configs = manager.list(Some(filters)).unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].spec.name, "config-a");
    }

    #[test]
    fn test_remove_config() {
        let manager = ConfigManager::new();
        let spec = create_test_spec("to-delete", "will be deleted");

        let id = manager.create(spec).unwrap();
        assert!(manager.get(&id).is_ok());

        manager.remove(&id).unwrap();
        assert!(manager.get(&id).is_err());
    }

    #[test]
    fn test_remove_config_by_name() {
        let manager = ConfigManager::new();
        let spec = create_test_spec("named-config", "data");

        manager.create(spec).unwrap();
        manager.remove("named-config").unwrap();
        assert!(manager.get("named-config").is_err());
    }

    #[test]
    fn test_duplicate_name_error() {
        let manager = ConfigManager::new();
        let spec1 = create_test_spec("same-name", "data1");
        let spec2 = create_test_spec("same-name", "data2");

        manager.create(spec1).unwrap();
        let result = manager.create(spec2);

        assert!(result.is_err());
    }

    #[test]
    fn test_update_config_labels() {
        let manager = ConfigManager::new();
        let spec = create_test_spec("updatable", "fixed-data");
        let id = manager.create(spec).unwrap();

        let config = manager.get(&id).unwrap();
        let version = config.version.index;

        let mut new_spec = create_test_spec("updatable-renamed", "fixed-data");
        new_spec
            .labels
            .insert("new-label".to_string(), "value".to_string());

        manager.update(&id, new_spec, version).unwrap();

        let updated = manager.get(&id).unwrap();
        assert_eq!(updated.spec.name, "updatable-renamed");
        assert!(updated.spec.labels.contains_key("new-label"));
        assert_eq!(updated.version.index, 2);
    }

    #[test]
    fn test_update_config_data_fails() {
        let manager = ConfigManager::new();
        let spec = create_test_spec("immutable-data", "original");
        let id = manager.create(spec).unwrap();

        let config = manager.get(&id).unwrap();
        let version = config.version.index;

        let new_spec = create_test_spec("immutable-data", "changed");
        let result = manager.update(&id, new_spec, version);

        assert!(result.is_err());
    }

    #[test]
    fn test_version_mismatch_error() {
        let manager = ConfigManager::new();
        let spec = create_test_spec("versioned", "data");
        let id = manager.create(spec).unwrap();

        let new_spec = create_test_spec("versioned", "data");
        let result = manager.update(&id, new_spec, 999);

        assert!(result.is_err());
    }

    #[test]
    fn test_config_count() {
        let manager = ConfigManager::new();
        assert_eq!(manager.count().unwrap(), 0);

        manager.create(create_test_spec("c1", "d1")).unwrap();
        assert_eq!(manager.count().unwrap(), 1);

        manager.create(create_test_spec("c2", "d2")).unwrap();
        assert_eq!(manager.count().unwrap(), 2);

        manager.remove("c1").unwrap();
        assert_eq!(manager.count().unwrap(), 1);
    }

    #[test]
    fn test_config_reference_spec() {
        let file_target = ConfigReferenceFileTarget {
            name: "/etc/nginx/nginx.conf".to_string(),
            uid: Some("0".to_string()),
            gid: Some("0".to_string()),
            mode: Some(0o444),
        };

        let ref_spec = ConfigReferenceSpec {
            file: Some(file_target),
            runtime: None,
            config_id: "abc123".to_string(),
            config_name: "nginx-config".to_string(),
        };

        assert_eq!(ref_spec.config_name, "nginx-config");
        assert_eq!(ref_spec.file.as_ref().unwrap().mode, Some(0o444));
    }

    #[test]
    fn test_config_filters_from_json() {
        let json = r#"{"name": ["config1", "config2"], "label": ["env=prod"]}"#;
        let filters = ConfigFilters::from_json(json).unwrap();

        assert_eq!(
            filters.name,
            Some(vec!["config1".to_string(), "config2".to_string()])
        );
        assert_eq!(filters.label, Some(vec!["env=prod".to_string()]));
    }

    #[test]
    fn test_config_templating() {
        use base64::Engine;

        let templating = ConfigTemplating {
            name: Some("golang".to_string()),
            options: HashMap::new(),
        };

        let spec = ConfigSpec {
            name: "templated-config".to_string(),
            labels: HashMap::new(),
            data: base64::engine::general_purpose::STANDARD.encode("{{ .Service.Name }}"),
            templating: Some(templating),
        };

        let config = Config::new(spec);
        assert!(config.spec.templating.is_some());
        assert_eq!(
            config.spec.templating.as_ref().unwrap().name,
            Some("golang".to_string())
        );
    }
}
