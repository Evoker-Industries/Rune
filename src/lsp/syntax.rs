//! Runefile Syntax Parser
//!
//! Parses Runefile/Dockerfile syntax for LSP features.

use std::collections::HashMap;

/// Runefile instruction kinds
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionKind {
    From,
    Run,
    Cmd,
    Label,
    Expose,
    Env,
    Add,
    Copy,
    Entrypoint,
    Volume,
    User,
    Workdir,
    Arg,
    Onbuild,
    Stopsignal,
    Healthcheck,
    Shell,
    Maintainer,
    Comment,
    Unknown(String),
}

impl InstructionKind {
    /// Parse instruction from string
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "FROM" => Self::From,
            "RUN" => Self::Run,
            "CMD" => Self::Cmd,
            "LABEL" => Self::Label,
            "EXPOSE" => Self::Expose,
            "ENV" => Self::Env,
            "ADD" => Self::Add,
            "COPY" => Self::Copy,
            "ENTRYPOINT" => Self::Entrypoint,
            "VOLUME" => Self::Volume,
            "USER" => Self::User,
            "WORKDIR" => Self::Workdir,
            "ARG" => Self::Arg,
            "ONBUILD" => Self::Onbuild,
            "STOPSIGNAL" => Self::Stopsignal,
            "HEALTHCHECK" => Self::Healthcheck,
            "SHELL" => Self::Shell,
            "MAINTAINER" => Self::Maintainer,
            _ => Self::Unknown(s.to_string()),
        }
    }

    /// Get documentation for this instruction
    pub fn documentation(&self) -> &'static str {
        match self {
            Self::From => r#"FROM - Set the base image for subsequent instructions.

Usage:
  FROM [--platform=<platform>] <image> [AS <name>]
  FROM [--platform=<platform>] <image>[:<tag>] [AS <name>]
  FROM [--platform=<platform>] <image>[@<digest>] [AS <name>]

Example:
  FROM ubuntu:22.04
  FROM rust:1.70 AS builder
  FROM --platform=linux/amd64 alpine:latest"#,

            Self::Run => r#"RUN - Execute commands in a new layer on top of the current image.

Usage:
  RUN <command>                    # Shell form
  RUN ["executable", "param1"]     # Exec form

Example:
  RUN apt-get update && apt-get install -y curl
  RUN ["apt-get", "install", "-y", "nginx"]"#,

            Self::Cmd => r#"CMD - Provide defaults for an executing container.

Usage:
  CMD ["executable","param1","param2"]  # Exec form (preferred)
  CMD ["param1","param2"]               # As default parameters to ENTRYPOINT
  CMD command param1 param2             # Shell form

Example:
  CMD ["nginx", "-g", "daemon off;"]
  CMD echo "Hello World""#,

            Self::Label => r#"LABEL - Add metadata to an image.

Usage:
  LABEL <key>=<value> <key>=<value> ...

Example:
  LABEL version="1.0"
  LABEL maintainer="user@example.com"
  LABEL org.opencontainers.image.source="https://github.com/example/repo""#,

            Self::Expose => r#"EXPOSE - Inform Docker that the container listens on specified ports.

Usage:
  EXPOSE <port> [<port>/<protocol>...]

Example:
  EXPOSE 80
  EXPOSE 80/tcp 443/tcp
  EXPOSE 8080/udp"#,

            Self::Env => r#"ENV - Set environment variables.

Usage:
  ENV <key>=<value> ...
  ENV <key> <value>

Example:
  ENV MY_VAR=value
  ENV PATH="/usr/local/bin:$PATH"
  ENV NODE_ENV=production PORT=3000"#,

            Self::Add => r#"ADD - Copy files, directories, or remote URLs to the image.

Usage:
  ADD [--chown=<user>:<group>] [--chmod=<perms>] <src>... <dest>
  ADD [--chown=<user>:<group>] [--chmod=<perms>] ["<src>",... "<dest>"]

Example:
  ADD app.tar.gz /app/
  ADD --chown=node:node package*.json ./
  ADD https://example.com/file.txt /app/"#,

            Self::Copy => r#"COPY - Copy files or directories to the image.

Usage:
  COPY [--chown=<user>:<group>] [--chmod=<perms>] <src>... <dest>
  COPY [--chown=<user>:<group>] [--chmod=<perms>] ["<src>",... "<dest>"]
  COPY [--from=<name>] <src>... <dest>

Example:
  COPY . /app
  COPY --from=builder /app/target/release/myapp /usr/local/bin/
  COPY --chown=1000:1000 config.json /etc/app/"#,

            Self::Entrypoint => r#"ENTRYPOINT - Configure a container to run as an executable.

Usage:
  ENTRYPOINT ["executable", "param1", "param2"]  # Exec form (preferred)
  ENTRYPOINT command param1 param2               # Shell form

Example:
  ENTRYPOINT ["docker-entrypoint.sh"]
  ENTRYPOINT ["/usr/bin/myapp"]"#,

            Self::Volume => r#"VOLUME - Create a mount point for external volumes.

Usage:
  VOLUME ["/data"]
  VOLUME /data /logs

Example:
  VOLUME /var/log
  VOLUME ["/data", "/config"]"#,

            Self::User => r#"USER - Set the user for subsequent instructions and container runtime.

Usage:
  USER <user>[:<group>]
  USER <UID>[:<GID>]

Example:
  USER node
  USER 1000:1000
  USER www-data:www-data"#,

            Self::Workdir => r#"WORKDIR - Set the working directory for subsequent instructions.

Usage:
  WORKDIR /path/to/workdir

Example:
  WORKDIR /app
  WORKDIR /home/user/project"#,

            Self::Arg => r#"ARG - Define a build-time variable.

Usage:
  ARG <name>[=<default value>]

Example:
  ARG VERSION=latest
  ARG NODE_VERSION
  ARG BUILD_DATE"#,

            Self::Onbuild => r#"ONBUILD - Add a trigger instruction for when the image is used as a base.

Usage:
  ONBUILD <INSTRUCTION>

Example:
  ONBUILD COPY . /app
  ONBUILD RUN npm install"#,

            Self::Stopsignal => r#"STOPSIGNAL - Set the system call signal for container exit.

Usage:
  STOPSIGNAL signal

Example:
  STOPSIGNAL SIGTERM
  STOPSIGNAL SIGKILL
  STOPSIGNAL 9"#,

            Self::Healthcheck => r#"HEALTHCHECK - Tell Docker how to test if the container is still working.

Usage:
  HEALTHCHECK [OPTIONS] CMD command
  HEALTHCHECK NONE

Options:
  --interval=DURATION (default: 30s)
  --timeout=DURATION (default: 30s)
  --start-period=DURATION (default: 0s)
  --start-interval=DURATION (default: 5s)
  --retries=N (default: 3)

Example:
  HEALTHCHECK --interval=30s --timeout=3s CMD curl -f http://localhost/ || exit 1
  HEALTHCHECK --interval=5m --timeout=3s CMD wget --spider http://localhost:8080/health
  HEALTHCHECK NONE"#,

            Self::Shell => r#"SHELL - Override the default shell for shell form commands.

Usage:
  SHELL ["executable", "parameters"]

Example:
  SHELL ["/bin/bash", "-c"]
  SHELL ["powershell", "-command"]"#,

            Self::Maintainer => r#"MAINTAINER - Set the author of the image (deprecated, use LABEL instead).

Usage:
  MAINTAINER <name>

Example:
  MAINTAINER John Doe <john@example.com>

Note: This instruction is deprecated. Use LABEL instead:
  LABEL maintainer="john@example.com""#,

            Self::Comment => "Comment line starting with #",

            Self::Unknown(_) => "Unknown instruction",
        }
    }

    /// Get completion snippet for this instruction
    pub fn snippet(&self) -> &'static str {
        match self {
            Self::From => "FROM ${1:image}:${2:tag}",
            Self::Run => "RUN ${1:command}",
            Self::Cmd => r#"CMD ["${1:executable}", "${2:param}"]"#,
            Self::Label => r#"LABEL ${1:key}="${2:value}""#,
            Self::Expose => "EXPOSE ${1:port}",
            Self::Env => "ENV ${1:KEY}=${2:value}",
            Self::Add => "ADD ${1:src} ${2:dest}",
            Self::Copy => "COPY ${1:src} ${2:dest}",
            Self::Entrypoint => r#"ENTRYPOINT ["${1:executable}"]"#,
            Self::Volume => "VOLUME ${1:/path}",
            Self::User => "USER ${1:user}",
            Self::Workdir => "WORKDIR ${1:/app}",
            Self::Arg => "ARG ${1:name}=${2:default}",
            Self::Onbuild => "ONBUILD ${1:INSTRUCTION}",
            Self::Stopsignal => "STOPSIGNAL ${1:SIGTERM}",
            Self::Healthcheck => "HEALTHCHECK --interval=${1:30s} --timeout=${2:3s} CMD ${3:command}",
            Self::Shell => r#"SHELL ["${1:/bin/bash}", "${2:-c}"]"#,
            Self::Maintainer => "MAINTAINER ${1:name}",
            _ => "",
        }
    }
}

