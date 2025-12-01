//! Runefile Language Server Protocol - WebAssembly Version
//!
//! This module provides a WASM-compatible LSP for Runefile editing in web-based IDEs.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use std::collections::HashMap;

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

/// Runefile parser
#[wasm_bindgen]
pub struct RunefileParser {
    instructions: Vec<Instruction>,
    errors: Vec<ParseError>,
}

#[wasm_bindgen]
impl RunefileParser {
    /// Create a new parser
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Parse Runefile content
    #[wasm_bindgen]
    pub fn parse(&mut self, content: &str) {
        self.instructions.clear();
        self.errors.clear();

        let mut has_from = false;
        let mut in_multiline = false;
        let mut multiline_buffer = String::new();
        let mut multiline_start_line = 0;

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Handle empty lines and comments
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with('#') {
                self.instructions.push(Instruction {
                    kind: InstructionKind::Comment,
                    line: line_num,
                    raw: line.to_string(),
                    keyword: "#".to_string(),
                    arguments: trimmed[1..].trim().to_string(),
                });
                continue;
            }

            // Handle line continuations
            if in_multiline {
                if trimmed.ends_with('\\') {
                    multiline_buffer.push(' ');
                    multiline_buffer.push_str(&trimmed[..trimmed.len() - 1]);
                } else {
                    multiline_buffer.push(' ');
                    multiline_buffer.push_str(trimmed);
                    self.parse_instruction(&multiline_buffer, multiline_start_line, &mut has_from);
                    in_multiline = false;
                    multiline_buffer.clear();
                }
                continue;
            }

            if trimmed.ends_with('\\') {
                in_multiline = true;
                multiline_start_line = line_num;
                multiline_buffer = trimmed[..trimmed.len() - 1].to_string();
                continue;
            }

            self.parse_instruction(line, line_num, &mut has_from);
        }

