//! Completion Provider for Runefile LSP

use super::server::CompletionItem;
use super::syntax::{InstructionKind, RunefileParser};

/// Completion provider for Runefile
pub struct CompletionProvider {
    /// All available instructions
    instructions: Vec<InstructionKind>,
}

impl CompletionProvider {
    /// Create a new completion provider
    pub fn new() -> Self {
        Self {
            instructions: vec![
                InstructionKind::From,
                InstructionKind::Run,
                InstructionKind::Cmd,
                InstructionKind::Label,
                InstructionKind::Expose,
                InstructionKind::Env,
                InstructionKind::Add,
                InstructionKind::Copy,
                InstructionKind::Entrypoint,
                InstructionKind::Volume,
                InstructionKind::User,
                InstructionKind::Workdir,
                InstructionKind::Arg,
                InstructionKind::Onbuild,
                InstructionKind::Stopsignal,
                InstructionKind::Healthcheck,
                InstructionKind::Shell,
            ],
        }
    }

    /// Get completions at the given position
    pub fn get_completions(
        &self,
        content: &str,
        parser: &RunefileParser,
        line: usize,
        column: usize,
        snippet_support: bool,
    ) -> Vec<CompletionItem> {
        let lines: Vec<&str> = content.lines().collect();
        let current_line = lines.get(line).copied().unwrap_or("");
        let before_cursor = &current_line[..column.min(current_line.len())];
        let trimmed = before_cursor.trim();

        // Empty line or start of line - suggest instructions
        if trimmed.is_empty() {
            return self.instruction_completions(snippet_support);
        }

        // Partial instruction name
        let upper = trimmed.to_uppercase();
        if !trimmed.contains(' ') {
            return self
                .instruction_completions(snippet_support)
                .into_iter()
                .filter(|item| item.label.starts_with(&upper))
                .collect();
        }

        // Context-specific completions
        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        let instruction = parts[0].to_uppercase();
        let args = parts.get(1).copied().unwrap_or("");

        match instruction.as_str() {
            "FROM" => self.complete_from_instruction(args, parser),
            "COPY" => self.copy_completions(args, parser),
            "RUN" => self.run_completions(args),
            "HEALTHCHECK" => self.healthcheck_completions(args, snippet_support),
            "EXPOSE" => self.expose_completions(args),
            "ENV" => self.env_completions(args, parser),
            "ARG" => self.arg_completions(args),
            _ => Vec::new(),
        }
    }

    /// Get instruction completions
    fn instruction_completions(&self, snippet_support: bool) -> Vec<CompletionItem> {
        self.instructions
            .iter()
            .map(|kind| {
                let label = format!("{:?}", kind).to_uppercase();
                CompletionItem {
                    label: label.clone(),
                    kind: Some(14), // Keyword
                    detail: Some(format!("{} instruction", label)),
                    documentation: Some(kind.documentation().to_string()),
                    insert_text: if snippet_support {
                        Some(kind.snippet().to_string())
                    } else {
                        Some(format!("{} ", label))
                    },
                    insert_text_format: if snippet_support { Some(2) } else { Some(1) },
                }
            })
            .collect()
    }

    /// FROM completions
    fn complete_from_instruction(
        &self,
        args: &str,
        _parser: &RunefileParser,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Common base images
        let common_images = [
            ("alpine", "Minimal Alpine Linux image"),
            ("ubuntu", "Ubuntu Linux image"),
            ("debian", "Debian Linux image"),
            ("rust", "Official Rust image"),
            ("node", "Official Node.js image"),
            ("python", "Official Python image"),
            ("golang", "Official Go image"),
            ("nginx", "Official Nginx image"),
            ("redis", "Official Redis image"),
            ("postgres", "Official PostgreSQL image"),
            ("mysql", "Official MySQL image"),
            ("scratch", "Empty base image (for static binaries)"),
        ];

        for (image, description) in common_images {
            if image.starts_with(args) || args.is_empty() {
                items.push(CompletionItem {
                    label: image.to_string(),
                    kind: Some(6), // Variable
                    detail: Some(description.to_string()),
                    documentation: None,
                    insert_text: Some(format!("{}:latest", image)),
                    insert_text_format: Some(1),
                });
            }
        }

        // AS keyword for multi-stage builds
        if args.contains(':') && !args.contains(" AS") {
            items.push(CompletionItem {
                label: "AS".to_string(),
                kind: Some(14), // Keyword
                detail: Some("Name this build stage".to_string()),
                documentation: Some(
                    "Create a named build stage for multi-stage builds".to_string(),
                ),
                insert_text: Some("AS ".to_string()),
                insert_text_format: Some(1),
            });
        }

        items
    }

