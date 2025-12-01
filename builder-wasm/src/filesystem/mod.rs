//! Filesystem types and interface

mod memory;

pub use memory::InMemoryFilesystem;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// File entry returned by list_dir
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
}

/// File stat result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStat {
    pub size: u64,
    pub is_dir: bool,
    pub mode: u32,
}

/// Filesystem interface for WASM
/// Users implement this via JavaScript callbacks
#[wasm_bindgen]
pub struct BuilderFilesystem {
    #[wasm_bindgen(skip)]
    pub read_file: Option<js_sys::Function>,
    #[wasm_bindgen(skip)]
    pub write_file: Option<js_sys::Function>,
    #[wasm_bindgen(skip)]
    pub list_dir: Option<js_sys::Function>,
    #[wasm_bindgen(skip)]
    pub exists: Option<js_sys::Function>,
    #[wasm_bindgen(skip)]
    pub mkdir: Option<js_sys::Function>,
    #[wasm_bindgen(skip)]
    pub stat: Option<js_sys::Function>,
    #[wasm_bindgen(skip)]
    pub remove: Option<js_sys::Function>,
    #[wasm_bindgen(skip)]
    pub copy: Option<js_sys::Function>,
}

#[wasm_bindgen]
impl BuilderFilesystem {
    /// Create a new filesystem interface
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            read_file: None,
            write_file: None,
            list_dir: None,
            exists: None,
            mkdir: None,
            stat: None,
            remove: None,
            copy: None,
        }
    }

    /// Set the read_file callback: (path: string) => Uint8Array | null
    #[wasm_bindgen(js_name = setReadFile)]
    pub fn set_read_file(&mut self, callback: js_sys::Function) {
        self.read_file = Some(callback);
    }

    /// Set the write_file callback: (path: string, contents: Uint8Array) => void
    #[wasm_bindgen(js_name = setWriteFile)]
    pub fn set_write_file(&mut self, callback: js_sys::Function) {
        self.write_file = Some(callback);
    }

    /// Set the list_dir callback: (path: string) => Array<{name: string, isDir: boolean}>
    #[wasm_bindgen(js_name = setListDir)]
    pub fn set_list_dir(&mut self, callback: js_sys::Function) {
        self.list_dir = Some(callback);
    }

    /// Set the exists callback: (path: string) => boolean
    #[wasm_bindgen(js_name = setExists)]
    pub fn set_exists(&mut self, callback: js_sys::Function) {
        self.exists = Some(callback);
    }

    /// Set the mkdir callback: (path: string) => void
    #[wasm_bindgen(js_name = setMkdir)]
    pub fn set_mkdir(&mut self, callback: js_sys::Function) {
        self.mkdir = Some(callback);
    }

    /// Set the stat callback: (path: string) => {size: number, isDir: boolean, mode: number} | null
    #[wasm_bindgen(js_name = setStat)]
    pub fn set_stat(&mut self, callback: js_sys::Function) {
        self.stat = Some(callback);
    }

    /// Set the remove callback: (path: string) => void
    #[wasm_bindgen(js_name = setRemove)]
    pub fn set_remove(&mut self, callback: js_sys::Function) {
        self.remove = Some(callback);
    }

    /// Set the copy callback: (src: string, dest: string) => void
    #[wasm_bindgen(js_name = setCopy)]
    pub fn set_copy(&mut self, callback: js_sys::Function) {
        self.copy = Some(callback);
    }
}

impl Default for BuilderFilesystem {
    fn default() -> Self {
        Self::new()
    }
}

impl BuilderFilesystem {
    /// Read a file from the filesystem
    pub fn read_file_impl(&self, path: &str) -> Option<Vec<u8>> {
        let callback = self.read_file.as_ref()?;
        let this = JsValue::null();
        let arg = JsValue::from_str(path);
        
        match callback.call1(&this, &arg) {
            Ok(result) => {
                if result.is_null() || result.is_undefined() {
                    None
                } else if let Some(array) = result.dyn_ref::<js_sys::Uint8Array>() {
                    Some(array.to_vec())
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Write a file to the filesystem
    pub fn write_file_impl(&self, path: &str, contents: &[u8]) -> bool {
        let callback = match &self.write_file {
            Some(cb) => cb,
            None => return false,
        };
        
        let this = JsValue::null();
        let path_arg = JsValue::from_str(path);
        let contents_arg = js_sys::Uint8Array::from(contents);
        
        callback.call2(&this, &path_arg, &contents_arg).is_ok()
    }

    /// List directory contents
    pub fn list_dir_impl(&self, path: &str) -> Option<Vec<FileEntry>> {
        let callback = self.list_dir.as_ref()?;
        let this = JsValue::null();
        let arg = JsValue::from_str(path);
        
        match callback.call1(&this, &arg) {
            Ok(result) => {
                if result.is_null() || result.is_undefined() {
                    None
                } else {
                    serde_wasm_bindgen::from_value(result).ok()
                }
            }
            Err(_) => None,
        }
    }

    /// Check if a path exists
    pub fn exists_impl(&self, path: &str) -> bool {
        let callback = match &self.exists {
            Some(cb) => cb,
            None => return false,
        };
        
        let this = JsValue::null();
        let arg = JsValue::from_str(path);
        
        match callback.call1(&this, &arg) {
            Ok(result) => result.as_bool().unwrap_or(false),
            Err(_) => false,
        }
    }

    /// Create a directory
    pub fn mkdir_impl(&self, path: &str) -> bool {
        let callback = match &self.mkdir {
            Some(cb) => cb,
            None => return false,
        };
        
        let this = JsValue::null();
        let arg = JsValue::from_str(path);
        
        callback.call1(&this, &arg).is_ok()
    }

    /// Get file stats
    pub fn stat_impl(&self, path: &str) -> Option<FileStat> {
        let callback = self.stat.as_ref()?;
        let this = JsValue::null();
        let arg = JsValue::from_str(path);
        
        match callback.call1(&this, &arg) {
            Ok(result) => {
                if result.is_null() || result.is_undefined() {
                    None
                } else {
                    serde_wasm_bindgen::from_value(result).ok()
                }
            }
            Err(_) => None,
        }
    }

    /// Remove a file or directory
    pub fn remove_impl(&self, path: &str) -> bool {
        let callback = match &self.remove {
            Some(cb) => cb,
            None => return false,
        };
        
        let this = JsValue::null();
        let arg = JsValue::from_str(path);
        
        callback.call1(&this, &arg).is_ok()
    }

    /// Copy a file
    pub fn copy_impl(&self, src: &str, dest: &str) -> bool {
        let callback = match &self.copy {
            Some(cb) => cb,
            None => return false,
        };
        
        let this = JsValue::null();
        let src_arg = JsValue::from_str(src);
        let dest_arg = JsValue::from_str(dest);
        
        callback.call2(&this, &src_arg, &dest_arg).is_ok()
    }
}