        // Check for missing FROM
        if !has_from && !self.instructions.is_empty() {
            self.errors.push(ParseError {
                line: 0,
                message: "Runefile must start with FROM instruction".to_string(),
                severity: ErrorSeverity::Error,
            });
        }
    }

    fn parse_instruction(&mut self, line: &str, line_num: usize, has_from: &mut bool) {
        let trimmed = line.trim();
        let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
        
        if parts.is_empty() {
            return;
        }

        let keyword = parts[0].to_uppercase();
        let arguments = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();

        let kind = match keyword.as_str() {
            "FROM" => {
                *has_from = true;
                InstructionKind::From
            }
            "RUN" => InstructionKind::Run,
            "COPY" => InstructionKind::Copy,
            "ADD" => InstructionKind::Add,
            "CMD" => InstructionKind::Cmd,
            "ENTRYPOINT" => InstructionKind::Entrypoint,
            "ENV" => InstructionKind::Env,
            "EXPOSE" => InstructionKind::Expose,
            "LABEL" => InstructionKind::Label,
            "MAINTAINER" => InstructionKind::Maintainer,
            "VOLUME" => InstructionKind::Volume,
            "WORKDIR" => InstructionKind::Workdir,
            "ARG" => InstructionKind::Arg,
            "USER" => InstructionKind::User,
            "HEALTHCHECK" => InstructionKind::Healthcheck,
            "SHELL" => InstructionKind::Shell,
            "STOPSIGNAL" => InstructionKind::Stopsignal,
            "ONBUILD" => InstructionKind::Onbuild,
            _ => {
                self.errors.push(ParseError {
                    line: line_num,
                    message: format!("Unknown instruction: {}", keyword),
                    severity: ErrorSeverity::Warning,
                });
                InstructionKind::Unknown
            }
        };

        // Validate arguments
        self.validate_instruction(kind, &arguments, line_num);

        self.instructions.push(Instruction {
            kind,
            line: line_num,
            raw: line.to_string(),
            keyword,
            arguments,
        });
    }

    fn validate_instruction(&mut self, kind: InstructionKind, arguments: &str, line_num: usize) {
        match kind {
            InstructionKind::From => {
                if arguments.is_empty() {
                    self.errors.push(ParseError {
                        line: line_num,
                        message: "FROM requires an image argument".to_string(),
                        severity: ErrorSeverity::Error,
                    });
                }
            }
            InstructionKind::Copy | InstructionKind::Add => {
                let args: Vec<&str> = arguments.split_whitespace().collect();
                let non_flag_args: Vec<&&str> = args.iter()
                    .filter(|a| !a.starts_with("--"))
                    .collect();
                if non_flag_args.len() < 2 {
                    self.errors.push(ParseError {
                        line: line_num,
                        message: format!("{} requires at least two arguments (source and destination)", 
                            if kind == InstructionKind::Copy { "COPY" } else { "ADD" }),
                        severity: ErrorSeverity::Error,
                    });
                }
            }
            InstructionKind::Expose => {
                for port in arguments.split_whitespace() {
                    let port_num = port.split('/').next().unwrap_or("");
                    if port_num.parse::<u16>().is_err() {
                        self.errors.push(ParseError {
                            line: line_num,
                            message: format!("Invalid port number: {}", port),
                            severity: ErrorSeverity::Warning,
                        });
                    }
                }
            }
            InstructionKind::Workdir => {
                if arguments.is_empty() {
                    self.errors.push(ParseError {
                        line: line_num,
                        message: "WORKDIR requires a path argument".to_string(),
                        severity: ErrorSeverity::Error,
                    });
                } else if !arguments.starts_with('/') && !arguments.starts_with('$') {
                    self.errors.push(ParseError {
                        line: line_num,
                        message: "WORKDIR should use absolute path".to_string(),
                        severity: ErrorSeverity::Warning,
                    });
                }
            }
            InstructionKind::Healthcheck => {
                if !arguments.is_empty() && !arguments.starts_with("NONE") && !arguments.starts_with("CMD") {
                    self.errors.push(ParseError {
                        line: line_num,
                        message: "HEALTHCHECK must be NONE or CMD".to_string(),
                        severity: ErrorSeverity::Error,
                    });
                }
            }
            _ => {}
        }
    }

    /// Get diagnostics as JSON
    #[wasm_bindgen]
    pub fn get_diagnostics_json(&self) -> String {
        let diagnostics: Vec<Diagnostic> = self.errors.iter().map(|e| {
            Diagnostic {
                range: Range {
                    start: Position { line: e.line as u32, character: 0 },
                    end: Position { line: e.line as u32, character: 100 },
                },
                severity: match e.severity {
                    ErrorSeverity::Error => 1,
                    ErrorSeverity::Warning => 2,
                    ErrorSeverity::Information => 3,
                    ErrorSeverity::Hint => 4,
                },
                message: e.message.clone(),
                source: "runefile-lsp".to_string(),
            }
        }).collect();

        serde_json::to_string(&diagnostics).unwrap_or_default()
    }

    /// Get instruction count
    #[wasm_bindgen]
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// Get error count
    #[wasm_bindgen]
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}

impl Default for RunefileParser {
    fn default() -> Self {
        Self::new()
    }
}

/// WASM LSP Server
#[wasm_bindgen]
pub struct WasmLspServer {
    documents: HashMap<String, String>,
    parsers: HashMap<String, RunefileParser>,
}