    /// COPY completions
    fn copy_completions(&self, args: &str, parser: &RunefileParser) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // --from flag for multi-stage builds
        if args.is_empty() || args.starts_with("--") {
            items.push(CompletionItem {
                label: "--from".to_string(),
                kind: Some(6), // Variable
                detail: Some("Copy from a build stage".to_string()),
                documentation: Some("Copy files from a named build stage".to_string()),
                insert_text: Some("--from=".to_string()),
                insert_text_format: Some(1),
            });

            items.push(CompletionItem {
                label: "--chown".to_string(),
                kind: Some(6),
                detail: Some("Set file ownership".to_string()),
                documentation: Some("Set user:group ownership of copied files".to_string()),
                insert_text: Some("--chown=".to_string()),
                insert_text_format: Some(1),
            });

            items.push(CompletionItem {
                label: "--chmod".to_string(),
                kind: Some(6),
                detail: Some("Set file permissions".to_string()),
                documentation: Some("Set permissions of copied files".to_string()),
                insert_text: Some("--chmod=".to_string()),
                insert_text_format: Some(1),
            });
        }

        // Stage names after --from=
        if args.contains("--from=") {
            for stage in parser.get_stages() {
                items.push(CompletionItem {
                    label: stage.clone(),
                    kind: Some(6),
                    detail: Some("Build stage".to_string()),
                    documentation: None,
                    insert_text: Some(stage.clone()),
                    insert_text_format: Some(1),
                });
            }
        }

