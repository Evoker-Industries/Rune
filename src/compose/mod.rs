//! Docker Compose compatible orchestration
//!
//! This module provides Docker Compose compatibility for multi-container
//! application orchestration.

pub mod config;
pub mod parser;
pub mod orchestrator;

pub use config::{ComposeConfig, ServiceConfig};
pub use parser::ComposeParser;
pub use orchestrator::ComposeOrchestrator;