#[wasm_bindgen]
impl WasmLspServer {
    /// Create a new WASM LSP server
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            parsers: HashMap::new(),
        }
    }

    /// Open a document
    #[wasm_bindgen]
    pub fn did_open(&mut self, uri: &str, content: &str) -> String {
        self.documents.insert(uri.to_string(), content.to_string());
        
        let mut parser = RunefileParser::new();
        parser.parse(content);
        let diagnostics = parser.get_diagnostics_json();
        self.parsers.insert(uri.to_string(), parser);
        
        diagnostics
    }

    /// Update a document
    #[wasm_bindgen]
    pub fn did_change(&mut self, uri: &str, content: &str) -> String {
        self.documents.insert(uri.to_string(), content.to_string());
        
        let mut parser = RunefileParser::new();
        parser.parse(content);
        let diagnostics = parser.get_diagnostics_json();
        self.parsers.insert(uri.to_string(), parser);
        
        diagnostics
    }

    /// Close a document
    #[wasm_bindgen]
    pub fn did_close(&mut self, uri: &str) {
        self.documents.remove(uri);
        self.parsers.remove(uri);
    }

    /// Get completions
    #[wasm_bindgen]
    pub fn get_completions(&self, uri: &str, line: u32, character: u32) -> String {
        let content = match self.documents.get(uri) {
            Some(c) => c,
            None => return "[]".to_string(),
        };

        let lines: Vec<&str> = content.lines().collect();
        let current_line = lines.get(line as usize).unwrap_or(&"");
        let prefix = &current_line[..std::cmp::min(character as usize, current_line.len())];
        let trimmed = prefix.trim().to_uppercase();

        let mut completions = Vec::new();

        // Instruction completions
        let instructions = [
            ("FROM", "Base image", "FROM ${1:image}:${2:tag}"),
            ("RUN", "Execute command", "RUN ${1:command}"),
            ("COPY", "Copy files", "COPY ${1:src} ${2:dest}"),
            ("ADD", "Add files", "ADD ${1:src} ${2:dest}"),
            ("CMD", "Default command", "CMD [\"${1:command}\"]"),
            ("ENTRYPOINT", "Entry point", "ENTRYPOINT [\"${1:command}\"]"),
            ("ENV", "Environment variable", "ENV ${1:key}=${2:value}"),
            ("EXPOSE", "Expose port", "EXPOSE ${1:port}"),
            ("LABEL", "Add label", "LABEL ${1:key}=\"${2:value}\""),
            ("VOLUME", "Define volume", "VOLUME [\"${1:path}\"]"),
            ("WORKDIR", "Working directory", "WORKDIR ${1:/app}"),
            ("ARG", "Build argument", "ARG ${1:name}"),
            ("USER", "Set user", "USER ${1:user}"),
            ("HEALTHCHECK", "Health check", "HEALTHCHECK CMD ${1:command}"),
            ("SHELL", "Default shell", "SHELL [\"${1:/bin/sh}\", \"-c\"]"),
            ("STOPSIGNAL", "Stop signal", "STOPSIGNAL ${1:SIGTERM}"),
        ];

        for (label, detail, snippet) in instructions {
            if trimmed.is_empty() || label.starts_with(&trimmed) {
                completions.push(CompletionItem {
                    label: label.to_string(),
                    kind: 14, // Keyword
                    detail: Some(detail.to_string()),
                    documentation: Some(format!("Runefile {} instruction", label)),
                    insert_text: Some(snippet.to_string()),
                    insert_text_format: Some(2), // Snippet
                });
            }
        }

        serde_json::to_string(&completions).unwrap_or_default()
    }

    /// Get hover information
    #[wasm_bindgen]
    pub fn get_hover(&self, uri: &str, line: u32, _character: u32) -> String {
        let content = match self.documents.get(uri) {
            Some(c) => c,
            None => return "null".to_string(),
        };

        let lines: Vec<&str> = content.lines().collect();
        let current_line = lines.get(line as usize).unwrap_or(&"");
        let trimmed = current_line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            return "null".to_string();
        }

        let keyword = trimmed.split_whitespace().next().unwrap_or("").to_uppercase();

        let documentation = match keyword.as_str() {
            "FROM" => "**FROM** - Sets the base image for subsequent instructions.\n\n```\nFROM <image>[:<tag>] [AS <name>]\n```",
            "RUN" => "**RUN** - Executes commands in a new layer on top of the current image.\n\n```\nRUN <command>\nRUN [\"executable\", \"param1\", \"param2\"]\n```",
            "COPY" => "**COPY** - Copies files/directories from source to destination.\n\n```\nCOPY [--chown=<user>:<group>] <src>... <dest>\n```",
            "ADD" => "**ADD** - Adds files, directories, or remote URLs to the image.\n\n```\nADD [--chown=<user>:<group>] <src>... <dest>\n```",
            "CMD" => "**CMD** - Provides defaults for executing container.\n\n```\nCMD [\"executable\",\"param1\",\"param2\"]\nCMD command param1 param2\n```",
            "ENTRYPOINT" => "**ENTRYPOINT** - Configures container to run as executable.\n\n```\nENTRYPOINT [\"executable\", \"param1\", \"param2\"]\n```",
            "ENV" => "**ENV** - Sets environment variables.\n\n```\nENV <key>=<value> ...\n```",
            "EXPOSE" => "**EXPOSE** - Informs Docker about ports the container listens on.\n\n```\nEXPOSE <port>[/<protocol>]...\n```",
            "LABEL" => "**LABEL** - Adds metadata to an image.\n\n```\nLABEL <key>=<value> ...\n```",
            "VOLUME" => "**VOLUME** - Creates a mount point for external volumes.\n\n```\nVOLUME [\"/data\"]\n```",
            "WORKDIR" => "**WORKDIR** - Sets the working directory.\n\n```\nWORKDIR /path/to/workdir\n```",
            "ARG" => "**ARG** - Defines a build-time variable.\n\n```\nARG <name>[=<default value>]\n```",
            "USER" => "**USER** - Sets the user for subsequent instructions.\n\n```\nUSER <user>[:<group>]\n```",
            "HEALTHCHECK" => "**HEALTHCHECK** - Tells Docker how to test container health.\n\n```\nHEALTHCHECK CMD <command>\nHEALTHCHECK NONE\n```",
            "SHELL" => "**SHELL** - Overrides default shell.\n\n```\nSHELL [\"executable\", \"parameters\"]\n```",
            "STOPSIGNAL" => "**STOPSIGNAL** - Sets the system call signal for exit.\n\n```\nSTOPSIGNAL signal\n```",
            "ONBUILD" => "**ONBUILD** - Adds trigger instruction for child images.\n\n```\nONBUILD <INSTRUCTION>\n```",
            _ => return "null".to_string(),
        };

        let result = HoverResult {
            contents: documentation.to_string(),
            range: Some(Range {
                start: Position { line, character: 0 },
                end: Position { line, character: current_line.len() as u32 },
            }),
        };

        serde_json::to_string(&result).unwrap_or_default()
    }

    /// Format document
    #[wasm_bindgen]
    pub fn format_document(&self, uri: &str) -> String {
        let content = match self.documents.get(uri) {
            Some(c) => c,
            None => return "[]".to_string(),
        };

        let mut edits = Vec::new();
        
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            
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

                    if line != formatted {
                        edits.push(serde_json::json!({
                            "range": {
                                "start": {"line": i, "character": 0},
                                "end": {"line": i, "character": line.len()}
                            },
                            "newText": formatted
                        }));
                    }
                }
            }
        }

        serde_json::to_string(&edits).unwrap_or_default()
    }

    /// Initialize server capabilities
    #[wasm_bindgen]
    pub fn get_capabilities() -> String {
        serde_json::json!({
            "textDocumentSync": 1,
            "completionProvider": {
                "triggerCharacters": [" ", "-", "="],
                "resolveProvider": false
            },
            "hoverProvider": true,
            "documentFormattingProvider": true
        }).to_string()
    }
}

impl Default for WasmLspServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_basic() {
        let mut parser = RunefileParser::new();
        parser.parse("FROM alpine\nRUN echo hello");
        assert_eq!(parser.instruction_count(), 2);
        assert_eq!(parser.error_count(), 0);
    }

    #[test]
    fn test_parser_missing_from() {
        let mut parser = RunefileParser::new();
        parser.parse("RUN echo hello");
        assert!(parser.error_count() > 0);
    }

    #[test]
    fn test_lsp_server() {
        let mut server = WasmLspServer::new();
        let diagnostics = server.did_open("file:///test", "FROM alpine\nRUN echo");
        assert!(diagnostics.contains("[]") || !diagnostics.contains("error"));
    }
}
