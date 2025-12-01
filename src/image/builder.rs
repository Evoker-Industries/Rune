//! Image builder - Supports building images from Runefile (or Dockerfile)
//!
//! Runefile is the default build file format for Rune, but Dockerfile
//! syntax is also supported for Docker compatibility.

use crate::error::{Result, RuneError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Default build file name
pub const DEFAULT_BUILD_FILE: &str = "Runefile";

/// Alternative build file name (Docker compatibility)
pub const DOCKERFILE_NAME: &str = "Dockerfile";

/// Build context for image building
#[derive(Debug, Clone)]
pub struct BuildContext {
    /// Context directory
    pub context_dir: PathBuf,
    /// Build file path
    pub build_file: PathBuf,
    /// Build arguments
    pub build_args: HashMap<String, String>,
    /// Target stage (for multi-stage builds)
    pub target: Option<String>,
    /// No cache
    pub no_cache: bool,
    /// Pull latest base image
    pub pull: bool,
    /// Tags for the built image
    pub tags: Vec<String>,
    /// Labels for the built image
    pub labels: HashMap<String, String>,
}

impl BuildContext {
    /// Create a new build context
    pub fn new(context_dir: PathBuf) -> Self {
        // Look for Runefile first, then Dockerfile
        let build_file = if context_dir.join(DEFAULT_BUILD_FILE).exists() {
            context_dir.join(DEFAULT_BUILD_FILE)
        } else if context_dir.join(DOCKERFILE_NAME).exists() {
            context_dir.join(DOCKERFILE_NAME)
        } else {
            context_dir.join(DEFAULT_BUILD_FILE)
        };

        Self {
            context_dir,
            build_file,
            build_args: HashMap::new(),
            target: None,
            no_cache: false,
            pull: false,
            tags: Vec::new(),
            labels: HashMap::new(),
        }
    }

    /// Set build file path
    pub fn build_file(mut self, path: PathBuf) -> Self {
        self.build_file = path;
        self
    }

    /// Add build argument
    pub fn arg(mut self, key: &str, value: &str) -> Self {
        self.build_args.insert(key.to_string(), value.to_string());
        self
    }

    /// Set target stage
    pub fn target(mut self, target: &str) -> Self {
        self.target = Some(target.to_string());
        self
    }

    /// Add tag
    pub fn tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    /// Add label
    pub fn label(mut self, key: &str, value: &str) -> Self {
        self.labels.insert(key.to_string(), value.to_string());
        self
    }
}

/// Parsed build instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildInstruction {
    /// FROM instruction - base image
    From {
        image: String,
        tag: Option<String>,
        alias: Option<String>,
    },
    /// RUN instruction - execute command
    Run { command: String, shell: bool },
    /// COPY instruction - copy files
    Copy {
        src: Vec<String>,
        dest: String,
        from: Option<String>,
        chown: Option<String>,
    },
    /// ADD instruction - add files (with URL/archive support)
    Add {
        src: Vec<String>,
        dest: String,
        chown: Option<String>,
    },
    /// CMD instruction - default command
    Cmd { command: Vec<String>, shell: bool },
    /// ENTRYPOINT instruction
    Entrypoint { command: Vec<String>, shell: bool },
    /// ENV instruction - set environment variable
    Env { key: String, value: String },
    /// ARG instruction - build argument
    Arg {
        name: String,
        default: Option<String>,
    },
    /// WORKDIR instruction - set working directory
    Workdir { path: String },
    /// USER instruction - set user
    User { user: String, group: Option<String> },
    /// EXPOSE instruction - expose port
    Expose { port: u16, protocol: String },
    /// VOLUME instruction - create volume mount point
    Volume { paths: Vec<String> },
    /// LABEL instruction - add metadata
    Label { labels: HashMap<String, String> },
    /// HEALTHCHECK instruction
    Healthcheck {
        cmd: Option<String>,
        interval: Option<String>,
        timeout: Option<String>,
        start_period: Option<String>,
        retries: Option<u32>,
    },
    /// STOPSIGNAL instruction
    Stopsignal { signal: String },
    /// SHELL instruction - set default shell
    Shell { shell: Vec<String> },
    /// ONBUILD instruction - trigger for child builds
    Onbuild { instruction: Box<BuildInstruction> },
}

/// Parsed build file (Runefile or Dockerfile)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedBuildFile {
    /// Build stages
    pub stages: Vec<BuildStage>,
}

/// Build stage (for multi-stage builds)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildStage {
    /// Stage name/alias
    pub name: Option<String>,
    /// Base image
    pub base_image: String,
    /// Base image tag
    pub base_tag: Option<String>,
    /// Instructions in this stage
    pub instructions: Vec<BuildInstruction>,
}

