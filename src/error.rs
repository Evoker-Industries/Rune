//! Error types for Rune

use thiserror::Error;

/// Result type for Rune operations
pub type Result<T> = std::result::Result<T, RuneError>;

/// Rune error types
#[derive(Error, Debug)]
pub enum RuneError {
    #[error("Container error: {0}")]
    Container(String),

    #[error("Container not found: {0}")]
    ContainerNotFound(String),

    #[error("Container already exists: {0}")]
    ContainerExists(String),

    #[error("Container already running: {0}")]
    ContainerAlreadyRunning(String),

    #[error("Container not running: {0}")]
    ContainerNotRunning(String),

    #[error("Image error: {0}")]
    Image(String),

    #[error("Image not found: {0}")]
    ImageNotFound(String),

    #[error("Image already exists: {0}")]
    ImageExists(String),

    #[error("Build error: {0}")]
    Build(String),

    #[error("Dockerfile parse error at line {line}: {message}")]
    DockerfileParse { line: usize, message: String },

    #[error("Network error: {0}")]
    Network(String),

    #[error("Network not found: {0}")]
    NetworkNotFound(String),

    #[error("Volume error: {0}")]
    Volume(String),

    #[error("Volume not found: {0}")]
    VolumeNotFound(String),

    #[error("Compose error: {0}")]
    Compose(String),

    #[error("Compose file parse error: {0}")]
    ComposeParse(String),

    #[error("Swarm error: {0}")]
    Swarm(String),

    #[error("Service error: {0}")]
    Service(String),

    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    #[error("Node error: {0}")]
    Node(String),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Lock error: {0}")]
    Lock(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
