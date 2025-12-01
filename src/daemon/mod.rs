//! Rune Daemon - Unix Socket Server
//!
//! This module implements a Docker-like daemon that listens on a Unix socket
//! at `/var/run/rune.sock` and provides a REST API for container management.

mod server;
mod api;

pub use server::RuneDaemon;
pub use api::ApiHandler;
