//! Runefile builder for WASM

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Build instruction types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum BuildInstruction {
    From {
        image: String,
        tag: Option<String>,
        alias: Option<String>,
    },
    Run {
        command: String,
        shell: bool,
    },
    Copy {
        src: Vec<String>,
        dest: String,
        from: Option<String>,
    },
    Add {
        src: Vec<String>,
        dest: String,
    },
    Cmd {
        command: Vec<String>,
        shell: bool,
    },
    Entrypoint {
        command: Vec<String>,
        shell: bool,
    },
    Env {
        key: String,
        value: String,
    },
    Arg {
        name: String,
        default: Option<String>,
    },
    Workdir {
        path: String,
    },
    User {
        user: String,
        group: Option<String>,
    },
    Expose {
        port: u16,
        protocol: String,
    },
    Volume {
        paths: Vec<String>,
    },
    Label {
        labels: HashMap<String, String>,
    },
    Healthcheck {
        cmd: Option<String>,
        interval: Option<String>,
        timeout: Option<String>,
        retries: Option<u32>,
    },
    Stopsignal {
        signal: String,
    },
    Shell {
        shell: Vec<String>,
    },
}

/// Build stage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildStage {
    pub name: Option<String>,
    pub base_image: String,
    pub base_tag: Option<String>,
    pub instructions: Vec<BuildInstruction>,
}

/// Parsed Runefile
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedRunefile {
    pub stages: Vec<BuildStage>,
}

/// Runefile builder for WASM
#[wasm_bindgen]
pub struct RunefileBuilder {
    #[wasm_bindgen(skip)]
    pub build_args: HashMap<String, String>,
}

