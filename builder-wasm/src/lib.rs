//! Runefile Builder - WebAssembly Version with Bring-Your-Own-Filesystem
//!
//! This module provides a WASM-compatible image builder for Runefiles that allows
//! users to provide their own filesystem implementation via JavaScript callbacks.
//!
//! ## Offline/Local Mode
//!
//! This module works entirely offline - no server connection required. All operations
//! use the bring-your-own-filesystem (BYOFS) interface or in-memory storage.
//!
//! ## Usage with In-Memory Filesystem (No Server Required)
//!
//! ```javascript
//! import init, { WasmBuilder, InMemoryFilesystem } from 'runefile-builder-wasm';
//!
//! // Create an in-memory filesystem (works completely offline)
//! const memFs = new InMemoryFilesystem();
//! memFs.writeTextFile('/project/Runefile', 'FROM alpine\nRUN echo hello');
//! memFs.writeTextFile('/project/app.js', 'console.log("hello")');
//!
//! // Create filesystem adapter from in-memory fs
//! const fs = new BuilderFilesystem();
//! fs.setReadFile((path) => memFs.readFile(path));
//! fs.setExists((path) => memFs.exists(path));
//!
//! // Create the builder and build (all local, no network)
//! const builder = new WasmBuilder(fs);
//! const result = builder.build(JSON.stringify({
//!     contextDir: '/project',
//!     tags: ['myapp:latest'],
//! }));
//! ```
//!
//! ## Usage with Custom Filesystem (Browser File API, etc.)
//!
//! ```javascript
//! const fs = new BuilderFilesystem();
//! fs.setReadFile((path) => myFilesystem.readFile(path));
//! fs.setWriteFile((path, contents) => myFilesystem.writeFile(path, contents));
//! fs.setListDir((path) => myFilesystem.listDir(path));
//! fs.setExists((path) => myFilesystem.exists(path));
//!
//! const builder = new WasmBuilder(fs);
//! const parsed = builder.parseRunefile(runefileContent);
//! ```

pub mod builder;
pub mod filesystem;
pub mod parser;
pub mod types;

// Re-export main types
pub use builder::WasmBuilder;
pub use filesystem::{BuilderFilesystem, InMemoryFilesystem};
pub use parser::RunefileParser;
pub use types::*;

use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;

/// Calculate SHA-256 digest (works offline)
#[wasm_bindgen(js_name = calculateDigest)]
pub fn calculate_digest(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    format!("sha256:{}", hex::encode(result))
}

/// Generate a simple ID (works offline, no UUID dependency needed)
#[wasm_bindgen(js_name = generateId)]
pub fn generate_id() -> String {
    let timestamp = js_sys::Date::now() as u64;
    let random = js_sys::Math::random();
    format!(
        "{:016x}{:08x}",
        timestamp,
        (random * u32::MAX as f64) as u32
    )
}

/// Get current timestamp as ISO string (works offline)
#[wasm_bindgen(js_name = getCurrentTimestamp)]
pub fn get_current_timestamp() -> String {
    js_sys::Date::new_0().to_iso_string().into()
}
