//! Image management module
//!
//! This module provides functionality for managing container images,
//! including pulling, building, and storing images.

pub mod builder;
pub mod registry;
pub mod store;

pub use builder::{BuildContext, ImageBuilder};
pub use registry::Registry;
pub use store::{Image, ImageStore};
