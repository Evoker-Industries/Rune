//! Network management module
//!
//! This module provides networking functionality for containers.

pub mod bridge;
pub mod config;

pub use bridge::BridgeNetwork;
pub use config::{NetworkConfig, NetworkDriver};
