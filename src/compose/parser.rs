//! Docker Compose file parser

use super::config::ComposeConfig;
use crate::error::{Result, RuneError};
use std::path::Path;

/// Default compose file names
pub const DEFAULT_COMPOSE_FILES: &[&str] = &[
    "compose.yaml",
    "compose.yml",
    "docker-compose.yaml",
    "docker-compose.yml",
];

/// Compose file parser
pub struct ComposeParser;

impl ComposeParser {
    /// Find compose file in directory
    pub fn find_compose_file(dir: &Path) -> Option<std::path::PathBuf> {
        for name in DEFAULT_COMPOSE_FILES {
            let path = dir.join(name);
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    /// Parse compose file from path
    pub fn parse_file(path: &Path) -> Result<ComposeConfig> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| RuneError::ComposeParse(format!("Failed to read file: {}", e)))?;

        Self::parse_str(&content)
    }

    /// Parse compose file from string
    pub fn parse_str(content: &str) -> Result<ComposeConfig> {
        // Try YAML first
        serde_yaml::from_str(content)
            .map_err(|e| RuneError::ComposeParse(format!("Failed to parse YAML: {}", e)))
    }

    /// Parse multiple compose files (with merging)
    pub fn parse_files(paths: &[&Path]) -> Result<ComposeConfig> {
        let mut config = ComposeConfig::default();

        for path in paths {
            let file_config = Self::parse_file(path)?;
            config = Self::merge_configs(config, file_config);
        }

        Ok(config)
    }

    /// Merge two compose configurations
    pub fn merge_configs(base: ComposeConfig, overlay: ComposeConfig) -> ComposeConfig {
        let mut result = base;

        // Merge version (overlay wins)
        if overlay.version.is_some() {
            result.version = overlay.version;
        }

        // Merge name (overlay wins)
        if overlay.name.is_some() {
            result.name = overlay.name;
        }

        // Merge services
        for (name, service) in overlay.services {
            if let Some(existing) = result.services.get_mut(&name) {
                // Merge service configs (overlay wins for most fields)
                if service.image.is_some() {
                    existing.image = service.image;
                }
                if service.build.is_some() {
                    existing.build = service.build;
                }
                if service.command.is_some() {
                    existing.command = service.command;
                }
                // Continue for other fields...
            } else {
                result.services.insert(name, service);
            }
        }

        // Merge networks
        for (name, network) in overlay.networks {
            result.networks.insert(name, network);
        }

        // Merge volumes
        for (name, volume) in overlay.volumes {
            result.volumes.insert(name, volume);
        }

        // Merge secrets
        for (name, secret) in overlay.secrets {
            result.secrets.insert(name, secret);
        }

        // Merge configs
        for (name, config) in overlay.configs {
            result.configs.insert(name, config);
        }

        result
    }

    /// Validate compose configuration
    pub fn validate(config: &ComposeConfig) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Validate services
        for (name, service) in &config.services {
            // Service must have either image or build
            if service.image.is_none() && service.build.is_none() {
                return Err(RuneError::ComposeParse(format!(
                    "Service '{}' must have either 'image' or 'build' specified",
                    name
                )));
            }

            // Validate depends_on references
            if let Some(depends) = &service.depends_on {
                let deps = match depends {
                    super::config::DependsOnConfig::Array(arr) => arr.clone(),
                    super::config::DependsOnConfig::Map(map) => map.keys().cloned().collect(),
                };

                for dep in deps {
                    if !config.services.contains_key(&dep) {
                        return Err(RuneError::ComposeParse(format!(
                            "Service '{}' depends on unknown service '{}'",
                            name, dep
                        )));
                    }
                }
            }

            // Validate network references
            if let Some(networks) = &service.networks {
                let nets = match networks {
                    super::config::NetworksConfig::Array(arr) => arr.clone(),
                    super::config::NetworksConfig::Map(map) => map.keys().cloned().collect(),
                };

                for net in nets {
                    if net != "default" && !config.networks.contains_key(&net) {
                        warnings.push(format!(
                            "Service '{}' references undefined network '{}' (will be created)",
                            name, net
                        ));
                    }
                }
            }

            // Validate volume references
            if let Some(volumes) = &service.volumes {
                for vol in volumes {
                    if let super::config::VolumeMount::Long(v) = vol {
                        if v.mount_type.as_deref() == Some("volume") {
                            if let Some(source) = &v.source {
                                if !config.volumes.contains_key(source) {
                                    warnings.push(format!(
                                        "Service '{}' references undefined volume '{}' (will be created)",
                                        name, source
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(warnings)
    }

    /// Interpolate environment variables in config
    pub fn interpolate(
        config: &mut ComposeConfig,
        env: &std::collections::HashMap<String, String>,
    ) {
        // This is a simplified version - full implementation would recursively
        // interpolate all string fields

        for service in config.services.values_mut() {
            // Interpolate image
            if let Some(ref mut image) = service.image {
                *image = interpolate_string(image, env);
            }

            // Interpolate environment
            if let Some(ref mut environment) = service.environment {
                match environment {
                    super::config::EnvironmentConfig::Map(map) => {
                        for value in map.values_mut() {
                            if let Some(v) = value {
                                *v = interpolate_string(v, env);
                            }
                        }
                    }
                    super::config::EnvironmentConfig::Array(arr) => {
                        for item in arr.iter_mut() {
                            *item = interpolate_string(item, env);
                        }
                    }
                }
            }
        }
    }
}

/// Interpolate environment variables in a string
fn interpolate_string(s: &str, env: &std::collections::HashMap<String, String>) -> String {
    let mut result = s.to_string();

    // Handle ${VAR} and $VAR syntax
    for (key, value) in env {
        result = result.replace(&format!("${{{}}}", key), value);
        result = result.replace(&format!("${}", key), value);
    }

    // Handle ${VAR:-default} syntax
    let re = regex::Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*):-([^}]*)\}").unwrap();
    result = re
        .replace_all(&result, |caps: &regex::Captures| {
            let var = &caps[1];
            let default = &caps[2];
            env.get(var).cloned().unwrap_or_else(|| default.to_string())
        })
        .to_string();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_compose() {
        let yaml = r#"
version: "3.8"
services:
  web:
    image: nginx:latest
    ports:
      - "80:80"
  db:
    image: postgres:13
    environment:
      POSTGRES_PASSWORD: secret
"#;

        let config = ComposeParser::parse_str(yaml).unwrap();
        assert_eq!(config.services.len(), 2);
        assert!(config.services.contains_key("web"));
        assert!(config.services.contains_key("db"));
    }

    #[test]
    fn test_validate_missing_image() {
        let yaml = r#"
services:
  web:
    ports:
      - "80:80"
"#;

        let config = ComposeParser::parse_str(yaml).unwrap();
        let result = ComposeParser::validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_interpolate() {
        use std::collections::HashMap;

        let mut env = HashMap::new();
        env.insert("TAG".to_string(), "1.0.0".to_string());

        let s = "nginx:${TAG}";
        let result = interpolate_string(s, &env);
        assert_eq!(result, "nginx:1.0.0");
    }
}
