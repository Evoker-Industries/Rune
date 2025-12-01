//! Runefile Builder - WebAssembly Version with Bring-Your-Own-Filesystem
//!
//! This module provides a WASM-compatible image builder for Runefiles that allows
//! users to provide their own filesystem implementation via JavaScript callbacks.
//!
//! ## Usage
//!
//! ```javascript
//! import init, { WasmBuilder, BuilderFilesystem } from 'runefile-builder-wasm';
//!
//! // Create a filesystem adapter
//! const fs = new BuilderFilesystem();
//! fs.setReadFile((path) => {
//!     // Return file contents as Uint8Array or null if not found
//!     return myFilesystem.readFile(path);
//! });
//! fs.setWriteFile((path, contents) => {
//!     // Write contents (Uint8Array) to path
//!     myFilesystem.writeFile(path, contents);
//! });
//! fs.setListDir((path) => {
//!     // Return array of {name, isDir} objects
//!     return myFilesystem.listDir(path);
//! });
//! fs.setExists((path) => {
//!     return myFilesystem.exists(path);
//! });
//! fs.setMkdir((path) => {
//!     myFilesystem.mkdir(path);
//! });
//! fs.setStat((path) => {
//!     // Return {size, isDir, mode} or null
//!     return myFilesystem.stat(path);
//! });
//!
//! // Create the builder
//! const builder = new WasmBuilder(fs);
//!
//! // Parse a Runefile
//! const parsed = builder.parseRunefile(runefileContent);
//!
//! // Build an image
//! const result = builder.build({
//!     contextDir: '/project',
//!     buildFile: '/project/Runefile',
//!     tags: ['myapp:latest'],
//!     buildArgs: { VERSION: '1.0.0' },
//! });
//!
//! console.log(result.imageId);
//! console.log(result.layers);
//! ```

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

// Re-export common types
pub use wasm_bindgen::JsValue;

/// File entry returned by list_dir
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
}

/// File stat result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStat {
    pub size: u64,
    pub is_dir: bool,
    pub mode: u32,
}

/// Filesystem interface for WASM
/// Users implement this via JavaScript callbacks
#[wasm_bindgen]
pub struct BuilderFilesystem {
    #[wasm_bindgen(skip)]
    pub read_file: Option<js_sys::Function>,
    #[wasm_bindgen(skip)]
    pub write_file: Option<js_sys::Function>,
    #[wasm_bindgen(skip)]
    pub list_dir: Option<js_sys::Function>,
    #[wasm_bindgen(skip)]
    pub exists: Option<js_sys::Function>,
    #[wasm_bindgen(skip)]
    pub mkdir: Option<js_sys::Function>,
    #[wasm_bindgen(skip)]
    pub stat: Option<js_sys::Function>,
    #[wasm_bindgen(skip)]
    pub remove: Option<js_sys::Function>,
    #[wasm_bindgen(skip)]
    pub copy: Option<js_sys::Function>,
}

#[wasm_bindgen]
impl BuilderFilesystem {
    /// Create a new filesystem interface
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            read_file: None,
            write_file: None,
            list_dir: None,
            exists: None,
            mkdir: None,
            stat: None,
            remove: None,
            copy: None,
        }
    }

    /// Set the read_file callback: (path: string) => Uint8Array | null
    #[wasm_bindgen(js_name = setReadFile)]
    pub fn set_read_file(&mut self, callback: js_sys::Function) {
        self.read_file = Some(callback);
    }

    /// Set the write_file callback: (path: string, contents: Uint8Array) => void
    #[wasm_bindgen(js_name = setWriteFile)]
    pub fn set_write_file(&mut self, callback: js_sys::Function) {
        self.write_file = Some(callback);
    }

    /// Set the list_dir callback: (path: string) => Array<{name: string, isDir: boolean}>
    #[wasm_bindgen(js_name = setListDir)]
    pub fn set_list_dir(&mut self, callback: js_sys::Function) {
        self.list_dir = Some(callback);
    }

    /// Set the exists callback: (path: string) => boolean
    #[wasm_bindgen(js_name = setExists)]
    pub fn set_exists(&mut self, callback: js_sys::Function) {
        self.exists = Some(callback);
    }

    /// Set the mkdir callback: (path: string) => void
    #[wasm_bindgen(js_name = setMkdir)]
    pub fn set_mkdir(&mut self, callback: js_sys::Function) {
        self.mkdir = Some(callback);
    }

    /// Set the stat callback: (path: string) => {size: number, isDir: boolean, mode: number} | null
    #[wasm_bindgen(js_name = setStat)]
    pub fn set_stat(&mut self, callback: js_sys::Function) {
        self.stat = Some(callback);
    }

    /// Set the remove callback: (path: string) => void
    #[wasm_bindgen(js_name = setRemove)]
    pub fn set_remove(&mut self, callback: js_sys::Function) {
        self.remove = Some(callback);
    }

    /// Set the copy callback: (src: string, dest: string) => void
    #[wasm_bindgen(js_name = setCopy)]
    pub fn set_copy(&mut self, callback: js_sys::Function) {
        self.copy = Some(callback);
    }
}

