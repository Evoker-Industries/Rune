//! LSP types for Runefile

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Runefile instruction types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[wasm_bindgen]
pub enum InstructionKind {
    From,
    Run,
    Copy,
    Add,
    Cmd,
    Entrypoint,
    Env,
    Expose,
    Label,
    Maintainer,
    Volume,
    Workdir,
    Arg,
    User,
    Healthcheck,
    Shell,
    Stopsignal,
    Onbuild,
    Comment,
    Unknown,
}

/// Parsed instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    pub kind: InstructionKind,
    pub line: usize,
    pub raw: String,
    pub keyword: String,
    pub arguments: String,
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[wasm_bindgen]
pub enum ErrorSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// Parser error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
    pub severity: ErrorSeverity,
}

/// Position in a document
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Range in a document
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Diagnostic item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: u8,
    pub message: String,
    pub source: String,
}

/// Completion item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionItem {
    pub label: String,
    pub kind: u8,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
    pub insert_text_format: Option<u8>,
}

/// Hover result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverResult {
    pub contents: String,
    pub range: Option<Range>,
}
