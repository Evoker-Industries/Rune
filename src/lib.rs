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
//! - Unix socket daemon (like Docker)
//! - Runefile Language Server Protocol (LSP)
//!
//! ## Custom Container Runtime
//!
//! Rune implements its own container runtime using direct Linux syscalls
//! without external dependencies. The runtime provides:
//!
//! - Linux namespace isolation (PID, NET, MNT, UTS, IPC, USER, CGROUP)
//! - Cgroup v1/v2 resource management
//! - Root filesystem setup with pivot_root
//! - Process execution and management
//!
//! ## Healthcheck Support
//!
//! Containers can define health checks that are executed periodically to
//! determine if the container is healthy. The HEALTHCHECK instruction
//! in Runefile supports:
//!
//! - `--interval`: Time between checks (default: 30s)
//! - `--timeout`: Check timeout (default: 30s)
//! - `--start-period`: Grace period on startup (default: 0s)
//! - `--retries`: Consecutive failures needed (default: 3)

#![recursion_limit = "256"]

pub mod compose;
pub mod container;
pub mod daemon;
pub mod error;
pub mod image;
pub mod lsp;
pub mod network;
pub mod registry;
pub mod runtime;
pub mod storage;
pub mod swarm;
pub mod tui;

pub use error::{Result, RuneError};
