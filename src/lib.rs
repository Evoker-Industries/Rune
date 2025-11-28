//! Rune - A Docker-like and Docker-compatible container service
//!
//! Rune is a container runtime and orchestration platform written in Rust.
//! It provides Docker-compatible APIs and commands, along with support for:
//!
//! - Container lifecycle management
//! - Image building (from Runefile or Dockerfile)
//! - Docker Compose compatibility
//! - Docker Swarm compatibility
//! - OCI-compatible container registry
//! - Terminal User Interface (TUI)

pub mod container;
pub mod compose;
pub mod error;
pub mod image;
pub mod network;
pub mod registry;
pub mod storage;
pub mod swarm;
pub mod tui;

pub use error::{Result, RuneError};
