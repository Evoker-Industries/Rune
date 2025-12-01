//! Runefile Language Server Protocol Implementation
//!
//! Provides IDE support for Runefile editing including:
//! - Syntax highlighting
//! - Auto-completion
//! - Hover documentation
//! - Diagnostics (linting)
//! - Go to definition
//! - Document formatting

mod completion;
mod diagnostics;
mod hover;
mod server;
mod syntax;

pub use server::RunefileLanguageServer;
pub use syntax::{Instruction, InstructionKind, RunefileParser};
