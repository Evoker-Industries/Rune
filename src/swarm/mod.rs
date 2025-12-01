//! Docker Swarm compatible cluster orchestration
//!
//! This module provides Docker Swarm compatibility for cluster
//! management and service orchestration.

pub mod cluster;
pub mod config;
pub mod node;
pub mod service;
pub mod task;

pub use cluster::{SwarmCluster, SwarmConfig};
pub use config::{Config, ConfigManager, ConfigSpec};
pub use node::{Node, NodeRole, NodeState};
pub use service::{Service, ServiceSpec};
pub use task::{Task, TaskState};
