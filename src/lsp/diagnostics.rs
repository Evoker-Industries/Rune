//! Diagnostics Provider for Runefile LSP

use super::syntax::{RunefileParser, ErrorSeverity};
use super::server::{Diagnostic, Range, Position};

/// Diagnostics provider for Runefile
pub struct DiagnosticsProvider {}

impl DiagnosticsProvider {
    /// Create a new diagnostics provider
    pub fn new() -> Self {
        Self {}
    }

    /// Get diagnostics for the parsed Runefile
    pub fn get_diagnostics(&self, parser: &RunefileParser) -> Vec<Diagnostic> {
        parser.errors.iter().map(|error| {
            Diagnostic {
                range: Range {
                    start: Position {
                        line: error.line as u32,
                        character: error.column as u32,
                    },
                    end: Position {
                        line: error.line as u32,
                        character: (error.column + 10) as u32, // Approximate end
                    },
                },
                severity: Some(match error.severity {
                    ErrorSeverity::Error => 1,
                    ErrorSeverity::Warning => 2,
                    ErrorSeverity::Info => 3,
                    ErrorSeverity::Hint => 4,
                }),
                code: None,
                source: Some("runefile-lsp".to_string()),
                message: error.message.clone(),
            }
        }).collect()
    }
}

impl Default for DiagnosticsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostics_missing_from() {
        let provider = DiagnosticsProvider::new();
        let mut parser = RunefileParser::new();
        parser.parse("RUN echo hello");

        let diagnostics = provider.get_diagnostics(&parser);
        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.message.contains("FROM")));
    }

    #[test]
    fn test_diagnostics_deprecated_maintainer() {
        let provider = DiagnosticsProvider::new();
        let mut parser = RunefileParser::new();
        parser.parse("FROM alpine\nMAINTAINER John Doe");

        let diagnostics = provider.get_diagnostics(&parser);
        assert!(diagnostics.iter().any(|d| d.severity == Some(2))); // Warning
    }

    #[test]
    fn test_diagnostics_valid_file() {
        let provider = DiagnosticsProvider::new();
        let mut parser = RunefileParser::new();
        parser.parse("FROM alpine:latest\nRUN echo hello\nCMD [\"echo\", \"world\"]");

        let diagnostics = provider.get_diagnostics(&parser);
        assert!(diagnostics.is_empty());
    }
}
