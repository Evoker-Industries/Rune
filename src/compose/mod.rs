//! Docker Compose compatible orchestration
//!
//! This module provides Docker Compose compatibility for multi-container
//! application orchestration.

pub mod config;
pub mod orchestrator;
pub mod parser;

pub use config::{ComposeConfig, ServiceConfig};
pub use orchestrator::ComposeOrchestrator;
pub use parser::ComposeParser;
