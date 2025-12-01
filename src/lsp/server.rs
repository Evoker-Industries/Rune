//! Runefile LSP Server Implementation

use super::completion::CompletionProvider;
use super::diagnostics::DiagnosticsProvider;
use super::hover::HoverProvider;
use super::syntax::{InstructionKind, RunefileParser};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// LSP message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
#[allow(dead_code)]
pub enum LspMessage {
    #[serde(rename = "initialize")]
    Initialize { id: i64, params: InitializeParams },

    #[serde(rename = "initialized")]
    Initialized,

    #[serde(rename = "shutdown")]
    Shutdown { id: i64 },

    #[serde(rename = "exit")]
    Exit,

    #[serde(rename = "textDocument/didOpen")]
    DidOpen { params: DidOpenParams },

    #[serde(rename = "textDocument/didChange")]
    DidChange { params: DidChangeParams },

    #[serde(rename = "textDocument/didClose")]
    DidClose { params: DidCloseParams },

    #[serde(rename = "textDocument/didSave")]
    DidSave { params: DidSaveParams },

    #[serde(rename = "textDocument/completion")]
    Completion { id: i64, params: CompletionParams },

    #[serde(rename = "textDocument/hover")]
    Hover { id: i64, params: HoverParams },

    #[serde(rename = "textDocument/definition")]
    Definition { id: i64, params: DefinitionParams },

    #[serde(rename = "textDocument/formatting")]
    Formatting { id: i64, params: FormattingParams },
}

/// Initialize request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub process_id: Option<i64>,
    pub root_uri: Option<String>,
    pub capabilities: ClientCapabilities,
}

/// Client capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    pub text_document: Option<TextDocumentClientCapabilities>,
}

/// Text document capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentClientCapabilities {
    pub completion: Option<CompletionClientCapabilities>,
    pub hover: Option<HoverClientCapabilities>,
}

/// Completion capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionClientCapabilities {
    pub snippet_support: Option<bool>,
}

/// Hover capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HoverClientCapabilities {}

/// Document open params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidOpenParams {
    pub text_document: TextDocumentItem,
}

/// Document change params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidChangeParams {
    pub text_document: VersionedTextDocumentIdentifier,
    pub content_changes: Vec<TextDocumentContentChangeEvent>,
}

/// Document close params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidCloseParams {
    pub text_document: TextDocumentIdentifier,
}

/// Document save params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct DidSaveParams {
    pub text_document: TextDocumentIdentifier,
    pub text: Option<String>,
}

/// Completion params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionParams {
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

/// Hover params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoverParams {
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

/// Definition params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionParams {
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

/// Formatting params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormattingParams {
    pub text_document: TextDocumentIdentifier,
    pub options: FormattingOptions,
}

/// Formatting options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormattingOptions {
    pub tab_size: u32,
    pub insert_spaces: bool,
}

/// Text document item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentItem {
    pub uri: String,
    pub language_id: String,
    pub version: i64,
    pub text: String,
}

/// Text document identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentIdentifier {
    pub uri: String,
}

/// Versioned text document identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedTextDocumentIdentifier {
    pub uri: String,
    pub version: i64,
}

/// Text document change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentContentChangeEvent {
    pub text: String,
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

/// Location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    pub text_document_sync: TextDocumentSyncOptions,
    pub completion_provider: Option<CompletionOptions>,
    pub hover_provider: Option<bool>,
    pub definition_provider: Option<bool>,
    pub document_formatting_provider: Option<bool>,
}

/// Text document sync options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentSyncOptions {
    pub open_close: bool,
    pub change: u8, // 1 = Full, 2 = Incremental
    pub save: Option<SaveOptions>,
}

/// Save options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveOptions {
    pub include_text: bool,
}

/// Completion options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionOptions {
    pub trigger_characters: Vec<String>,
    pub resolve_provider: bool,
}

/// Initialize result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    pub capabilities: ServerCapabilities,
}

