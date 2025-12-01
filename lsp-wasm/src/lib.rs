//! Runefile Language Server Protocol - WebAssembly Version
//!
//! This module provides a WASM-compatible LSP for Runefile editing.
//! It works entirely offline - no server connection required.
//!
//! ## Features
//!
//! - **Code Completion**: Context-aware completions for instructions and arguments
//! - **Hover Documentation**: Detailed docs for all Dockerfile/Runefile instructions
//! - **Diagnostics**: Real-time error and warning detection
//! - **Formatting**: Basic code formatting
//!
//! ## Offline Usage (No Server Required)
//!
//! ```javascript
//! import init, { RunefileLspServer } from 'runefile-lsp-wasm';
//!
//! await init();
//!
//! // Create LSP server (works offline)
//! const lsp = new RunefileLspServer();
//!
//! // Validate content directly (no files needed)
//! const result = lsp.validate('FROM alpine\nRUN echo hello');
//!
//! // Get completions for content
//! const completions = lsp.getCompletionsForContent('FROM alp', 0, 8);
//!
//! // Get hover for content
//! const hover = lsp.getHoverForContent('FROM alpine', 0, 0);
//!
//! // Format content
//! const formatted = lsp.format('from alpine\nrun echo hello');
//!
//! // Or work with documents
//! lsp.openDocument('file:///Runefile', content, 1);
//! const diagnostics = lsp.getDiagnostics('file:///Runefile');
//! ```

pub mod completion;
pub mod hover;
pub mod parser;
pub mod server;

// Re-export main types
pub use completion::CompletionProvider;
pub use hover::HoverProvider;
pub use parser::{types::*, RunefileParser};
pub use server::RunefileLspServer;
