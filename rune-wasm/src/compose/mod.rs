//! Compose file parser for WASM

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Compose service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeService {
    pub name: String,
    pub image: Option<String>,
    pub build: Option<ComposeBuild>,
    pub command: Option<Vec<String>>,
    pub environment: Option<HashMap<String, String>>,
    pub ports: Option<Vec<String>>,
    pub volumes: Option<Vec<String>>,
    pub depends_on: Option<Vec<String>>,
    pub networks: Option<Vec<String>>,
    pub labels: Option<HashMap<String, String>>,
    pub restart: Option<String>,
}

/// Compose build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeBuild {
    pub context: String,
    pub dockerfile: Option<String>,
    pub args: Option<HashMap<String, String>>,
}

/// Parsed compose file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCompose {
    pub version: Option<String>,
    pub services: HashMap<String, ComposeService>,
    pub networks: Option<HashMap<String, serde_json::Value>>,
    pub volumes: Option<HashMap<String, serde_json::Value>>,
}

/// Compose file parser
#[wasm_bindgen]
pub struct ComposeParser;

#[wasm_bindgen]
impl ComposeParser {
    /// Create a new compose parser
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self
    }

    /// Parse a compose file (YAML as JSON)
    #[wasm_bindgen]
    pub fn parse(&self, json_content: &str) -> String {
        match serde_json::from_str::<ParsedCompose>(json_content) {
            Ok(compose) => serde_json::to_string(&compose).unwrap_or_default(),
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string(),
        }
    }

    /// Get the start order for services based on depends_on
    #[wasm_bindgen(js_name = getStartOrder)]
    pub fn get_start_order(&self, json_content: &str) -> String {
        match serde_json::from_str::<ParsedCompose>(json_content) {
            Ok(compose) => {
                let mut order = Vec::new();
                let mut visited = std::collections::HashSet::new();

                fn visit(
                    name: &str,
                    services: &HashMap<String, ComposeService>,
                    visited: &mut std::collections::HashSet<String>,
                    order: &mut Vec<String>,
                ) {
                    if visited.contains(name) {
                        return;
                    }
                    visited.insert(name.to_string());

                    if let Some(service) = services.get(name) {
                        if let Some(deps) = &service.depends_on {
                            for dep in deps {
                                visit(dep, services, visited, order);
                            }
                        }
                    }
                    order.push(name.to_string());
                }

                for name in compose.services.keys() {
                    visit(name, &compose.services, &mut visited, &mut order);
                }

                serde_json::to_string(&order).unwrap_or_default()
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string(),
        }
    }

    /// Validate a compose file
    #[wasm_bindgen]
    pub fn validate(&self, json_content: &str) -> String {
        let mut errors = Vec::new();
        let warnings: Vec<String> = Vec::new();

        match serde_json::from_str::<ParsedCompose>(json_content) {
            Ok(compose) => {
                for (name, service) in &compose.services {
                    if service.image.is_none() && service.build.is_none() {
                        errors.push(format!("Service '{}' has no image or build", name));
                    }

                    if let Some(deps) = &service.depends_on {
                        for dep in deps {
                            if !compose.services.contains_key(dep) {
                                errors.push(format!(
                                    "Service '{}' depends on unknown service '{}'",
                                    name, dep
                                ));
                            }
                        }
                    }
                }
            }
            Err(e) => errors.push(e.to_string()),
        }

        serde_json::json!({
            "valid": errors.is_empty(),
            "errors": errors,
            "warnings": warnings
        })
        .to_string()
    }
}

impl Default for ComposeParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compose_parser_valid() {
        let parser = ComposeParser::new();
        // Use a properly formatted JSON that matches our struct
        let json = r#"{"services":{"web":{"name":"web","image":"nginx"}}}"#;
        let result = parser.parse(json);
        // The parse should succeed
        assert!(result.contains("web") || result.contains("nginx"));
    }

    #[test]
    fn test_compose_validation() {
        let parser = ComposeParser::new();
        // Service without image or build should fail validation
        let json = r#"{"services":{"web":{"name":"web"}}}"#;
        let result = parser.validate(json);
        assert!(result.contains("no image or build"));
    }
}