impl Default for BuilderFilesystem {
    fn default() -> Self {
        Self::new()
    }
}

impl BuilderFilesystem {
    /// Read a file from the filesystem
    pub fn read_file_impl(&self, path: &str) -> Option<Vec<u8>> {
        let callback = self.read_file.as_ref()?;
        let this = JsValue::null();
        let arg = JsValue::from_str(path);
        
        match callback.call1(&this, &arg) {
            Ok(result) => {
                if result.is_null() || result.is_undefined() {
                    None
                } else if let Some(array) = result.dyn_ref::<js_sys::Uint8Array>() {
                    Some(array.to_vec())
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Write a file to the filesystem
    pub fn write_file_impl(&self, path: &str, contents: &[u8]) -> bool {
        let callback = match &self.write_file {
            Some(cb) => cb,
            None => return false,
        };
        
        let this = JsValue::null();
        let path_arg = JsValue::from_str(path);
        let contents_arg = js_sys::Uint8Array::from(contents);
        
        callback.call2(&this, &path_arg, &contents_arg).is_ok()
    }

    /// List directory contents
    pub fn list_dir_impl(&self, path: &str) -> Option<Vec<FileEntry>> {
        let callback = self.list_dir.as_ref()?;
        let this = JsValue::null();
        let arg = JsValue::from_str(path);
        
        match callback.call1(&this, &arg) {
            Ok(result) => {
                if result.is_null() || result.is_undefined() {
                    None
                } else {
                    serde_wasm_bindgen::from_value(result).ok()
                }
            }
            Err(_) => None,
        }
    }

    /// Check if a path exists
    pub fn exists_impl(&self, path: &str) -> bool {
        let callback = match &self.exists {
            Some(cb) => cb,
            None => return false,
        };
        
        let this = JsValue::null();
        let arg = JsValue::from_str(path);
        
        match callback.call1(&this, &arg) {
            Ok(result) => result.as_bool().unwrap_or(false),
            Err(_) => false,
        }
    }

    /// Create a directory
    pub fn mkdir_impl(&self, path: &str) -> bool {
        let callback = match &self.mkdir {
            Some(cb) => cb,
            None => return false,
        };
        
        let this = JsValue::null();
        let arg = JsValue::from_str(path);
        
        callback.call1(&this, &arg).is_ok()
    }

    /// Get file stats
    pub fn stat_impl(&self, path: &str) -> Option<FileStat> {
        let callback = self.stat.as_ref()?;
        let this = JsValue::null();
        let arg = JsValue::from_str(path);
        
        match callback.call1(&this, &arg) {
            Ok(result) => {
                if result.is_null() || result.is_undefined() {
                    None
                } else {
                    serde_wasm_bindgen::from_value(result).ok()
                }
            }
            Err(_) => None,
        }
    }

    /// Remove a file or directory
    pub fn remove_impl(&self, path: &str) -> bool {
        let callback = match &self.remove {
            Some(cb) => cb,
            None => return false,
        };
        
        let this = JsValue::null();
        let arg = JsValue::from_str(path);
        
        callback.call1(&this, &arg).is_ok()
    }

    /// Copy a file
    pub fn copy_impl(&self, src: &str, dest: &str) -> bool {
        let callback = match &self.copy {
            Some(cb) => cb,
            None => return false,
        };
        
        let this = JsValue::null();
        let src_arg = JsValue::from_str(src);
        let dest_arg = JsValue::from_str(dest);
        
        callback.call2(&this, &src_arg, &dest_arg).is_ok()
    }
}

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
        chown: Option<String>,
    },
    Add {
        src: Vec<String>,
        dest: String,
        chown: Option<String>,
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
        start_period: Option<String>,
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

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildConfig {
    pub context_dir: String,
    pub build_file: Option<String>,
    pub tags: Vec<String>,
    pub build_args: HashMap<String, String>,
    pub target: Option<String>,
    pub no_cache: bool,
    pub labels: HashMap<String, String>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            context_dir: ".".to_string(),
            build_file: None,
            tags: Vec::new(),
            build_args: HashMap::new(),
            target: None,
            no_cache: false,
            labels: HashMap::new(),
        }
    }
}

/// Image layer
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageLayer {
    pub id: String,
    pub digest: String,
    pub size: u64,
    pub created_by: String,
    pub empty_layer: bool,
}

/// Build result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildResult {
    pub success: bool,
    pub image_id: Option<String>,
    pub layers: Vec<ImageLayer>,
    pub config: Option<ImageConfig>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Image configuration (OCI config)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageConfig {
    pub architecture: String,
    pub os: String,
    pub config: ContainerConfig,
    pub rootfs: RootFs,
    pub history: Vec<HistoryEntry>,
}

/// Container configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerConfig {
    pub hostname: String,
    pub user: String,
    pub env: Vec<String>,
    pub cmd: Vec<String>,
    pub entrypoint: Vec<String>,
    pub working_dir: String,
    pub labels: HashMap<String, String>,
    pub exposed_ports: HashMap<String, serde_json::Value>,
    pub volumes: HashMap<String, serde_json::Value>,
    pub stop_signal: String,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            hostname: String::new(),
            user: String::new(),
            env: Vec::new(),
            cmd: Vec::new(),
            entrypoint: Vec::new(),
            working_dir: String::new(),
            labels: HashMap::new(),
            exposed_ports: HashMap::new(),
            volumes: HashMap::new(),
            stop_signal: "SIGTERM".to_string(),
        }
    }
}