/// A parsed instruction from a Runefile
#[derive(Debug, Clone)]
pub struct Instruction {
    /// The instruction kind
    pub kind: InstructionKind,
    /// The raw instruction text
    pub raw: String,
    /// Arguments to the instruction
    pub arguments: String,
    /// Line number (0-indexed)
    pub line: usize,
    /// Column offset
    pub column: usize,
    /// Span of the instruction keyword
    pub keyword_span: (usize, usize),
    /// Span of the arguments
    pub arguments_span: Option<(usize, usize)>,
}

/// Parser for Runefile/Dockerfile syntax
pub struct RunefileParser {
    /// Parsed instructions
    pub instructions: Vec<Instruction>,
    /// Build arguments defined in the file
    pub args: HashMap<String, Option<String>>,
    /// Environment variables defined
    pub envs: HashMap<String, String>,
    /// Labels defined
    pub labels: HashMap<String, String>,
    /// Build stages (FROM ... AS name)
    pub stages: Vec<String>,
    /// Parser errors
    pub errors: Vec<ParseError>,
}

/// Parse error
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub severity: ErrorSeverity,
}

/// Error severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl RunefileParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            args: HashMap::new(),
            envs: HashMap::new(),
            labels: HashMap::new(),
            stages: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Parse a Runefile/Dockerfile
    pub fn parse(&mut self, content: &str) {
        self.instructions.clear();
        self.errors.clear();
        self.args.clear();
        self.envs.clear();
        self.labels.clear();
        self.stages.clear();

        let mut continuation_buffer = String::new();
        let mut continuation_start_line = 0;

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Handle line continuations
            if trimmed.ends_with('\\') {
                if continuation_buffer.is_empty() {
                    continuation_start_line = line_num;
                }
                continuation_buffer.push_str(&trimmed[..trimmed.len() - 1]);
                continuation_buffer.push(' ');
                continue;
            }

            let full_line = if !continuation_buffer.is_empty() {
                continuation_buffer.push_str(trimmed);
                let result = continuation_buffer.clone();
                continuation_buffer.clear();
                result
            } else {
                trimmed.to_string()
            };

            let actual_line = if !continuation_buffer.is_empty() {
                continuation_start_line
            } else {
                line_num
            };

            self.parse_line(&full_line, actual_line);
        }

        // Check for unclosed continuation
        if !continuation_buffer.is_empty() {
            self.errors.push(ParseError {
                message: "Unclosed line continuation".to_string(),
                line: continuation_start_line,
                column: 0,
                severity: ErrorSeverity::Error,
            });
        }

        // Validate the file
        self.validate();
    }

    /// Parse a single line
    fn parse_line(&mut self, line: &str, line_num: usize) {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            return;
        }

        // Handle comments
        if trimmed.starts_with('#') {
            // Check for parser directives
            if line_num == 0 || self.instructions.is_empty() {
                if let Some(directive) = trimmed.strip_prefix("# syntax=") {
                    // Syntax directive - valid
                    return;
                }
                if let Some(directive) = trimmed.strip_prefix("# escape=") {
                    // Escape directive - valid
                    return;
                }
            }

            self.instructions.push(Instruction {
                kind: InstructionKind::Comment,
                raw: line.to_string(),
                arguments: trimmed[1..].trim().to_string(),
                line: line_num,
                column: 0,
                keyword_span: (0, 1),
                arguments_span: Some((1, trimmed.len())),
            });
            return;
        }

        // Parse instruction
        let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
        if parts.is_empty() {
            return;
        }

        let keyword = parts[0];
        let arguments = parts.get(1).map(|s| s.trim()).unwrap_or("");
        let kind = InstructionKind::from_str(keyword);

        let keyword_start = line.find(keyword).unwrap_or(0);
        let keyword_end = keyword_start + keyword.len();

        let arguments_span = if !arguments.is_empty() {
            let arg_start = line[keyword_end..].find(|c: char| !c.is_whitespace())
                .map(|i| keyword_end + i)
                .unwrap_or(keyword_end);
            Some((arg_start, line.len()))
        } else {
            None
        };

        // Extract metadata
        match &kind {
            InstructionKind::Arg => {
                self.parse_arg(arguments);
            }
            InstructionKind::Env => {
                self.parse_env(arguments);
            }
            InstructionKind::Label => {
                self.parse_label(arguments);
            }
            InstructionKind::From => {
                self.parse_from(arguments);
            }
            _ => {}
        }

        self.instructions.push(Instruction {
            kind,
            raw: line.to_string(),
            arguments: arguments.to_string(),
            line: line_num,
            column: keyword_start,
            keyword_span: (keyword_start, keyword_end),
            arguments_span,
        });
    }

    fn parse_arg(&mut self, arguments: &str) {
        if let Some((name, default)) = arguments.split_once('=') {
            self.args.insert(name.trim().to_string(), Some(default.trim().to_string()));
        } else {
            self.args.insert(arguments.trim().to_string(), None);
        }
    }

    fn parse_env(&mut self, arguments: &str) {
        // Handle both ENV KEY=VALUE and ENV KEY VALUE formats
        if arguments.contains('=') {
            for pair in arguments.split_whitespace() {
                if let Some((key, value)) = pair.split_once('=') {
                    self.envs.insert(key.to_string(), value.trim_matches('"').to_string());
                }
            }
        } else {
            let parts: Vec<&str> = arguments.splitn(2, char::is_whitespace).collect();
            if parts.len() == 2 {
                self.envs.insert(parts[0].to_string(), parts[1].to_string());
            }
        }
    }

    fn parse_label(&mut self, arguments: &str) {
        for pair in arguments.split_whitespace() {
            if let Some((key, value)) = pair.split_once('=') {
                self.labels.insert(key.to_string(), value.trim_matches('"').to_string());
            }
        }
    }

    fn parse_from(&mut self, arguments: &str) {
        // Check for AS clause
        let parts: Vec<&str> = arguments.split_whitespace().collect();
        for (i, part) in parts.iter().enumerate() {
            if part.to_uppercase() == "AS" && i + 1 < parts.len() {
                self.stages.push(parts[i + 1].to_string());
                break;
            }
        }
    }

    /// Validate the parsed Runefile
    fn validate(&mut self) {
        // Check for FROM instruction
        let has_from = self.instructions.iter().any(|i| i.kind == InstructionKind::From);
        if !has_from {
            self.errors.push(ParseError {
                message: "Runefile must have at least one FROM instruction".to_string(),
                line: 0,
                column: 0,
                severity: ErrorSeverity::Error,
            });
        }

        // Check FROM is first (excluding ARG and comments)
        let first_non_arg = self.instructions.iter()
            .find(|i| i.kind != InstructionKind::Arg && i.kind != InstructionKind::Comment);
        
        if let Some(inst) = first_non_arg {
            if inst.kind != InstructionKind::From {
                self.errors.push(ParseError {
                    message: "First instruction must be FROM (except for ARG)".to_string(),
                    line: inst.line,
                    column: inst.column,
                    severity: ErrorSeverity::Error,
                });
            }
        }

        // Check for deprecated MAINTAINER
        for inst in &self.instructions {
            if inst.kind == InstructionKind::Maintainer {
                self.errors.push(ParseError {
                    message: "MAINTAINER is deprecated, use LABEL maintainer=\"...\" instead".to_string(),
                    line: inst.line,
                    column: inst.column,
                    severity: ErrorSeverity::Warning,
                });
            }
        }

        // Check for multiple CMD instructions
        let cmd_count = self.instructions.iter()
            .filter(|i| i.kind == InstructionKind::Cmd)
            .count();
        if cmd_count > 1 {
            self.errors.push(ParseError {
                message: "Multiple CMD instructions found; only the last one will be used".to_string(),
                line: 0,
                column: 0,
                severity: ErrorSeverity::Warning,
            });
        }

        // Check for HEALTHCHECK issues
        let healthcheck_issues: Vec<ParseError> = self.instructions
            .iter()
            .filter(|inst| inst.kind == InstructionKind::Healthcheck)
            .filter_map(|inst| Self::check_healthcheck(inst))
            .collect();
        
        self.errors.extend(healthcheck_issues);
    }

    fn check_healthcheck(inst: &Instruction) -> Option<ParseError> {
        let args = inst.arguments.to_uppercase();
        
        // HEALTHCHECK NONE is valid
        if args.trim() == "NONE" {
            return None;
        }

        // Must have CMD
        if !args.contains("CMD") {
            return Some(ParseError {
                message: "HEALTHCHECK must specify CMD or NONE".to_string(),
                line: inst.line,
                column: inst.column,
                severity: ErrorSeverity::Error,
            });
        }
        
        None
    }

    /// Get instruction at a specific position
    pub fn instruction_at(&self, line: usize, _column: usize) -> Option<&Instruction> {
        self.instructions.iter().find(|i| i.line == line)
    }

    /// Get all defined stages
    pub fn get_stages(&self) -> &[String] {
        &self.stages
    }

    /// Get all defined ARGs
    pub fn get_args(&self) -> &HashMap<String, Option<String>> {
        &self.args
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
    fn test_parse_simple_runefile() {
        let content = r#"
FROM rust:1.70
WORKDIR /app
COPY . .
RUN cargo build --release
CMD ["./target/release/myapp"]
"#;

        let mut parser = RunefileParser::new();
        parser.parse(content);

        assert!(parser.errors.is_empty());
        assert_eq!(parser.instructions.len(), 5);
        assert_eq!(parser.instructions[0].kind, InstructionKind::From);
        assert_eq!(parser.instructions[4].kind, InstructionKind::Cmd);
    }

    #[test]
    fn test_parse_healthcheck() {
        let content = r#"
FROM nginx:latest
HEALTHCHECK --interval=30s --timeout=3s CMD curl -f http://localhost/ || exit 1
"#;

        let mut parser = RunefileParser::new();
        parser.parse(content);

        assert!(parser.errors.is_empty());
        let healthcheck = parser.instructions.iter()
            .find(|i| i.kind == InstructionKind::Healthcheck);
        assert!(healthcheck.is_some());
    }

    #[test]
    fn test_parse_multi_stage() {
        let content = r#"
FROM rust:1.70 AS builder
WORKDIR /app
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/myapp /usr/local/bin/
CMD ["myapp"]
"#;

        let mut parser = RunefileParser::new();
        parser.parse(content);

        assert!(parser.errors.is_empty());
        assert_eq!(parser.stages.len(), 1);
        assert_eq!(parser.stages[0], "builder");
    }

    #[test]
    fn test_missing_from() {
        let content = r#"
RUN echo "hello"
"#;

        let mut parser = RunefileParser::new();
        parser.parse(content);

        assert!(!parser.errors.is_empty());
        assert!(parser.errors.iter().any(|e| e.message.contains("FROM")));
    }

    #[test]
    fn test_deprecated_maintainer() {
        let content = r#"
FROM alpine
MAINTAINER John Doe
"#;

        let mut parser = RunefileParser::new();
        parser.parse(content);

        assert!(parser.errors.iter().any(|e| 
            e.severity == ErrorSeverity::Warning && 
            e.message.contains("deprecated")
        ));
    }
}
