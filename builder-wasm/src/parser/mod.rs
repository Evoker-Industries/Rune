//! Runefile parser for WASM builder

use crate::types::{BuildInstruction, BuildStage, ParsedRunefile};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Runefile parser
#[wasm_bindgen]
pub struct RunefileParser;

#[wasm_bindgen]
impl RunefileParser {
    /// Create a new parser
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self
    }

    /// Parse Runefile content
    #[wasm_bindgen]
    pub fn parse(&self, content: &str) -> String {
        match Self::parse_content(content) {
            Ok(parsed) => serde_json::to_string(&parsed).unwrap_or_else(|_| "null".to_string()),
            Err(e) => serde_json::json!({ "error": e }).to_string(),
        }
    }

    /// Validate Runefile content
    #[wasm_bindgen]
    pub fn validate(&self, content: &str) -> String {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        match Self::parse_content(content) {
            Ok(parsed) => {
                if parsed.stages.is_empty() {
                    errors.push(
                        "Runefile must have at least one stage (FROM instruction)".to_string(),
                    );
                }

                for (i, stage) in parsed.stages.iter().enumerate() {
                    if stage.base_image.is_empty() {
                        errors.push(format!("Stage {} has empty base image", i));
                    }

                    for instruction in &stage.instructions {
                        match instruction {
                            BuildInstruction::Copy { src, dest, .. } => {
                                if src.is_empty() {
                                    errors.push("COPY instruction has no source files".to_string());
                                }
                                if dest.is_empty() {
                                    errors.push("COPY instruction has no destination".to_string());
                                }
                            }
                            BuildInstruction::Add { src, dest, .. } => {
                                if src.is_empty() {
                                    errors.push("ADD instruction has no source files".to_string());
                                }
                                if dest.is_empty() {
                                    errors.push("ADD instruction has no destination".to_string());
                                }
                            }
                            BuildInstruction::Expose { port, .. } => {
                                if *port == 0 {
                                    warnings.push("EXPOSE port 0 is unusual".to_string());
                                }
                            }
                            BuildInstruction::Workdir { path } => {
                                if !path.starts_with('/') && !path.starts_with('$') {
                                    warnings.push(format!(
                                        "WORKDIR '{}' should be an absolute path",
                                        path
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(e) => {
                errors.push(e);
            }
        }

        serde_json::json!({
            "valid": errors.is_empty(),
            "errors": errors,
            "warnings": warnings
        })
        .to_string()
    }

    /// Get the default build file name
    #[wasm_bindgen(js_name = getDefaultBuildFile)]
    pub fn get_default_build_file() -> String {
        "Runefile".to_string()
    }
}

impl Default for RunefileParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RunefileParser {
    /// Parse Runefile content
    pub fn parse_content(content: &str) -> Result<ParsedRunefile, String> {
        let mut stages = Vec::new();
        let mut current_stage: Option<BuildStage> = None;
        let mut continued_line = String::new();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.ends_with('\\') {
                continued_line.push_str(&line[..line.len() - 1]);
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
                    if let Some(stage) = current_stage.take() {
                        stages.push(stage);
                    }
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
                        return Err(format!("Line {}: Instruction before FROM", line_num + 1));
                    }
                }
            }
        }

        if let Some(stage) = current_stage {
            stages.push(stage);
        }

        if stages.is_empty() {
            return Err("No FROM instruction found".to_string());
        }

        Ok(ParsedRunefile { stages })
    }

    /// Parse a single instruction
    fn parse_instruction(line: &str, line_num: usize) -> Result<BuildInstruction, String> {
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
            _ => Err(format!(
                "Line {}: Unknown instruction: {}",
                line_num, instruction
            )),
        }
    }

    fn parse_from(args: &str, line_num: usize) -> Result<BuildInstruction, String> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        if parts.is_empty() {
            return Err(format!("Line {}: FROM requires an image", line_num));
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

    fn parse_run(args: &str) -> Result<BuildInstruction, String> {
        if args.starts_with('[') {
            Ok(BuildInstruction::Run {
                command: args.to_string(),
                shell: false,
            })
        } else {
            Ok(BuildInstruction::Run {
                command: args.to_string(),
                shell: true,
            })
        }
    }

    fn parse_copy(args: &str) -> Result<BuildInstruction, String> {
        let mut from = None;
        let mut chown = None;
        let mut remaining = args;

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

    fn parse_add(args: &str) -> Result<BuildInstruction, String> {
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

    fn parse_cmd(args: &str) -> Result<BuildInstruction, String> {
        if args.starts_with('[') {
            let command: Vec<String> = serde_json::from_str(args).unwrap_or_default();
            Ok(BuildInstruction::Cmd {
                command,
                shell: false,
            })
        } else {
            Ok(BuildInstruction::Cmd {
                command: vec![args.to_string()],
                shell: true,
            })
        }
    }

    fn parse_entrypoint(args: &str) -> Result<BuildInstruction, String> {
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

    fn parse_env(args: &str, line_num: usize) -> Result<BuildInstruction, String> {
        if let Some(eq_pos) = args.find('=') {
            let key = args[..eq_pos].trim().to_string();
            let value = args[eq_pos + 1..].trim().trim_matches('"').to_string();
            Ok(BuildInstruction::Env { key, value })
        } else {
            let parts: Vec<&str> = args.splitn(2, char::is_whitespace).collect();
            if parts.len() < 2 {
                return Err(format!("Line {}: ENV requires a key and value", line_num));
            }
            Ok(BuildInstruction::Env {
                key: parts[0].to_string(),
                value: parts[1].trim().to_string(),
            })
        }
    }

    fn parse_arg(args: &str) -> Result<BuildInstruction, String> {
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

    fn parse_user(args: &str) -> Result<BuildInstruction, String> {
        let parts: Vec<&str> = args.splitn(2, ':').collect();
        Ok(BuildInstruction::User {
            user: parts[0].to_string(),
            group: parts.get(1).map(|s| s.to_string()),
        })
    }

    fn parse_expose(args: &str, line_num: usize) -> Result<BuildInstruction, String> {
        let parts: Vec<&str> = args.split('/').collect();
        let port: u16 = parts[0]
            .parse()
            .map_err(|_| format!("Line {}: Invalid port number: {}", line_num, parts[0]))?;
        let protocol = parts.get(1).unwrap_or(&"tcp").to_string();

        Ok(BuildInstruction::Expose { port, protocol })
    }

    fn parse_volume(args: &str) -> Result<BuildInstruction, String> {
        let paths = if args.starts_with('[') {
            serde_json::from_str(args).unwrap_or_default()
        } else {
            args.split_whitespace().map(|s| s.to_string()).collect()
        };

        Ok(BuildInstruction::Volume { paths })
    }

    fn parse_label(args: &str) -> Result<BuildInstruction, String> {
        let mut labels = HashMap::new();

        for part in args.split_whitespace() {
            if let Some(eq_pos) = part.find('=') {
                let key = part[..eq_pos].to_string();
                let value = part[eq_pos + 1..].trim_matches('"').to_string();
                labels.insert(key, value);
            }
        }

        Ok(BuildInstruction::Label { labels })
    }

    fn parse_healthcheck(args: &str) -> Result<BuildInstruction, String> {
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

    fn parse_shell(args: &str, line_num: usize) -> Result<BuildInstruction, String> {
        let shell: Vec<String> = serde_json::from_str(args)
            .map_err(|_| format!("Line {}: SHELL requires JSON array format", line_num))?;

        Ok(BuildInstruction::Shell { shell })
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

        let parsed = RunefileParser::parse_content(content).unwrap();
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

        let parsed = RunefileParser::parse_content(content).unwrap();
        assert_eq!(parsed.stages.len(), 2);
        assert_eq!(parsed.stages[0].name, Some("builder".to_string()));
        assert_eq!(parsed.stages[1].base_image, "debian");
    }

    #[test]
    fn test_default_build_file() {
        assert_eq!(RunefileParser::get_default_build_file(), "Runefile");
    }
}