/// Root filesystem definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootFs {
    #[serde(rename = "type")]
    pub fs_type: String,
    pub diff_ids: Vec<String>,
}

/// History entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HistoryEntry {
    pub created: String,
    pub created_by: String,
    pub empty_layer: bool,
    pub comment: Option<String>,
}

/// Build event for progress reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum BuildEvent {
    StageStart { stage: usize, name: Option<String>, base: String },
    StepStart { step: usize, instruction: String },
    StepComplete { step: usize, layer_id: Option<String> },
    StageComplete { stage: usize },
    BuildComplete { image_id: String },
    Error { message: String },
    Warning { message: String },
    Progress { message: String, percent: Option<u8> },
}

/// WASM Image Builder
#[wasm_bindgen]
pub struct WasmBuilder {
    #[wasm_bindgen(skip)]
    pub fs: BuilderFilesystem,
    #[wasm_bindgen(skip)]
    pub progress_callback: Option<js_sys::Function>,
}

#[wasm_bindgen]
impl WasmBuilder {
    /// Create a new WASM builder with a filesystem implementation
    #[wasm_bindgen(constructor)]
    pub fn new(fs: BuilderFilesystem) -> Self {
        Self {
            fs,
            progress_callback: None,
        }
    }

    /// Set the progress callback: (event: BuildEvent) => void
    #[wasm_bindgen(js_name = setProgressCallback)]
    pub fn set_progress_callback(&mut self, callback: js_sys::Function) {
        self.progress_callback = Some(callback);
    }

    /// Parse a Runefile and return the parsed structure as JSON
    #[wasm_bindgen(js_name = parseRunefile)]
    pub fn parse_runefile(&self, content: &str) -> String {
        match Self::parse_content(content) {
            Ok(parsed) => serde_json::to_string(&parsed).unwrap_or_else(|_| "null".to_string()),
            Err(e) => serde_json::json!({ "error": e }).to_string(),
        }
    }

