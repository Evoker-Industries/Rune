//! OCI-Compatible Container Registry Server
//!
//! This module implements an OCI Distribution Specification compliant registry
//! that is compatible with Docker, Podman, and other OCI-compliant tools.

pub mod server;
pub mod storage;
pub mod auth;

pub use server::RegistryServer;
pub use storage::RegistryStorage;
pub use auth::RegistryAuth;
