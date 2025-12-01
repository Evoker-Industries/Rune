//! WASM Image Builder

use crate::filesystem::BuilderFilesystem;
use crate::parser::RunefileParser;
use crate::types::*;
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;

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
        match RunefileParser::parse_content(content) {
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
        let parser = RunefileParser::new();
        parser.validate(content)
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
    /// Build implementation
    fn build_impl(&mut self, config: BuildConfig) -> String {
        let errors: Vec<String> = Vec::new();
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

        let parsed = match RunefileParser::parse_content(&content) {
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
    "2024-01-01T00:00:00Z".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_digest() {
        let digest = WasmBuilder::calculate_digest(b"hello world");
        assert!(digest.starts_with("sha256:"));
        assert_eq!(digest.len(), 71);
    }

    #[test]
    fn test_default_build_file() {
        assert_eq!(WasmBuilder::get_default_build_file(), "Runefile");
    }
}