    /// Parse a Runefile from a path using the filesystem
    #[wasm_bindgen(js_name = parseRunefileFromPath)]
    pub fn parse_runefile_from_path(&self, path: &str) -> String {
        let content = match self.fs.read_file_impl(path) {
            Some(bytes) => match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(_) => return serde_json::json!({ "error": "Invalid UTF-8 in file" }).to_string(),
            },
            None => return serde_json::json!({ "error": format!("File not found: {}", path) }).to_string(),
        };

        self.parse_runefile(&content)
    }

    /// Build an image from configuration (JSON)
    #[wasm_bindgen]
    pub fn build(&mut self, config_json: &str) -> String {
        let config: BuildConfig = match serde_json::from_str(config_json) {
            Ok(c) => c,
            Err(e) => {
                return serde_json::to_string(&BuildResult {
                    success: false,
                    image_id: None,
                    layers: Vec::new(),
                    config: None,
                    errors: vec![format!("Invalid config: {}", e)],
                    warnings: Vec::new(),
                })
                .unwrap_or_default();
            }
        };

        self.build_impl(config)
    }

    /// Validate a Runefile content
    #[wasm_bindgen]
    pub fn validate(&self, content: &str) -> String {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        match Self::parse_content(content) {
            Ok(parsed) => {
                // Validate stages
                if parsed.stages.is_empty() {
                    errors.push("Runefile must have at least one stage (FROM instruction)".to_string());
                }

                for (i, stage) in parsed.stages.iter().enumerate() {
                    // Check base image
                    if stage.base_image.is_empty() {
                        errors.push(format!("Stage {} has empty base image", i));
                    }

                    // Validate instructions
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
                                    warnings.push(format!("WORKDIR '{}' should be an absolute path", path));
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

    /// Calculate the digest of content
    #[wasm_bindgen(js_name = calculateDigest)]
    pub fn calculate_digest(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let result = hasher.finalize();
        format!("sha256:{}", hex::encode(result))
    }
}

impl WasmBuilder {
    /// Parse Runefile content
    fn parse_content(content: &str) -> Result<ParsedRunefile, String> {
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
                        return Err(format!("Line {}: Instruction before FROM", line_num + 1));
                    }
                }
            }
        }

        // Save final stage
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
            _ => Err(format!("Line {}: Unknown instruction: {}", line_num, instruction)),
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
        let port: u16 = parts[0].parse().map_err(|_| {
            format!("Line {}: Invalid port number: {}", line_num, parts[0])
        })?;
        let protocol = parts.get(1).unwrap_or(&"tcp").to_string();

        Ok(BuildInstruction::Expose { port, protocol })
    }

    fn parse_volume(args: &str) -> Result<BuildInstruction, String> {
        let paths = if args.starts_with('[') {
            serde_json::from_str(args).unwrap_or_default()
        } else {
            args.split_whitespace()
                .map(|s| s.to_string())
                .collect()
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
        let shell: Vec<String> = serde_json::from_str(args).map_err(|_| {
            format!("Line {}: SHELL requires JSON array format", line_num)
        })?;

        Ok(BuildInstruction::Shell { shell })
    }

    /// Build implementation
    fn build_impl(&mut self, config: BuildConfig) -> String {
        let errors = Vec::new();
        let mut warnings = Vec::new();
        let mut layers = Vec::new();

        // Find build file
        let build_file = config.build_file.clone().unwrap_or_else(|| {
            let runefile = format!("{}/Runefile", config.context_dir);
            if self.fs.exists_impl(&runefile) {
                runefile
            } else {
                format!("{}/Dockerfile", config.context_dir)
            }
        });

        // Read and parse build file
        let content = match self.fs.read_file_impl(&build_file) {
            Some(bytes) => match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(_) => {
                    return serde_json::to_string(&BuildResult {
                        success: false,
                        image_id: None,
                        layers: Vec::new(),
                        config: None,
                        errors: vec!["Invalid UTF-8 in build file".to_string()],
                        warnings: Vec::new(),
                    })
                    .unwrap_or_default();
                }
            },
            None => {
                return serde_json::to_string(&BuildResult {
                    success: false,
                    image_id: None,
                    layers: Vec::new(),
                    config: None,
                    errors: vec![format!("Build file not found: {}", build_file)],
                    warnings: Vec::new(),
                })
                .unwrap_or_default();
            }
        };

        let parsed = match Self::parse_content(&content) {
            Ok(p) => p,
            Err(e) => {
                return serde_json::to_string(&BuildResult {
                    success: false,
                    image_id: None,
                    layers: Vec::new(),
                    config: None,
                    errors: vec![e],
                    warnings: Vec::new(),
                })
                .unwrap_or_default();
            }
        };

        // Process stages
        let target_stage = config.target.as_ref();
        let mut container_config = ContainerConfig::default();
        let mut diff_ids = Vec::new();
        let mut history = Vec::new();

        for (stage_idx, stage) in parsed.stages.iter().enumerate() {
            // Check if this is the target stage
            if let Some(target) = target_stage {
                if stage.name.as_ref() != Some(target) && stage_idx < parsed.stages.len() - 1 {
                    continue;
                }
            }

            self.emit_event(BuildEvent::StageStart {
                stage: stage_idx,
                name: stage.name.clone(),
                base: format!(
                    "{}:{}",
                    stage.base_image,
                    stage.base_tag.as_deref().unwrap_or("latest")
                ),
            });

            // Process instructions
            for (step_idx, instruction) in stage.instructions.iter().enumerate() {
                let instruction_str = format!("{:?}", instruction);
                self.emit_event(BuildEvent::StepStart {
                    step: step_idx,
                    instruction: instruction_str.clone(),
                });

                let (layer_id, empty_layer) = match instruction {
                    BuildInstruction::Run { command, .. } => {
                        // Simulate running a command - create a layer
                        let layer_digest = Self::calculate_digest(command.as_bytes());
                        let layer_id = layer_digest[7..19].to_string();
                        
                        layers.push(ImageLayer {
                            id: layer_id.clone(),
                            digest: layer_digest.clone(),
                            size: command.len() as u64,
                            created_by: format!("RUN {}", command),
                            empty_layer: false,
                        });
                        
                        diff_ids.push(layer_digest);
                        (Some(layer_id), false)
                    }
                    BuildInstruction::Copy { src, dest, .. } => {
                        // Copy files from context
                        let mut layer_content = Vec::new();
                        
                        for src_path in src {
                            let full_path = if src_path.starts_with('/') {
                                src_path.clone()
                            } else {
                                format!("{}/{}", config.context_dir, src_path)
                            };
                            
                            if let Some(content) = self.fs.read_file_impl(&full_path) {
                                layer_content.extend_from_slice(&content);
                            } else {
                                warnings.push(format!("Source file not found: {}", full_path));
                            }
                        }

                        if !layer_content.is_empty() {
                            let layer_digest = Self::calculate_digest(&layer_content);
                            let layer_id = layer_digest[7..19].to_string();
                            
                            layers.push(ImageLayer {
                                id: layer_id.clone(),
                                digest: layer_digest.clone(),
                                size: layer_content.len() as u64,
                                created_by: format!("COPY {} {}", src.join(" "), dest),
                                empty_layer: false,
                            });
                            
                            diff_ids.push(layer_digest);
                            (Some(layer_id), false)
                        } else {
                            (None, true)
                        }
                    }
                    BuildInstruction::Add { src, dest, .. } => {
                        // Similar to COPY but could handle URLs/archives
                        let mut layer_content = Vec::new();
                        
                        for src_path in src {
                            let full_path = if src_path.starts_with('/') {
                                src_path.clone()
                            } else {
                                format!("{}/{}", config.context_dir, src_path)
                            };
                            
                            if let Some(content) = self.fs.read_file_impl(&full_path) {
                                layer_content.extend_from_slice(&content);
                            }
                        }

                        if !layer_content.is_empty() {
                            let layer_digest = Self::calculate_digest(&layer_content);
                            let layer_id = layer_digest[7..19].to_string();
                            
                            layers.push(ImageLayer {
                                id: layer_id.clone(),
                                digest: layer_digest.clone(),
                                size: layer_content.len() as u64,
                                created_by: format!("ADD {} {}", src.join(" "), dest),
                                empty_layer: false,
                            });
                            
                            diff_ids.push(layer_digest);
                            (Some(layer_id), false)
                        } else {
                            (None, true)
                        }
                    }
                    BuildInstruction::Env { key, value } => {
                        container_config.env.push(format!("{}={}", key, value));
                        (None, true)
                    }
                    BuildInstruction::Cmd { command, .. } => {
                        container_config.cmd = command.clone();
                        (None, true)
                    }
                    BuildInstruction::Entrypoint { command, .. } => {
                        container_config.entrypoint = command.clone();
                        (None, true)
                    }
                    BuildInstruction::Workdir { path } => {
                        container_config.working_dir = path.clone();
                        (None, true)
                    }
                    BuildInstruction::User { user, .. } => {
                        container_config.user = user.clone();
                        (None, true)
                    }
                    BuildInstruction::Expose { port, protocol } => {
                        container_config.exposed_ports.insert(
                            format!("{}/{}", port, protocol),
                            serde_json::json!({}),
                        );
                        (None, true)
                    }
                    BuildInstruction::Volume { paths } => {
                        for path in paths {
                            container_config.volumes.insert(path.clone(), serde_json::json!({}));
                        }
                        (None, true)
                    }
                    BuildInstruction::Label { labels } => {
                        container_config.labels.extend(labels.clone());
                        (None, true)
                    }
                    BuildInstruction::Stopsignal { signal } => {
                        container_config.stop_signal = signal.clone();
                        (None, true)
                    }
                    _ => (None, true),
                };

                // Add history entry
                history.push(HistoryEntry {
                    created: chrono_lite_now(),
                    created_by: instruction_str,
                    empty_layer,
                    comment: None,
                });

                self.emit_event(BuildEvent::StepComplete {
                    step: step_idx,
                    layer_id,
                });
            }

            self.emit_event(BuildEvent::StageComplete { stage: stage_idx });
        }

        // Add build labels
        for (key, value) in &config.labels {
            container_config.labels.insert(key.clone(), value.clone());
        }

        // Generate image ID
        let config_json = serde_json::to_string(&container_config).unwrap_or_default();
        let image_id = Self::calculate_digest(config_json.as_bytes())[7..19].to_string();

        // Create image config
        let image_config = ImageConfig {
            architecture: "amd64".to_string(),
            os: "linux".to_string(),
            config: container_config,
            rootfs: RootFs {
                fs_type: "layers".to_string(),
                diff_ids,
            },
            history,
        };

        self.emit_event(BuildEvent::BuildComplete {
            image_id: image_id.clone(),
        });

        serde_json::to_string(&BuildResult {
            success: errors.is_empty(),
            image_id: Some(image_id),
            layers,
            config: Some(image_config),
            errors,
            warnings,
        })
        .unwrap_or_default()
    }

    /// Emit a build event to the progress callback
    fn emit_event(&self, event: BuildEvent) {
        if let Some(ref callback) = self.progress_callback {
            let event_json = serde_json::to_string(&event).unwrap_or_default();
            let this = JsValue::null();
            let arg = JsValue::from_str(&event_json);
            let _ = callback.call1(&this, &arg);
        }
    }
}

/// Simple timestamp function (no chrono dependency)
fn chrono_lite_now() -> String {
    // In WASM, we could use js_sys::Date, but for simplicity return a placeholder
    // that can be overridden by the host
    "2024-01-01T00:00:00Z".to_string()
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

        let parsed = WasmBuilder::parse_content(content).unwrap();
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

        let parsed = WasmBuilder::parse_content(content).unwrap();
        assert_eq!(parsed.stages.len(), 2);
        assert_eq!(parsed.stages[0].name, Some("builder".to_string()));
        assert_eq!(parsed.stages[1].base_image, "debian");
    }

    #[test]
    fn test_calculate_digest() {
        let digest = WasmBuilder::calculate_digest(b"hello world");
        assert!(digest.starts_with("sha256:"));
        assert_eq!(digest.len(), 71); // sha256: + 64 hex chars
    }

    #[test]
    fn test_default_build_file() {
        assert_eq!(WasmBuilder::get_default_build_file(), "Runefile");
    }
}