        items
    }

    /// RUN completions
    fn run_completions(&self, args: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Common package managers and commands
        let commands = [
            (
                "apt-get update && apt-get install -y",
                "Update and install packages (Debian/Ubuntu)",
            ),
            ("apk add --no-cache", "Install packages (Alpine)"),
            ("yum install -y", "Install packages (RHEL/CentOS)"),
            ("dnf install -y", "Install packages (Fedora)"),
            ("pip install", "Install Python packages"),
            ("npm install", "Install Node.js packages"),
            (
                "cargo build --release",
                "Build Rust project in release mode",
            ),
            ("go build -o", "Build Go binary"),
            ("chmod +x", "Make file executable"),
            ("mkdir -p", "Create directory with parents"),
            ("rm -rf", "Remove files/directories recursively"),
        ];

        for (cmd, description) in commands {
            if cmd.starts_with(args) || args.is_empty() {
                items.push(CompletionItem {
                    label: cmd.to_string(),
                    kind: Some(1), // Text
                    detail: Some(description.to_string()),
                    documentation: None,
                    insert_text: Some(cmd.to_string()),
                    insert_text_format: Some(1),
                });
            }
        }

        items
    }

    /// HEALTHCHECK completions
    fn healthcheck_completions(&self, args: &str, snippet_support: bool) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        if args.is_empty() || args.starts_with("--") {
            items.push(CompletionItem {
                label: "--interval".to_string(),
                kind: Some(6),
                detail: Some("Time between health checks (default: 30s)".to_string()),
                documentation: None,
                insert_text: Some("--interval=30s ".to_string()),
                insert_text_format: Some(1),
            });

            items.push(CompletionItem {
                label: "--timeout".to_string(),
                kind: Some(6),
                detail: Some("Health check timeout (default: 30s)".to_string()),
                documentation: None,
                insert_text: Some("--timeout=30s ".to_string()),
                insert_text_format: Some(1),
            });

            items.push(CompletionItem {
                label: "--start-period".to_string(),
                kind: Some(6),
                detail: Some("Initialization grace period (default: 0s)".to_string()),
                documentation: None,
                insert_text: Some("--start-period=0s ".to_string()),
                insert_text_format: Some(1),
            });

            items.push(CompletionItem {
                label: "--retries".to_string(),
                kind: Some(6),
                detail: Some("Consecutive failures needed (default: 3)".to_string()),
                documentation: None,
                insert_text: Some("--retries=3 ".to_string()),
                insert_text_format: Some(1),
            });

            items.push(CompletionItem {
                label: "NONE".to_string(),
                kind: Some(14),
                detail: Some("Disable healthcheck".to_string()),
                documentation: Some(
                    "Disable any healthcheck inherited from the base image".to_string(),
                ),
                insert_text: Some("NONE".to_string()),
                insert_text_format: Some(1),
            });

            items.push(CompletionItem {
                label: "CMD".to_string(),
                kind: Some(14),
                detail: Some("Health check command".to_string()),
                documentation: None,
                insert_text: if snippet_support {
                    Some("CMD ${1:curl -f http://localhost/ || exit 1}".to_string())
                } else {
                    Some("CMD ".to_string())
                },
                insert_text_format: if snippet_support { Some(2) } else { Some(1) },
            });
        }

        // Common health check commands
        if args.contains("CMD") || args.ends_with(' ') {
            items.push(CompletionItem {
                label: "curl -f http://localhost/ || exit 1".to_string(),
                kind: Some(1),
                detail: Some("HTTP health check with curl".to_string()),
                documentation: None,
                insert_text: Some("curl -f http://localhost/ || exit 1".to_string()),
                insert_text_format: Some(1),
            });

            items.push(CompletionItem {
                label: "wget --spider http://localhost/health".to_string(),
                kind: Some(1),
                detail: Some("HTTP health check with wget".to_string()),
                documentation: None,
                insert_text: Some("wget --spider http://localhost/health".to_string()),
                insert_text_format: Some(1),
            });
        }

        items
    }

    /// EXPOSE completions
    fn expose_completions(&self, _args: &str) -> Vec<CompletionItem> {
        let common_ports = [
            ("80", "HTTP"),
            ("443", "HTTPS"),
            ("8080", "Alternative HTTP"),
            ("3000", "Node.js default"),
            ("5000", "Flask/Python default"),
            ("5432", "PostgreSQL"),
            ("3306", "MySQL"),
            ("6379", "Redis"),
            ("27017", "MongoDB"),
        ];

        common_ports
            .iter()
            .map(|(port, desc)| CompletionItem {
                label: port.to_string(),
                kind: Some(12), // Value
                detail: Some(desc.to_string()),
                documentation: None,
                insert_text: Some(port.to_string()),
                insert_text_format: Some(1),
            })
            .collect()
    }

    /// ENV completions
    fn env_completions(&self, _args: &str, _parser: &RunefileParser) -> Vec<CompletionItem> {
        let common_vars = [
            ("PATH", "System PATH"),
            ("HOME", "User home directory"),
            ("NODE_ENV", "Node.js environment"),
            ("PYTHONUNBUFFERED", "Python output buffering"),
            ("RUST_LOG", "Rust logging level"),
            ("TZ", "Timezone"),
            ("LANG", "Language/locale"),
        ];

        common_vars
            .iter()
            .map(|(var, desc)| CompletionItem {
                label: var.to_string(),
                kind: Some(6), // Variable
                detail: Some(desc.to_string()),
                documentation: None,
                insert_text: Some(format!("{}=", var)),
                insert_text_format: Some(1),
            })
            .collect()
    }

    /// ARG completions
    fn arg_completions(&self, _args: &str) -> Vec<CompletionItem> {
        let common_args = [
            ("VERSION", "Build version"),
            ("BUILD_DATE", "Build timestamp"),
            ("VCS_REF", "VCS commit reference"),
            ("TARGETPLATFORM", "Target platform"),
            ("TARGETOS", "Target OS"),
            ("TARGETARCH", "Target architecture"),
            ("BUILDPLATFORM", "Build platform"),
        ];

        common_args
            .iter()
            .map(|(arg, desc)| CompletionItem {
                label: arg.to_string(),
                kind: Some(6),
                detail: Some(desc.to_string()),
                documentation: None,
                insert_text: Some(format!("{}=", arg)),
                insert_text_format: Some(1),
            })
            .collect()
    }
}

impl Default for CompletionProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_completions() {
        let provider = CompletionProvider::new();
        let completions = provider.instruction_completions(false);

        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.label == "FROM"));
        assert!(completions.iter().any(|c| c.label == "RUN"));
        assert!(completions.iter().any(|c| c.label == "HEALTHCHECK"));
    }

    #[test]
    fn test_healthcheck_completions() {
        let provider = CompletionProvider::new();
        let completions = provider.healthcheck_completions("", false);

        assert!(completions.iter().any(|c| c.label == "--interval"));
        assert!(completions.iter().any(|c| c.label == "--timeout"));
        assert!(completions.iter().any(|c| c.label == "CMD"));
        assert!(completions.iter().any(|c| c.label == "NONE"));
    }
}
