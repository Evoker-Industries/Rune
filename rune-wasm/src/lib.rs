//! Rune WASM - Container Runtime for WebAssembly
//!
//! This module provides a browser-compatible version of the Rune container runtime.
//! It can operate in two modes:
//!
//! 1. **Remote Mode**: Connect to a Rune/Docker daemon via WebSocket
//! 2. **Local/Offline Mode**: Manage containers entirely in-memory (no server required)
//!
//! ## Local/Offline Usage (No Server Required)
//!
//! ```javascript
//! import init, { LocalContainerManager, RunefileBuilder } from 'rune-wasm';
//!
//! await init();
//!
//! // Create a local container manager (works offline)
//! const manager = new LocalContainerManager();
//!
//! // Create and manage containers locally
//! const result = manager.createContainer(JSON.stringify({
//!     Image: 'alpine',
//!     Name: 'my-container'
//! }));
//!
//! // Start/stop containers (simulated locally)
//! manager.startContainer(containerId);
//! manager.stopContainer(containerId);
//!
//! // Persist to localStorage (browser only)
//! manager.saveToLocalStorage('rune-containers');
//!
//! // Restore from localStorage
//! manager.loadFromLocalStorage('rune-containers');
//! ```
//!
//! ## Remote Usage (With Server)
//!
//! ```javascript
//! import init, { RuneClient } from 'rune-wasm';
//!
//! await init();
//!
//! const client = new RuneClient('ws://localhost:2375');
//! await client.connect();
//! const containers = await client.listContainers();
//! ```

pub mod builder;
pub mod client;
pub mod compose;
pub mod types;
pub mod utils;

// Re-export main types for convenience
pub use builder::RunefileBuilder;
pub use client::{LocalContainerManager, RuneClient};
pub use compose::ComposeParser;
pub use types::*;
pub use utils::{calculate_digest, generate_id, get_current_timestamp};
