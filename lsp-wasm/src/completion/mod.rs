//! Code completion for Runefile LSP

use crate::parser::types::*;
use wasm_bindgen::prelude::*;

/// Completion kind constants (LSP spec)
pub const COMPLETION_KIND_KEYWORD: u8 = 14;
pub const COMPLETION_KIND_SNIPPET: u8 = 15;
pub const COMPLETION_KIND_VALUE: u8 = 12;

/// Completion provider for Runefile
#[wasm_bindgen]
pub struct CompletionProvider;

#[wasm_bindgen]
impl CompletionProvider {
    /// Create a new completion provider
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self
    }

    /// Get completions at position (works offline)
    #[wasm_bindgen(js_name = getCompletions)]
    pub fn get_completions(&self, content: &str, line: u32, character: u32) -> String {
        let lines: Vec<&str> = content.lines().collect();

        if (line as usize) >= lines.len() {
            return self.get_instruction_completions();
        }

        let current_line = lines[line as usize];
        let prefix = if (character as usize) <= current_line.len() {
            &current_line[..character as usize]
        } else {
            current_line
        };

        let trimmed = prefix.trim();

        // At start of line or after whitespace - suggest instructions
        if trimmed.is_empty() || prefix.ends_with(' ') && trimmed.len() < 3 {
            return self.get_instruction_completions();
        }

        // Check if we're completing an instruction name
        let upper = trimmed.to_uppercase();
        if !trimmed.contains(' ') {
            return self.get_filtered_instruction_completions(&upper);
        }

        // Context-specific completions
        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        let instruction = parts[0].to_uppercase();

        match instruction.as_str() {
            "FROM" => self.get_from_completions(),
            "RUN" => self.get_run_completions(),
            "COPY" | "ADD" => self.get_copy_completions(),
            "EXPOSE" => self.get_expose_completions(),
            "ENV" => self.get_env_completions(),
            "HEALTHCHECK" => self.get_healthcheck_completions(),
            "CMD" | "ENTRYPOINT" => self.get_cmd_completions(),
            _ => "[]".to_string(),
        }
    }

    fn get_instruction_completions(&self) -> String {
        let completions = vec![
            self.instruction_completion("FROM", "Base image", "FROM ${1:image}:${2:tag}"),
            self.instruction_completion("RUN", "Execute command", "RUN ${1:command}"),
            self.instruction_completion("COPY", "Copy files", "COPY ${1:src} ${2:dest}"),
            self.instruction_completion("ADD", "Add files", "ADD ${1:src} ${2:dest}"),
            self.instruction_completion("CMD", "Default command", "CMD [\"${1:command}\"]"),
            self.instruction_completion(
                "ENTRYPOINT",
                "Entry point",
                "ENTRYPOINT [\"${1:command}\"]",
            ),
            self.instruction_completion("ENV", "Environment variable", "ENV ${1:KEY}=${2:value}"),
            self.instruction_completion("EXPOSE", "Expose port", "EXPOSE ${1:port}"),
            self.instruction_completion("WORKDIR", "Working directory", "WORKDIR ${1:/app}"),
            self.instruction_completion("USER", "Run as user", "USER ${1:user}"),
            self.instruction_completion("VOLUME", "Mount point", "VOLUME ${1:/data}"),
            self.instruction_completion("ARG", "Build argument", "ARG ${1:name}=${2:default}"),
            self.instruction_completion("LABEL", "Metadata label", "LABEL ${1:key}=\"${2:value}\""),
            self.instruction_completion(
                "HEALTHCHECK",
                "Health check",
                "HEALTHCHECK CMD ${1:command}",
            ),
            self.instruction_completion(
                "SHELL",
                "Default shell",
                "SHELL [\"${1:/bin/bash}\", \"-c\"]",
            ),
            self.instruction_completion("STOPSIGNAL", "Stop signal", "STOPSIGNAL ${1:SIGTERM}"),
        ];

        serde_json::to_string(&completions).unwrap_or_else(|_| "[]".to_string())
    }

    fn get_filtered_instruction_completions(&self, prefix: &str) -> String {
        let all: Vec<CompletionItem> =
            serde_json::from_str(&self.get_instruction_completions()).unwrap_or_default();
        let filtered: Vec<CompletionItem> = all
            .into_iter()
            .filter(|c| c.label.to_uppercase().starts_with(prefix))
            .collect();
        serde_json::to_string(&filtered).unwrap_or_else(|_| "[]".to_string())
    }

    fn get_from_completions(&self) -> String {
        let completions = vec![
            self.value_completion("alpine", "Minimal Linux", "alpine:${1:latest}"),
            self.value_completion("ubuntu", "Ubuntu Linux", "ubuntu:${1:22.04}"),
            self.value_completion("debian", "Debian Linux", "debian:${1:bookworm}"),
            self.value_completion("node", "Node.js", "node:${1:20}-alpine"),
            self.value_completion("python", "Python", "python:${1:3.11}-slim"),
            self.value_completion("rust", "Rust", "rust:${1:1.70}"),
            self.value_completion("golang", "Go", "golang:${1:1.21}-alpine"),
            self.value_completion("nginx", "Nginx", "nginx:${1:alpine}"),
            self.value_completion("scratch", "Empty image", "scratch"),
        ];
        serde_json::to_string(&completions).unwrap_or_else(|_| "[]".to_string())
    }

    fn get_run_completions(&self) -> String {
        let completions = vec![
            self.snippet_completion(
                "apt-get install",
                "Install packages (Debian)",
                "apt-get update && apt-get install -y ${1:package}",
            ),
            self.snippet_completion(
                "apk add",
                "Install packages (Alpine)",
                "apk add --no-cache ${1:package}",
            ),
            self.snippet_completion(
                "pip install",
                "Install Python packages",
                "pip install --no-cache-dir ${1:package}",
            ),
            self.snippet_completion(
                "npm install",
                "Install Node packages",
                "npm install ${1:package}",
            ),
            self.snippet_completion("cargo build", "Build Rust project", "cargo build --release"),
            self.snippet_completion("chmod", "Change permissions", "chmod +x ${1:file}"),
            self.snippet_completion("mkdir", "Create directory", "mkdir -p ${1:/app}"),
        ];
        serde_json::to_string(&completions).unwrap_or_else(|_| "[]".to_string())
    }

    fn get_copy_completions(&self) -> String {
        let completions = vec![
            self.snippet_completion(
                "--from",
                "Copy from stage",
                "--from=${1:builder} ${2:src} ${3:dest}",
            ),
            self.snippet_completion(
                "--chown",
                "Set ownership",
                "--chown=${1:user}:${2:group} ${3:src} ${4:dest}",
            ),
            self.snippet_completion(". .", "Copy current dir", ". ."),
            self.snippet_completion("package.json", "Copy package.json", "package*.json ./"),
            self.snippet_completion(
                "requirements.txt",
                "Copy requirements",
                "requirements.txt .",
            ),
        ];
        serde_json::to_string(&completions).unwrap_or_else(|_| "[]".to_string())
    }

    fn get_expose_completions(&self) -> String {
        let completions = vec![
            self.value_completion("80", "HTTP", "80"),
            self.value_completion("443", "HTTPS", "443"),
            self.value_completion("3000", "Node.js default", "3000"),
            self.value_completion("5000", "Flask default", "5000"),
            self.value_completion("8080", "Alt HTTP", "8080"),
            self.value_completion("8000", "Django default", "8000"),
        ];
        serde_json::to_string(&completions).unwrap_or_else(|_| "[]".to_string())
    }

    fn get_env_completions(&self) -> String {
        let completions = vec![
            self.snippet_completion("PATH", "Add to PATH", "PATH=\"/app/bin:$PATH\""),
            self.snippet_completion("NODE_ENV", "Node environment", "NODE_ENV=${1:production}"),
            self.snippet_completion(
                "PYTHONUNBUFFERED",
                "Python unbuffered",
                "PYTHONUNBUFFERED=1",
            ),
            self.snippet_completion("RUST_LOG", "Rust logging", "RUST_LOG=${1:info}"),
        ];
        serde_json::to_string(&completions).unwrap_or_else(|_| "[]".to_string())
    }

    fn get_healthcheck_completions(&self) -> String {
        let completions = vec![
            self.snippet_completion("NONE", "Disable healthcheck", "NONE"),
            self.snippet_completion(
                "CMD curl",
                "HTTP health check",
                "CMD curl -f http://localhost:${1:80}/ || exit 1",
            ),
            self.snippet_completion(
                "CMD wget",
                "Wget health check",
                "CMD wget -q --spider http://localhost:${1:80}/ || exit 1",
            ),
            self.snippet_completion(
                "--interval",
                "Check interval",
                "--interval=${1:30s} CMD ${2:command}",
            ),
            self.snippet_completion(
                "--timeout",
                "Timeout option",
                "--timeout=${1:10s} CMD ${2:command}",
            ),
            self.snippet_completion(
                "--retries",
                "Retry count",
                "--retries=${1:3} CMD ${2:command}",
            ),
        ];
        serde_json::to_string(&completions).unwrap_or_else(|_| "[]".to_string())
    }

    fn get_cmd_completions(&self) -> String {
        let completions = vec![
            self.snippet_completion(
                "exec form",
                "JSON array form",
                "[\"${1:command}\", \"${2:arg}\"]",
            ),
            self.snippet_completion("shell form", "Shell form", "${1:command} ${2:args}"),
        ];
        serde_json::to_string(&completions).unwrap_or_else(|_| "[]".to_string())
    }

    fn instruction_completion(&self, label: &str, detail: &str, insert: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: COMPLETION_KIND_KEYWORD,
            detail: Some(detail.to_string()),
            documentation: None,
            insert_text: Some(insert.to_string()),
            insert_text_format: Some(2), // Snippet format
        }
    }

    fn snippet_completion(&self, label: &str, detail: &str, insert: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: COMPLETION_KIND_SNIPPET,
            detail: Some(detail.to_string()),
            documentation: None,
            insert_text: Some(insert.to_string()),
            insert_text_format: Some(2),
        }
    }

    fn value_completion(&self, label: &str, detail: &str, insert: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: COMPLETION_KIND_VALUE,
            detail: Some(detail.to_string()),
            documentation: None,
            insert_text: Some(insert.to_string()),
            insert_text_format: Some(2),
        }
    }
}

impl Default for CompletionProvider {
    fn default() -> Self {
        Self::new()
    }
}
