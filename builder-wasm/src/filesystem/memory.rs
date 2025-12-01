//! In-memory filesystem for offline/local operation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// File content and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFile {
    pub content: Vec<u8>,
    pub is_dir: bool,
}

/// In-memory filesystem for offline operation
/// This allows the builder to work without any server connection
#[wasm_bindgen]
pub struct InMemoryFilesystem {
    #[wasm_bindgen(skip)]
    pub files: HashMap<String, MemoryFile>,
}

#[wasm_bindgen]
impl InMemoryFilesystem {
    /// Create a new in-memory filesystem
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Write a file to memory
    #[wasm_bindgen(js_name = writeFile)]
    pub fn write_file(&mut self, path: &str, content: &[u8]) {
        let normalized = Self::normalize_path(path);
        self.files.insert(normalized, MemoryFile {
            content: content.to_vec(),
            is_dir: false,
        });
    }

    /// Write a text file to memory
    #[wasm_bindgen(js_name = writeTextFile)]
    pub fn write_text_file(&mut self, path: &str, content: &str) {
        self.write_file(path, content.as_bytes());
    }

    /// Read a file from memory
    #[wasm_bindgen(js_name = readFile)]
    pub fn read_file(&self, path: &str) -> Option<Vec<u8>> {
        let normalized = Self::normalize_path(path);
        self.files.get(&normalized).filter(|f| !f.is_dir).map(|f| f.content.clone())
    }

    /// Read a text file from memory
    #[wasm_bindgen(js_name = readTextFile)]
    pub fn read_text_file(&self, path: &str) -> Option<String> {
        self.read_file(path).and_then(|bytes| String::from_utf8(bytes).ok())
    }

    /// Check if a path exists
    #[wasm_bindgen]
    pub fn exists(&self, path: &str) -> bool {
        let normalized = Self::normalize_path(path);
        self.files.contains_key(&normalized)
    }

    /// Create a directory
    #[wasm_bindgen]
    pub fn mkdir(&mut self, path: &str) {
        let normalized = Self::normalize_path(path);
        self.files.insert(normalized, MemoryFile {
            content: Vec::new(),
            is_dir: true,
        });
    }

    /// Remove a file or directory
    #[wasm_bindgen]
    pub fn remove(&mut self, path: &str) -> bool {
        let normalized = Self::normalize_path(path);
        self.files.remove(&normalized).is_some()
    }

    /// List directory contents
    #[wasm_bindgen(js_name = listDir)]
    pub fn list_dir(&self, path: &str) -> String {
        let normalized = Self::normalize_path(path);
        let prefix = if normalized.ends_with('/') {
            normalized.clone()
        } else {
            format!("{}/", normalized)
        };

        let entries: Vec<serde_json::Value> = self.files
            .keys()
            .filter(|k| k.starts_with(&prefix) && *k != &normalized)
            .filter_map(|k| {
                let relative = &k[prefix.len()..];
                let name = relative.split('/').next()?;
                if name.is_empty() {
                    return None;
                }
                let full_path = format!("{}{}", prefix, name);
                let is_dir = self.files.get(&full_path).map(|f| f.is_dir).unwrap_or(false);
                Some(serde_json::json!({
                    "name": name,
                    "isDir": is_dir
                }))
            })
            .collect();

        serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
    }

    /// Get file size
    #[wasm_bindgen(js_name = getSize)]
    pub fn get_size(&self, path: &str) -> Option<u32> {
        let normalized = Self::normalize_path(path);
        self.files.get(&normalized).map(|f| f.content.len() as u32)
    }

    /// Check if path is a directory
    #[wasm_bindgen(js_name = isDir)]
    pub fn is_dir(&self, path: &str) -> bool {
        let normalized = Self::normalize_path(path);
        self.files.get(&normalized).map(|f| f.is_dir).unwrap_or(false)
    }

    /// Clear all files
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.files.clear();
    }

    /// Get number of files
    #[wasm_bindgen(js_name = fileCount)]
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Export all files as JSON (for debugging/serialization)
    #[wasm_bindgen(js_name = exportAsJson)]
    pub fn export_as_json(&self) -> String {
        let export: HashMap<String, String> = self.files
            .iter()
            .filter(|(_, f)| !f.is_dir)
            .filter_map(|(k, v)| {
                String::from_utf8(v.content.clone()).ok().map(|s| (k.clone(), s))
            })
            .collect();
        serde_json::to_string(&export).unwrap_or_else(|_| "{}".to_string())
    }

    /// Import files from JSON
    #[wasm_bindgen(js_name = importFromJson)]
    pub fn import_from_json(&mut self, json: &str) -> bool {
        match serde_json::from_str::<HashMap<String, String>>(json) {
            Ok(files) => {
                for (path, content) in files {
                    self.write_text_file(&path, &content);
                }
                true
            }
            Err(_) => false,
        }
    }

    fn normalize_path(path: &str) -> String {
        let mut normalized = path.to_string();
        // Remove trailing slash unless root
        while normalized.len() > 1 && normalized.ends_with('/') {
            normalized.pop();
        }
        // Ensure starts with /
        if !normalized.starts_with('/') {
            normalized = format!("/{}", normalized);
        }
        normalized
    }
}

impl Default for InMemoryFilesystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_and_read() {
        let mut fs = InMemoryFilesystem::new();
        fs.write_text_file("/test.txt", "hello world");
        assert_eq!(fs.read_text_file("/test.txt"), Some("hello world".to_string()));
    }

    #[test]
    fn test_exists() {
        let mut fs = InMemoryFilesystem::new();
        assert!(!fs.exists("/test.txt"));
        fs.write_text_file("/test.txt", "content");
        assert!(fs.exists("/test.txt"));
    }

    #[test]
    fn test_remove() {
        let mut fs = InMemoryFilesystem::new();
        fs.write_text_file("/test.txt", "content");
        assert!(fs.remove("/test.txt"));
        assert!(!fs.exists("/test.txt"));
    }
}
