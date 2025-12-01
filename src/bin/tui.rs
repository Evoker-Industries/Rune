//! Rune TUI - Standalone Terminal User Interface
//!
//! Launch the TUI directly with: rune-tui

use rune::container::ContainerManager;
use rune::error::Result;
use rune::tui::App;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("warn"))
        .init();

    // Get base path for rune data
    let base_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/var/lib"))
        .join("rune");

    // Initialize container manager
    let container_manager = Arc::new(ContainerManager::new(base_path.join("containers"))?);

    // Run TUI
    let mut app = App::new(container_manager);
    app.run()
}