/// Image builder
pub struct ImageBuilder {
    /// Build context
    context: BuildContext,
}

impl ImageBuilder {
    /// Create a new image builder
    pub fn new(context: BuildContext) -> Self {
        Self { context }
    }

    /// Parse a build file (Runefile or Dockerfile)
    pub fn parse_build_file(path: &Path) -> Result<ParsedBuildFile> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_build_content(&content)
    }

    /// Parse build file content
    pub fn parse_build_content(content: &str) -> Result<ParsedBuildFile> {
        let mut stages = Vec::new();
        let mut current_stage: Option<BuildStage> = None;
        let mut continued_line = String::new();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Handle line continuation
            if let Some(line_without_backslash) = line.strip_suffix('\\') {
                continued_line.push_str(line_without_backslash);
                continued_line.push(' ');
                continue;
            }

            let full_line = if !continued_line.is_empty() {
                let result = format!("{}{}", continued_line, line);
                continued_line.clear();
                result
            } else {
                line.to_string()
            };

            let instruction = Self::parse_instruction(&full_line, line_num + 1)?;

            match instruction {
                BuildInstruction::From { image, tag, alias } => {
                    // Save current stage if exists
                    if let Some(stage) = current_stage.take() {
                        stages.push(stage);
                    }

                    // Start new stage
                    current_stage = Some(BuildStage {
                        name: alias,
                        base_image: image,
                        base_tag: tag,
                        instructions: Vec::new(),
                    });
                }
                _ => {
                    if let Some(ref mut stage) = current_stage {
                        stage.instructions.push(instruction);
                    } else {
                        return Err(RuneError::DockerfileParse {
                            line: line_num + 1,
                            message: "Instruction before FROM".to_string(),
                        });
                    }
                }
            }
        }

        // Save final stage
        if let Some(stage) = current_stage {
            stages.push(stage);
        }

        if stages.is_empty() {
            return Err(RuneError::DockerfileParse {
                line: 0,
                message: "No FROM instruction found".to_string(),
            });
        }

        Ok(ParsedBuildFile { stages })
    }

    /// Parse a single instruction
    fn parse_instruction(line: &str, line_num: usize) -> Result<BuildInstruction> {
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        let instruction = parts[0].to_uppercase();
        let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match instruction.as_str() {
            "FROM" => Self::parse_from(args, line_num),
            "RUN" => Self::parse_run(args),
            "COPY" => Self::parse_copy(args),
            "ADD" => Self::parse_add(args),
            "CMD" => Self::parse_cmd(args),
            "ENTRYPOINT" => Self::parse_entrypoint(args),
            "ENV" => Self::parse_env(args, line_num),
            "ARG" => Self::parse_arg(args),
            "WORKDIR" => Ok(BuildInstruction::Workdir {
                path: args.to_string(),
            }),
            "USER" => Self::parse_user(args),
            "EXPOSE" => Self::parse_expose(args, line_num),
            "VOLUME" => Self::parse_volume(args),
            "LABEL" => Self::parse_label(args),
            "HEALTHCHECK" => Self::parse_healthcheck(args),
            "STOPSIGNAL" => Ok(BuildInstruction::Stopsignal {
                signal: args.to_string(),
            }),
            "SHELL" => Self::parse_shell(args, line_num),
            "ONBUILD" => {
                let inner = Self::parse_instruction(args, line_num)?;
                Ok(BuildInstruction::Onbuild {
                    instruction: Box::new(inner),
                })
            }
            _ => Err(RuneError::DockerfileParse {
                line: line_num,
                message: format!("Unknown instruction: {}", instruction),
            }),
        }
    }

    fn parse_from(args: &str, line_num: usize) -> Result<BuildInstruction> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        if parts.is_empty() {
            return Err(RuneError::DockerfileParse {
                line: line_num,
                message: "FROM requires an image".to_string(),
            });
        }

        let image_parts: Vec<&str> = parts[0].splitn(2, ':').collect();
        let image = image_parts[0].to_string();
        let tag = image_parts.get(1).map(|s| s.to_string());

        let alias = if parts.len() >= 3 && parts[1].to_uppercase() == "AS" {
            Some(parts[2].to_string())
        } else {
            None
        };

        Ok(BuildInstruction::From { image, tag, alias })
    }

    fn parse_run(args: &str) -> Result<BuildInstruction> {
        if args.starts_with('[') {
            // JSON form
            Ok(BuildInstruction::Run {
                command: args.to_string(),
                shell: false,
            })
        } else {
            // Shell form
            Ok(BuildInstruction::Run {
                command: args.to_string(),
                shell: true,
            })
        }
    }

    fn parse_copy(args: &str) -> Result<BuildInstruction> {
        let mut from = None;
        let mut chown = None;
        let mut remaining = args;

        // Parse flags
        while remaining.starts_with("--") {
            if remaining.starts_with("--from=") {
                let end = remaining[7..].find(' ').unwrap_or(remaining.len() - 7);
                from = Some(remaining[7..7 + end].to_string());
                remaining = remaining[7 + end..].trim();
            } else if remaining.starts_with("--chown=") {
                let end = remaining[8..].find(' ').unwrap_or(remaining.len() - 8);
                chown = Some(remaining[8..8 + end].to_string());
                remaining = remaining[8 + end..].trim();
            } else {
                break;
            }
        }

        let parts: Vec<&str> = remaining.split_whitespace().collect();
        if parts.len() < 2 {
            return Ok(BuildInstruction::Copy {
                src: vec![],
                dest: String::new(),
                from,
                chown,
            });
        }

        let dest = parts.last().unwrap().to_string();
        let src: Vec<String> = parts[..parts.len() - 1]
            .iter()
            .map(|s| s.to_string())
            .collect();

        Ok(BuildInstruction::Copy {
            src,
            dest,
            from,
            chown,
        })
    }

    fn parse_add(args: &str) -> Result<BuildInstruction> {
        let mut chown = None;
        let mut remaining = args;

        if remaining.starts_with("--chown=") {
            let end = remaining[8..].find(' ').unwrap_or(remaining.len() - 8);
            chown = Some(remaining[8..8 + end].to_string());
            remaining = remaining[8 + end..].trim();
        }

        let parts: Vec<&str> = remaining.split_whitespace().collect();
        if parts.len() < 2 {
            return Ok(BuildInstruction::Add {
                src: vec![],
                dest: String::new(),
                chown,
            });
        }

        let dest = parts.last().unwrap().to_string();
        let src: Vec<String> = parts[..parts.len() - 1]
            .iter()
            .map(|s| s.to_string())
            .collect();

        Ok(BuildInstruction::Add { src, dest, chown })
    }

    fn parse_cmd(args: &str) -> Result<BuildInstruction> {
        if args.starts_with('[') {
            // JSON form
            let command: Vec<String> = serde_json::from_str(args).unwrap_or_default();
            Ok(BuildInstruction::Cmd {
                command,
                shell: false,
            })
        } else {
            // Shell form
            Ok(BuildInstruction::Cmd {
                command: vec![args.to_string()],
                shell: true,
            })
        }
    }

    fn parse_entrypoint(args: &str) -> Result<BuildInstruction> {
        if args.starts_with('[') {
            let command: Vec<String> = serde_json::from_str(args).unwrap_or_default();
            Ok(BuildInstruction::Entrypoint {
                command,
                shell: false,
            })
        } else {
            Ok(BuildInstruction::Entrypoint {
                command: vec![args.to_string()],
                shell: true,
            })
        }
    }

    fn parse_env(args: &str, line_num: usize) -> Result<BuildInstruction> {
        // Support both ENV key=value and ENV key value
        if let Some(eq_pos) = args.find('=') {
            let key = args[..eq_pos].trim().to_string();
            let value = args[eq_pos + 1..].trim().trim_matches('"').to_string();
            Ok(BuildInstruction::Env { key, value })
        } else {
            let parts: Vec<&str> = args.splitn(2, char::is_whitespace).collect();
            if parts.len() < 2 {
                return Err(RuneError::DockerfileParse {
                    line: line_num,
                    message: "ENV requires a key and value".to_string(),
                });
            }
            Ok(BuildInstruction::Env {
                key: parts[0].to_string(),
                value: parts[1].trim().to_string(),
            })
        }
    }

    fn parse_arg(args: &str) -> Result<BuildInstruction> {
        if let Some(eq_pos) = args.find('=') {
            Ok(BuildInstruction::Arg {
                name: args[..eq_pos].trim().to_string(),
                default: Some(args[eq_pos + 1..].trim().to_string()),
            })
        } else {
            Ok(BuildInstruction::Arg {
                name: args.trim().to_string(),
                default: None,
            })
        }
    }

    fn parse_user(args: &str) -> Result<BuildInstruction> {
        let parts: Vec<&str> = args.splitn(2, ':').collect();
        Ok(BuildInstruction::User {
            user: parts[0].to_string(),
            group: parts.get(1).map(|s| s.to_string()),
        })
    }

    fn parse_expose(args: &str, line_num: usize) -> Result<BuildInstruction> {
        let parts: Vec<&str> = args.split('/').collect();
        let port: u16 = parts[0].parse().map_err(|_| RuneError::DockerfileParse {
            line: line_num,
            message: format!("Invalid port number: {}", parts[0]),
        })?;
        let protocol = parts.get(1).unwrap_or(&"tcp").to_string();

        Ok(BuildInstruction::Expose { port, protocol })
    }

    fn parse_volume(args: &str) -> Result<BuildInstruction> {
        let paths = if args.starts_with('[') {
            serde_json::from_str(args).unwrap_or_default()
        } else {
            args.split_whitespace().map(|s| s.to_string()).collect()
        };

        Ok(BuildInstruction::Volume { paths })
    }

    fn parse_label(args: &str) -> Result<BuildInstruction> {
        let mut labels = HashMap::new();

        // Parse key=value pairs
        for part in args.split_whitespace() {
            if let Some(eq_pos) = part.find('=') {
                let key = part[..eq_pos].to_string();
                let value = part[eq_pos + 1..].trim_matches('"').to_string();
                labels.insert(key, value);
            }
        }

        Ok(BuildInstruction::Label { labels })
    }

    fn parse_healthcheck(args: &str) -> Result<BuildInstruction> {
        if args.trim().to_uppercase() == "NONE" {
            return Ok(BuildInstruction::Healthcheck {
                cmd: None,
                interval: None,
                timeout: None,
                start_period: None,
                retries: None,
            });
        }

        let mut cmd = None;
        let mut interval = None;
        let mut timeout = None;
        let mut start_period = None;
        let mut retries = None;

        let parts: Vec<&str> = args.split_whitespace().collect();
        let mut i = 0;

        while i < parts.len() {
            if parts[i].starts_with("--interval=") {
                interval = Some(parts[i][11..].to_string());
            } else if parts[i].starts_with("--timeout=") {
                timeout = Some(parts[i][10..].to_string());
            } else if parts[i].starts_with("--start-period=") {
                start_period = Some(parts[i][15..].to_string());
            } else if parts[i].starts_with("--retries=") {
                retries = parts[i][10..].parse().ok();
            } else if parts[i] == "CMD" {
                cmd = Some(parts[i + 1..].join(" "));
                break;
            }
            i += 1;
        }

        Ok(BuildInstruction::Healthcheck {
            cmd,
            interval,
            timeout,
            start_period,
            retries,
        })
    }

    fn parse_shell(args: &str, line_num: usize) -> Result<BuildInstruction> {
        let shell: Vec<String> =
            serde_json::from_str(args).map_err(|_| RuneError::DockerfileParse {
                line: line_num,
                message: "SHELL requires JSON array format".to_string(),
            })?;

        Ok(BuildInstruction::Shell { shell })
    }

    /// Build an image from the build context
    pub async fn build(&self) -> Result<String> {
        // Parse the build file
        let parsed = Self::parse_build_file(&self.context.build_file)?;

        // For now, return a placeholder image ID
        // In a full implementation, this would:
        // 1. Pull base images
        // 2. Execute each instruction
        // 3. Create image layers
        // 4. Store the final image

        let image_id = uuid::Uuid::new_v4().to_string().replace("-", "")[..12].to_string();

        tracing::info!(
            "Built image {} from {} with {} stages",
            image_id,
            self.context.build_file.display(),
            parsed.stages.len()
        );

        Ok(image_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_runefile() {
        let content = r#"
FROM ubuntu:22.04

RUN apt-get update && apt-get install -y curl

WORKDIR /app

COPY . /app

CMD ["./start.sh"]
"#;

        let parsed = ImageBuilder::parse_build_content(content).unwrap();
        assert_eq!(parsed.stages.len(), 1);
        assert_eq!(parsed.stages[0].base_image, "ubuntu");
        assert_eq!(parsed.stages[0].base_tag, Some("22.04".to_string()));
        assert_eq!(parsed.stages[0].instructions.len(), 4);
    }

    #[test]
    fn test_parse_multistage_build() {
        let content = r#"
FROM rust:1.70 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/myapp /usr/local/bin/
CMD ["myapp"]
"#;

        let parsed = ImageBuilder::parse_build_content(content).unwrap();
        assert_eq!(parsed.stages.len(), 2);
        assert_eq!(parsed.stages[0].name, Some("builder".to_string()));
        assert_eq!(parsed.stages[1].base_image, "debian");
    }

    #[test]
    fn test_default_build_file_name() {
        assert_eq!(DEFAULT_BUILD_FILE, "Runefile");
    }
}