/// Completion item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionItem {
    pub label: String,
    pub kind: Option<u8>,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
    pub insert_text_format: Option<u8>,
}

/// Hover result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hover {
    pub contents: MarkupContent,
    pub range: Option<Range>,
}

/// Markup content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkupContent {
    pub kind: String,
    pub value: String,
}

/// Diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<u8>,
    pub code: Option<String>,
    pub source: Option<String>,
    pub message: String,
}

/// Publish diagnostics params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PublishDiagnosticsParams {
    pub uri: String,
    pub diagnostics: Vec<Diagnostic>,
}

/// Text edit
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

/// Document state
#[allow(dead_code)]
struct DocumentState {
    content: String,
    version: i64,
    parser: RunefileParser,
}

/// Runefile Language Server
pub struct RunefileLanguageServer {
    documents: Arc<RwLock<HashMap<String, DocumentState>>>,
    completion_provider: CompletionProvider,
    hover_provider: HoverProvider,
    diagnostics_provider: DiagnosticsProvider,
    snippet_support: bool,
}

impl RunefileLanguageServer {
    /// Create a new language server
    pub fn new() -> Self {
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            completion_provider: CompletionProvider::new(),
            hover_provider: HoverProvider::new(),
            diagnostics_provider: DiagnosticsProvider::new(),
            snippet_support: false,
        }
    }

    /// Handle initialize request
    pub fn initialize(&mut self, params: &InitializeParams) -> InitializeResult {
        // Check for snippet support
        if let Some(ref td) = params.capabilities.text_document {
            if let Some(ref completion) = td.completion {
                self.snippet_support = completion.snippet_support.unwrap_or(false);
            }
        }

        InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: TextDocumentSyncOptions {
                    open_close: true,
                    change: 1, // Full sync
                    save: Some(SaveOptions { include_text: true }),
                },
                completion_provider: Some(CompletionOptions {
                    trigger_characters: vec![" ".to_string(), "-".to_string(), "=".to_string()],
                    resolve_provider: false,
                }),
                hover_provider: Some(true),
                definition_provider: Some(true),
                document_formatting_provider: Some(true),
            },
        }
    }

    /// Handle document open
    pub fn did_open(&self, params: &DidOpenParams) -> Vec<Diagnostic> {
        let mut parser = RunefileParser::new();
        parser.parse(&params.text_document.text);

        let diagnostics = self.diagnostics_provider.get_diagnostics(&parser);

        let mut docs = self.documents.write().unwrap();
        docs.insert(
            params.text_document.uri.clone(),
            DocumentState {
                content: params.text_document.text.clone(),
                version: params.text_document.version,
                parser,
            },
        );

        diagnostics
    }

    /// Handle document change
    pub fn did_change(&self, params: &DidChangeParams) -> Vec<Diagnostic> {
        if let Some(change) = params.content_changes.first() {
            let mut parser = RunefileParser::new();
            parser.parse(&change.text);

            let diagnostics = self.diagnostics_provider.get_diagnostics(&parser);

            let mut docs = self.documents.write().unwrap();
            docs.insert(
                params.text_document.uri.clone(),
                DocumentState {
                    content: change.text.clone(),
                    version: params.text_document.version,
                    parser,
                },
            );

            return diagnostics;
        }

        Vec::new()
    }

    /// Handle document close
    pub fn did_close(&self, params: &DidCloseParams) {
        let mut docs = self.documents.write().unwrap();
        docs.remove(&params.text_document.uri);
    }

    /// Handle completion request
    pub fn completion(&self, params: &CompletionParams) -> Vec<CompletionItem> {
        let docs = self.documents.read().unwrap();

        if let Some(doc) = docs.get(&params.text_document.uri) {
            return self.completion_provider.get_completions(
                &doc.content,
                &doc.parser,
                params.position.line as usize,
                params.position.character as usize,
                self.snippet_support,
            );
        }

        Vec::new()
    }

    /// Handle hover request
    pub fn hover(&self, params: &HoverParams) -> Option<Hover> {
        let docs = self.documents.read().unwrap();

        if let Some(doc) = docs.get(&params.text_document.uri) {
            return self.hover_provider.get_hover(
                &doc.content,
                &doc.parser,
                params.position.line as usize,
                params.position.character as usize,
            );
        }

        None
    }

    /// Handle definition request
    pub fn definition(&self, params: &DefinitionParams) -> Option<Location> {
        let docs = self.documents.read().unwrap();

        if let Some(doc) = docs.get(&params.text_document.uri) {
            // Find definitions for COPY --from=stage or variable references
            let line = params.position.line as usize;
            let col = params.position.character as usize;

            if let Some(inst) = doc.parser.instruction_at(line, col) {
                // Check for --from=stage in COPY
                if inst.kind == InstructionKind::Copy {
                    if let Some(from_match) = inst.arguments.find("--from=") {
                        let stage_start = from_match + 7;
                        let stage_end = inst.arguments[stage_start..]
                            .find(char::is_whitespace)
                            .map(|i| stage_start + i)
                            .unwrap_or(inst.arguments.len());
                        let stage_name = &inst.arguments[stage_start..stage_end];

                        // Find the FROM instruction that defines this stage
                        for i in &doc.parser.instructions {
                            if i.kind == InstructionKind::From
                                && (i.arguments.contains(&format!("AS {}", stage_name))
                                    || i.arguments.contains(&format!("as {}", stage_name)))
                            {
                                return Some(Location {
                                    uri: params.text_document.uri.clone(),
                                    range: Range {
                                        start: Position {
                                            line: i.line as u32,
                                            character: 0,
                                        },
                                        end: Position {
                                            line: i.line as u32,
                                            character: i.raw.len() as u32,
                                        },
                                    },
                                });
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Handle formatting request
    pub fn formatting(&self, params: &FormattingParams) -> Vec<TextEdit> {
        let docs = self.documents.read().unwrap();

        if let Some(doc) = docs.get(&params.text_document.uri) {
            return self.format_document(&doc.content, &params.options);
        }

        Vec::new()
    }

    /// Format a document
    fn format_document(&self, content: &str, _options: &FormattingOptions) -> Vec<TextEdit> {
        let mut edits = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Format instruction lines
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
                if !parts.is_empty() {
                    let keyword = parts[0].to_uppercase();
                    let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

                    let formatted = if args.is_empty() {
                        keyword
                    } else {
                        format!("{} {}", keyword, args)
                    };

                    if *line != formatted {
                        edits.push(TextEdit {
                            range: Range {
                                start: Position {
                                    line: i as u32,
                                    character: 0,
                                },
                                end: Position {
                                    line: i as u32,
                                    character: line.len() as u32,
                                },
                            },
                            new_text: formatted,
                        });
                    }
                }
            }
        }

        edits
    }
}

impl Default for RunefileLanguageServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_initialization() {
        let mut server = RunefileLanguageServer::new();
        let params = InitializeParams {
            process_id: Some(1234),
            root_uri: Some("file:///test".to_string()),
            capabilities: ClientCapabilities::default(),
        };

        let result = server.initialize(&params);
        assert!(result.capabilities.hover_provider.unwrap());
        assert!(result.capabilities.completion_provider.is_some());
    }

    #[test]
    fn test_document_open() {
        let server = RunefileLanguageServer::new();
        let params = DidOpenParams {
            text_document: TextDocumentItem {
                uri: "file:///test/Runefile".to_string(),
                language_id: "runefile".to_string(),
                version: 1,
                text: "FROM alpine\nRUN echo hello".to_string(),
            },
        };

        let diagnostics = server.did_open(&params);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_document_with_errors() {
        let server = RunefileLanguageServer::new();
        let params = DidOpenParams {
            text_document: TextDocumentItem {
                uri: "file:///test/Runefile".to_string(),
                language_id: "runefile".to_string(),
                version: 1,
                text: "RUN echo hello".to_string(), // Missing FROM
            },
        };

        let diagnostics = server.did_open(&params);
        assert!(!diagnostics.is_empty());
    }
}
