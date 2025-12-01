//! Local/Offline container management
//!
//! This module provides container management that works without a server connection.
//! It stores container state in memory and can optionally persist to localStorage.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Container state for local storage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalContainer {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub created: String,
    pub command: Vec<String>,
    pub env: Vec<String>,
    pub labels: HashMap<String, String>,
    pub ports: Vec<String>,
    pub volumes: Vec<String>,
}

/// Image state for local storage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalImage {
    pub id: String,
    pub tags: Vec<String>,
    pub size: u64,
    pub created: String,
    pub labels: HashMap<String, String>,
}

/// Local container manager - works entirely offline
#[wasm_bindgen]
pub struct LocalContainerManager {
    #[wasm_bindgen(skip)]
    pub containers: HashMap<String, LocalContainer>,
    #[wasm_bindgen(skip)]
    pub images: HashMap<String, LocalImage>,
    #[wasm_bindgen(skip)]
    pub id_counter: u64,
}

#[wasm_bindgen]
impl LocalContainerManager {
    /// Create a new local container manager
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            containers: HashMap::new(),
            images: HashMap::new(),
            id_counter: 0,
        }
    }

    /// Generate a new container ID with randomness
    fn generate_id(&mut self) -> String {
        self.id_counter += 1;
        let timestamp = js_sys::Date::now() as u64;
        let random = (js_sys::Math::random() * u32::MAX as f64) as u32;
        format!("{:012x}{:04x}{:08x}", timestamp, self.id_counter, random)
    }

    /// Create a container (local only)
    #[wasm_bindgen(js_name = createContainer)]
    pub fn create_container(&mut self, config_json: &str) -> String {
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct CreateConfig {
            image: String,
            name: Option<String>,
            cmd: Option<Vec<String>>,
            env: Option<Vec<String>>,
            labels: Option<HashMap<String, String>>,
        }

        let config: CreateConfig = match serde_json::from_str(config_json) {
            Ok(c) => c,
            Err(e) => return serde_json::json!({ "error": e.to_string() }).to_string(),
        };

        let id = self.generate_id();
        let name = config
            .name
            .unwrap_or_else(|| format!("container_{}", &id[..8]));

        let container = LocalContainer {
            id: id.clone(),
            name: name.clone(),
            image: config.image,
            state: "created".to_string(),
            status: "Created".to_string(),
            created: js_sys::Date::new_0().to_iso_string().into(),
            command: config.cmd.unwrap_or_default(),
            env: config.env.unwrap_or_default(),
            labels: config.labels.unwrap_or_default(),
            ports: Vec::new(),
            volumes: Vec::new(),
        };

        self.containers.insert(id.clone(), container);

        serde_json::json!({
            "Id": id,
            "Name": name
        })
        .to_string()
    }

    /// Start a container (simulated)
    #[wasm_bindgen(js_name = startContainer)]
    pub fn start_container(&mut self, id: &str) -> String {
        if let Some(container) = self.containers.get_mut(id) {
            container.state = "running".to_string();
            container.status = "Up".to_string();
            serde_json::json!({ "success": true }).to_string()
        } else {
            serde_json::json!({ "error": "Container not found" }).to_string()
        }
    }

    /// Stop a container (simulated)
    #[wasm_bindgen(js_name = stopContainer)]
    pub fn stop_container(&mut self, id: &str) -> String {
        if let Some(container) = self.containers.get_mut(id) {
            container.state = "exited".to_string();
            container.status = "Exited (0)".to_string();
            serde_json::json!({ "success": true }).to_string()
        } else {
            serde_json::json!({ "error": "Container not found" }).to_string()
        }
    }

    /// Remove a container
    #[wasm_bindgen(js_name = removeContainer)]
    pub fn remove_container(&mut self, id: &str) -> String {
        if self.containers.remove(id).is_some() {
            serde_json::json!({ "success": true }).to_string()
        } else {
            serde_json::json!({ "error": "Container not found" }).to_string()
        }
    }

    /// List all containers
    #[wasm_bindgen(js_name = listContainers)]
    pub fn list_containers(&self, all: bool) -> String {
        let containers: Vec<&LocalContainer> = self
            .containers
            .values()
            .filter(|c| all || c.state == "running")
            .collect();
        serde_json::to_string(&containers).unwrap_or_else(|_| "[]".to_string())
    }

    /// Get a container by ID
    #[wasm_bindgen(js_name = getContainer)]
    pub fn get_container(&self, id: &str) -> String {
        match self.containers.get(id) {
            Some(c) => serde_json::to_string(c).unwrap_or_else(|_| "null".to_string()),
            None => "null".to_string(),
        }
    }

    /// Add an image (local registry)
    #[wasm_bindgen(js_name = addImage)]
    pub fn add_image(&mut self, id: &str, tags: Vec<String>, size: u64) {
        let image = LocalImage {
            id: id.to_string(),
            tags,
            size,
            created: js_sys::Date::new_0().to_iso_string().into(),
            labels: HashMap::new(),
        };
        self.images.insert(id.to_string(), image);
    }

    /// List all images
    #[wasm_bindgen(js_name = listImages)]
    pub fn list_images(&self) -> String {
        let images: Vec<&LocalImage> = self.images.values().collect();
        serde_json::to_string(&images).unwrap_or_else(|_| "[]".to_string())
    }

    /// Remove an image
    #[wasm_bindgen(js_name = removeImage)]
    pub fn remove_image(&mut self, id: &str) -> String {
        if self.images.remove(id).is_some() {
            serde_json::json!({ "success": true }).to_string()
        } else {
            serde_json::json!({ "error": "Image not found" }).to_string()
        }
    }

    /// Export state as JSON (for persistence)
    #[wasm_bindgen(js_name = exportState)]
    pub fn export_state(&self) -> String {
        serde_json::json!({
            "containers": self.containers,
            "images": self.images,
            "idCounter": self.id_counter
        })
        .to_string()
    }

    /// Import state from JSON (for restoration)
    #[wasm_bindgen(js_name = importState)]
    pub fn import_state(&mut self, json: &str) -> bool {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct State {
            containers: HashMap<String, LocalContainer>,
            images: HashMap<String, LocalImage>,
            id_counter: u64,
        }

        match serde_json::from_str::<State>(json) {
            Ok(state) => {
                self.containers = state.containers;
                self.images = state.images;
                self.id_counter = state.id_counter;
                true
            }
            Err(_) => false,
        }
    }

    /// Save to localStorage (browser only)
    #[wasm_bindgen(js_name = saveToLocalStorage)]
    pub fn save_to_local_storage(&self, key: &str) -> bool {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let state = self.export_state();
                return storage.set_item(key, &state).is_ok();
            }
        }
        false
    }

    /// Load from localStorage (browser only)
    #[wasm_bindgen(js_name = loadFromLocalStorage)]
    pub fn load_from_local_storage(&mut self, key: &str) -> bool {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(state)) = storage.get_item(key) {
                    return self.import_state(&state);
                }
            }
        }
        false
    }

    /// Get container count
    #[wasm_bindgen(js_name = containerCount)]
    pub fn container_count(&self) -> usize {
        self.containers.len()
    }

    /// Get image count
    #[wasm_bindgen(js_name = imageCount)]
    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    /// Clear all state
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.containers.clear();
        self.images.clear();
        self.id_counter = 0;
    }
}

