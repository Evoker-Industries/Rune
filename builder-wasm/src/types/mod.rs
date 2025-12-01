//! Build types for WASM builder

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    StageStart {
        stage: usize,
        name: Option<String>,
        base: String,
    },
    StepStart {
        step: usize,
        instruction: String,
    },
    StepComplete {
        step: usize,
        layer_id: Option<String>,
    },
    StageComplete {
        stage: usize,
    },
    BuildComplete {
        image_id: String,
    },
    Error {
        message: String,
    },
    Warning {
        message: String,
    },
    Progress {
        message: String,
        percent: Option<u8>,
    },
}
