//! Runefile parser for LSP

pub mod types;

pub use types::*;

use wasm_bindgen::prelude::*;

/// Runefile parser
#[wasm_bindgen]
pub struct RunefileParser {
    #[wasm_bindgen(skip)]
    pub instructions: Vec<Instruction>,
    #[wasm_bindgen(skip)]
    pub errors: Vec<ParseError>,
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
        let arguments = parts
            .get(1)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

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
                let non_flag_args: Vec<&&str> =
                    args.iter().filter(|a| !a.starts_with("--")).collect();
                if non_flag_args.len() < 2 {
                    self.errors.push(ParseError {
                        line: line_num,
                        message: format!(
                            "{} requires at least two arguments (source and destination)",
                            if kind == InstructionKind::Copy {
                                "COPY"
                            } else {
                                "ADD"
                            }
                        ),
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
                if !arguments.is_empty()
                    && !arguments.starts_with("NONE")
                    && !arguments.starts_with("CMD")
                {
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
        let diagnostics: Vec<Diagnostic> = self
            .errors
            .iter()
            .map(|e| Diagnostic {
                range: Range {
                    start: Position {
                        line: e.line as u32,
                        character: 0,
                    },
                    end: Position {
                        line: e.line as u32,
                        character: 100,
                    },
                },
                severity: match e.severity {
                    ErrorSeverity::Error => 1,
                    ErrorSeverity::Warning => 2,
                    ErrorSeverity::Information => 3,
                    ErrorSeverity::Hint => 4,
                },
                message: e.message.clone(),
                source: "runefile-lsp".to_string(),
            })
            .collect();

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
}