#[wasm_bindgen]
impl RunefileBuilder {
    /// Create a new Runefile builder
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            build_args: HashMap::new(),
        }
    }

    /// Set a build argument
    #[wasm_bindgen(js_name = setBuildArg)]
    pub fn set_build_arg(&mut self, key: &str, value: &str) {
        self.build_args.insert(key.to_string(), value.to_string());
    }

    /// Parse a Runefile
    #[wasm_bindgen]
    pub fn parse(&self, content: &str) -> String {
        match Self::parse_content(content) {
            Ok(parsed) => serde_json::to_string(&parsed).unwrap_or_else(|_| "null".to_string()),
            Err(e) => serde_json::json!({ "error": e }).to_string(),
        }
    }

    /// Validate a Runefile
    #[wasm_bindgen]
    pub fn validate(&self, content: &str) -> String {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        match Self::parse_content(content) {
            Ok(parsed) => {
                if parsed.stages.is_empty() {
                    errors.push("Runefile must have at least one stage".to_string());
                }

                for (i, stage) in parsed.stages.iter().enumerate() {
                    if stage.base_image.is_empty() {
                        errors.push(format!("Stage {} has empty base image", i));
                    }

                    for instruction in &stage.instructions {
                        match instruction {
                            BuildInstruction::Copy { src, dest, .. }
                            | BuildInstruction::Add { src, dest, .. } => {
                                if src.is_empty() {
                                    errors.push("COPY/ADD has no source files".to_string());
                                }
                                if dest.is_empty() {
                                    errors.push("COPY/ADD has no destination".to_string());
                                }
                            }
                            BuildInstruction::Workdir { path } => {
                                if !path.starts_with('/') && !path.starts_with('$') {
                                    warnings.push(format!("WORKDIR '{}' should be absolute", path));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(e) => errors.push(e),
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

impl RunefileBuilder {
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

    fn parse_instruction(line: &str, line_num: usize) -> Result<BuildInstruction, String> {
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        let instruction = parts[0].to_uppercase();
        let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match instruction.as_str() {
            "FROM" => {
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
            "RUN" => Ok(BuildInstruction::Run {
                command: args.to_string(),
                shell: !args.starts_with('['),
            }),
            "COPY" => {
                let parts: Vec<&str> = args.split_whitespace().collect();
                let from = if args.starts_with("--from=") {
                    parts
                        .first()
                        .and_then(|p| p.strip_prefix("--from="))
                        .map(|s| s.to_string())
                } else {
                    None
                };
                let filtered: Vec<&str> = parts
                    .iter()
                    .filter(|p| !p.starts_with("--"))
                    .copied()
                    .collect();
                let dest = filtered.last().map(|s| s.to_string()).unwrap_or_default();
                let src: Vec<String> = filtered
                    .iter()
                    .take(filtered.len().saturating_sub(1))
                    .map(|s| s.to_string())
                    .collect();
                Ok(BuildInstruction::Copy { src, dest, from })
            }
            "ADD" => {
                let parts: Vec<&str> = args.split_whitespace().collect();
                let dest = parts.last().map(|s| s.to_string()).unwrap_or_default();
                let src: Vec<String> = parts
                    .iter()
                    .take(parts.len().saturating_sub(1))
                    .map(|s| s.to_string())
                    .collect();
                Ok(BuildInstruction::Add { src, dest })
            }
            "CMD" => {
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
            "ENTRYPOINT" => {
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
            "ENV" => {
                if let Some(eq_pos) = args.find('=') {
                    let key = args[..eq_pos].trim().to_string();
                    let value = args[eq_pos + 1..].trim().trim_matches('"').to_string();
                    Ok(BuildInstruction::Env { key, value })
                } else {
                    let parts: Vec<&str> = args.splitn(2, char::is_whitespace).collect();
                    if parts.len() < 2 {
                        return Err(format!("Line {}: ENV requires key and value", line_num));
                    }
                    Ok(BuildInstruction::Env {
                        key: parts[0].to_string(),
                        value: parts[1].trim().to_string(),
                    })
                }
            }
            "ARG" => {
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
            "WORKDIR" => Ok(BuildInstruction::Workdir {
                path: args.to_string(),
            }),
            "USER" => {
                let parts: Vec<&str> = args.splitn(2, ':').collect();
                Ok(BuildInstruction::User {
                    user: parts[0].to_string(),
                    group: parts.get(1).map(|s| s.to_string()),
                })
            }
            "EXPOSE" => {
                let parts: Vec<&str> = args.split('/').collect();
                let port: u16 = parts[0]
                    .parse()
                    .map_err(|_| format!("Line {}: Invalid port", line_num))?;
                let protocol = parts.get(1).unwrap_or(&"tcp").to_string();
                Ok(BuildInstruction::Expose { port, protocol })
            }
            "VOLUME" => {
                let paths = if args.starts_with('[') {
                    serde_json::from_str(args).unwrap_or_default()
                } else {
                    args.split_whitespace().map(|s| s.to_string()).collect()
                };
                Ok(BuildInstruction::Volume { paths })
            }
            "LABEL" => {
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
            "HEALTHCHECK" => {
                if args.trim().to_uppercase() == "NONE" {
                    return Ok(BuildInstruction::Healthcheck {
                        cmd: None,
                        interval: None,
                        timeout: None,
                        retries: None,
                    });
                }
                let mut cmd = None;
                let mut interval = None;
                let mut timeout = None;
                let mut retries = None;
                let parts: Vec<&str> = args.split_whitespace().collect();
                let mut i = 0;
                while i < parts.len() {
                    if parts[i].starts_with("--interval=") {
                        interval = Some(parts[i][11..].to_string());
                    } else if parts[i].starts_with("--timeout=") {
                        timeout = Some(parts[i][10..].to_string());
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
                    retries,
                })
            }
            "STOPSIGNAL" => Ok(BuildInstruction::Stopsignal {
                signal: args.to_string(),
            }),
            "SHELL" => {
                let shell: Vec<String> = serde_json::from_str(args)
                    .map_err(|_| format!("Line {}: SHELL requires JSON array", line_num))?;
                Ok(BuildInstruction::Shell { shell })
            }
            _ => Err(format!(
                "Line {}: Unknown instruction: {}",
                line_num, instruction
            )),
        }
    }
}

impl Default for RunefileBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runefile_builder() {
        let builder = RunefileBuilder::new();
        let content = "FROM alpine:latest\nRUN echo hello\nCMD [\"sh\"]";
        let result = builder.parse(content);
        assert!(!result.contains("error"));
    }

    #[test]
    fn test_runefile_validation() {
        let builder = RunefileBuilder::new();
        let content = "FROM alpine\nWORKDIR relative/path";
        let result = builder.validate(content);
        assert!(result.contains("should be absolute"));
    }
}