impl Default for LocalContainerManager {
    fn default() -> Self {
        Self::new()
    }
}

// Tests that use js-sys must run in wasm-bindgen-test
// These tests only run in WASM environment
#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_create_container() {
        let mut manager = LocalContainerManager::new();
        let config = r#"{"Image": "alpine", "Name": "test"}"#;
        let result = manager.create_container(config);
        assert!(result.contains("Id"));
    }

    #[wasm_bindgen_test]
    fn test_container_lifecycle() {
        let mut manager = LocalContainerManager::new();
        let config = r#"{"Image": "alpine"}"#;
        let result = manager.create_container(config);
        let id: serde_json::Value = serde_json::from_str(&result).unwrap();
        let container_id = id["Id"].as_str().unwrap();

        manager.start_container(container_id);
        let container = manager.get_container(container_id);
        assert!(container.contains("running"));

        manager.stop_container(container_id);
        let container = manager.get_container(container_id);
        assert!(container.contains("exited"));
    }
}

// Native tests that don't use js-sys
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_container_default() {
        // Test that default works
        let manager = LocalContainerManager::default();
        assert_eq!(manager.container_count(), 0);
    }

    #[test]
    fn test_export_import_state() {
        // Test state serialization (doesn't need js-sys)
        let mut manager = LocalContainerManager::new();
        manager.id_counter = 5;

        // Export state
        let state = manager.export_state();
        assert!(state.contains("idCounter"));

        // Import into new manager
        let mut new_manager = LocalContainerManager::new();
        assert!(new_manager.import_state(&state));
        assert_eq!(new_manager.id_counter, 5);
    }
}
