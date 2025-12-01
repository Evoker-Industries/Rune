//! Container management module
//!
//! This module provides core functionality for managing containers,
//! including creation, lifecycle management, and resource isolation.

pub mod config;
pub mod lifecycle;
pub mod runtime;

pub use config::{
    ContainerConfig, ContainerStatus, PortMapping, Protocol, ResourceLimits, VolumeMount,
};
pub use lifecycle::ContainerManager;
pub use runtime::Container;
