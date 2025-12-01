//! Hover Provider for Runefile LSP

use super::server::{Hover, MarkupContent, Position, Range};
use super::syntax::{InstructionKind, RunefileParser};

/// Hover provider for Runefile
pub struct HoverProvider {}

impl HoverProvider {
    /// Create a new hover provider
    pub fn new() -> Self {
        Self {}
    }

    /// Get hover information at the given position
    pub fn get_hover(
        &self,
        content: &str,
        parser: &RunefileParser,
        line: usize,
        column: usize,
    ) -> Option<Hover> {
        // Find the instruction at this line
        let instruction = parser.instruction_at(line, column)?;

        // Get documentation for the instruction
        let documentation = instruction.kind.documentation();

        Some(Hover {
            contents: MarkupContent {
                kind: "markdown".to_string(),
                value: format!(
                    "```dockerfile\n{}\n```\n\n{}",
                    instruction.raw.trim(),
                    documentation
                ),
            },
            range: Some(Range {
                start: Position {
                    line: line as u32,
                    character: instruction.keyword_span.0 as u32,
                },
                end: Position {
                    line: line as u32,
                    character: instruction.keyword_span.1 as u32,
                },
            }),
        })
    }
}

impl Default for HoverProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hover_on_from() {
        let provider = HoverProvider::new();
        let mut parser = RunefileParser::new();
        parser.parse("FROM alpine:latest\nRUN echo hello");

        let hover = provider.get_hover("FROM alpine:latest\nRUN echo hello", &parser, 0, 0);
        assert!(hover.is_some());
        let hover = hover.unwrap();
        assert!(hover.contents.value.contains("FROM"));
    }

    #[test]
    fn test_hover_on_healthcheck() {
        let provider = HoverProvider::new();
        let mut parser = RunefileParser::new();
        parser.parse("FROM alpine\nHEALTHCHECK --interval=30s CMD curl localhost");

        let hover = provider.get_hover(
            "FROM alpine\nHEALTHCHECK --interval=30s CMD curl localhost",
            &parser,
            1,
            0,
        );
        assert!(hover.is_some());
        let hover = hover.unwrap();
        assert!(hover.contents.value.contains("HEALTHCHECK"));
        assert!(hover.contents.value.contains("--interval"));
    }
}
