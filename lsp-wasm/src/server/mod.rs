//! LSP Server for Runefile - works entirely offline

use crate::completion::CompletionProvider;
use crate::hover::HoverProvider;
use crate::parser::RunefileParser;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Document stored in the server
#[derive(Debug, Clone)]
struct Document {
    content: String,
    version: i32,
}

/// Runefile LSP Server - works entirely offline with local files
#[wasm_bindgen]
pub struct RunefileLspServer {
    #[wasm_bindgen(skip)]
    documents: HashMap<String, Document>,
    #[wasm_bindgen(skip)]
    parser: RunefileParser,
    #[wasm_bindgen(skip)]
    completion: CompletionProvider,
    #[wasm_bindgen(skip)]
    hover: HoverProvider,
}

#[wasm_bindgen]
impl RunefileLspServer {
    /// Create a new LSP server (works offline)
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            parser: RunefileParser::new(),
            completion: CompletionProvider::new(),
            hover: HoverProvider::new(),
        }
    }

    /// Open a document
    #[wasm_bindgen(js_name = openDocument)]
    pub fn open_document(&mut self, uri: &str, content: &str, version: i32) {
        self.documents.insert(uri.to_string(), Document {
            content: content.to_string(),
            version,
        });
    }

    /// Update a document
    #[wasm_bindgen(js_name = updateDocument)]
    pub fn update_document(&mut self, uri: &str, content: &str, version: i32) {
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.content = content.to_string();
            doc.version = version;
        } else {
            self.open_document(uri, content, version);
        }
    }

    /// Close a document
    #[wasm_bindgen(js_name = closeDocument)]
    pub fn close_document(&mut self, uri: &str) {
        self.documents.remove(uri);
    }

    /// Get document content
    #[wasm_bindgen(js_name = getDocumentContent)]
    pub fn get_document_content(&self, uri: &str) -> Option<String> {
        self.documents.get(uri).map(|d| d.content.clone())
    }

    /// Get diagnostics for a document (works offline)
    #[wasm_bindgen(js_name = getDiagnostics)]
    pub fn get_diagnostics(&mut self, uri: &str) -> String {
        if let Some(doc) = self.documents.get(uri) {
            self.parser.parse(&doc.content);
            self.parser.get_diagnostics_json()
        } else {
            "[]".to_string()
        }
    }

    /// Get diagnostics for content directly (works offline)
    #[wasm_bindgen(js_name = getDiagnosticsForContent)]
    pub fn get_diagnostics_for_content(&mut self, content: &str) -> String {
        self.parser.parse(content);
        self.parser.get_diagnostics_json()
    }

    /// Get completions at position (works offline)
    #[wasm_bindgen(js_name = getCompletions)]
    pub fn get_completions(&self, uri: &str, line: u32, character: u32) -> String {
        if let Some(doc) = self.documents.get(uri) {
            self.completion.get_completions(&doc.content, line, character)
        } else {
            "[]".to_string()
        }
    }

    /// Get completions for content directly (works offline)
    #[wasm_bindgen(js_name = getCompletionsForContent)]
    pub fn get_completions_for_content(&self, content: &str, line: u32, character: u32) -> String {
        self.completion.get_completions(content, line, character)
    }

    /// Get hover information (works offline)
    #[wasm_bindgen(js_name = getHover)]
    pub fn get_hover(&self, uri: &str, line: u32, character: u32) -> String {
        if let Some(doc) = self.documents.get(uri) {
            self.hover.get_hover(&doc.content, line, character)
        } else {
            "null".to_string()
        }
    }

    /// Get hover for content directly (works offline)
    #[wasm_bindgen(js_name = getHoverForContent)]
    pub fn get_hover_for_content(&self, content: &str, line: u32, character: u32) -> String {
        self.hover.get_hover(content, line, character)
    }

    /// Validate content (works offline)
    #[wasm_bindgen]
    pub fn validate(&mut self, content: &str) -> String {
        self.parser.parse(content);
        
        let errors = self.parser.error_count();
        let instructions = self.parser.instruction_count();
        
        serde_json::json!({
            "valid": errors == 0,
            "errorCount": errors,
            "instructionCount": instructions,
            "diagnostics": serde_json::from_str::<serde_json::Value>(&self.parser.get_diagnostics_json()).unwrap_or(serde_json::json!([]))
        }).to_string()
    }

    /// Format a Runefile (basic formatting, works offline)
    #[wasm_bindgen]
    pub fn format(&self, content: &str) -> String {
        let mut result = Vec::new();
        let mut prev_was_empty = false;

        for line in content.lines() {
            let trimmed = line.trim();
            
            // Handle empty lines
            if trimmed.is_empty() {
                if !prev_was_empty && !result.is_empty() {
                    result.push(String::new());
                    prev_was_empty = true;
                }
                continue;
            }
            prev_was_empty = false;

            // Handle comments
            if trimmed.starts_with('#') {
                result.push(trimmed.to_string());
                continue;
            }

            // Handle instructions
            let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
            if parts.len() == 2 {
                let instruction = parts[0].to_uppercase();
                let args = parts[1].trim();
                result.push(format!("{} {}", instruction, args));
            } else if parts.len() == 1 {
                result.push(parts[0].to_uppercase());
            }
        }

        result.join("\n")
    }

    /// Get document count
    #[wasm_bindgen(js_name = documentCount)]
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// Clear all documents
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.documents.clear();
    }

    /// Get server capabilities as JSON
    #[wasm_bindgen(js_name = getCapabilities)]
    pub fn get_capabilities() -> String {
        serde_json::json!({
            "textDocumentSync": 1,
            "completionProvider": {
                "triggerCharacters": [" ", "\n"],
                "resolveProvider": false
            },
            "hoverProvider": true,
            "diagnosticProvider": {
                "interFileDependencies": false,
                "workspaceDiagnostics": false
            },
            "documentFormattingProvider": true
        }).to_string()
    }
}

impl Default for RunefileLspServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_server() {
        let mut server = RunefileLspServer::new();
        server.open_document("file:///test.dockerfile", "FROM alpine\nRUN echo hello", 1);
        
        let diagnostics = server.get_diagnostics("file:///test.dockerfile");
        assert!(diagnostics.contains("[]") || !diagnostics.contains("error"));
    }

    #[test]
    fn test_validate() {
        let mut server = RunefileLspServer::new();
        let result = server.validate("FROM alpine\nRUN echo test");
        assert!(result.contains("\"valid\":true"));
    }

    #[test]
    fn test_format() {
        let server = RunefileLspServer::new();
        let formatted = server.format("from alpine\nrun echo hello");
        assert!(formatted.contains("FROM alpine"));
        assert!(formatted.contains("RUN echo hello"));
    }
}
