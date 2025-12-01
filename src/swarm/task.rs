//! Swarm task management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Task state
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    /// Task is new
    #[default]
    New,
    /// Task is pending
    Pending,
    /// Task is assigned
    Assigned,
    /// Task is accepted
    Accepted,
    /// Task is preparing
    Preparing,
    /// Task is ready
    Ready,
    /// Task is starting
    Starting,
    /// Task is running
    Running,
    /// Task completed
    Complete,
    /// Task shutdown
    Shutdown,
    /// Task failed
    Failed,
    /// Task rejected
    Rejected,
    /// Task removed
    Remove,
    /// Task is orphaned
    Orphaned,
}

/// Swarm task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Task ID
    pub id: String,
    /// Service ID
    pub service_id: String,
    /// Slot (for replicated services)
    pub slot: Option<u64>,
    /// Node ID
    pub node_id: Option<String>,
    /// Task spec
    pub spec: TaskSpecRef,
    /// Task status
    pub status: TaskStatus,
    /// Desired state
    pub desired_state: TaskState,
    /// Network attachments
    pub network_attachments: Vec<NetworkAttachment>,
    /// Generic resources
    pub assigned_generic_resources: Vec<GenericResource>,
    /// Task version
    pub version: TaskVersion,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl Task {
    /// Create a new task
    pub fn new(service_id: &str, slot: Option<u64>) -> Self {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        Self {
            id,
            service_id: service_id.to_string(),
            slot,
            node_id: None,
            spec: TaskSpecRef::default(),
            status: TaskStatus::default(),
            desired_state: TaskState::Running,
            network_attachments: Vec::new(),
            assigned_generic_resources: Vec::new(),
            version: TaskVersion { index: 1 },
            created_at: now,
            updated_at: now,
        }
    }

    /// Assign task to a node
    pub fn assign(&mut self, node_id: &str) {
        self.node_id = Some(node_id.to_string());
        self.status.state = TaskState::Assigned;
        self.updated_at = Utc::now();
    }

    /// Start the task
    pub fn start(&mut self) {
        self.status.state = TaskState::Starting;
        self.updated_at = Utc::now();
    }

    /// Set task as running
    pub fn set_running(&mut self, container_id: &str) {
        self.status.state = TaskState::Running;
        self.status.container_status = Some(ContainerStatus {
            container_id: container_id.to_string(),
            pid: None,
            exit_code: None,
        });
        self.updated_at = Utc::now();
    }

    /// Complete the task
    pub fn complete(&mut self, exit_code: i64) {
        self.status.state = TaskState::Complete;
        if let Some(ref mut cs) = self.status.container_status {
            cs.exit_code = Some(exit_code);
        }
        self.updated_at = Utc::now();
    }

    /// Fail the task
    pub fn fail(&mut self, error: &str) {
        self.status.state = TaskState::Failed;
        self.status.err = Some(error.to_string());
        self.updated_at = Utc::now();
    }

    /// Shutdown the task
    pub fn shutdown(&mut self) {
        self.desired_state = TaskState::Shutdown;
        self.updated_at = Utc::now();
    }

    /// Check if task is terminal
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status.state,
            TaskState::Complete
                | TaskState::Failed
                | TaskState::Rejected
                | TaskState::Remove
                | TaskState::Orphaned
        )
    }

    /// Check if task is running
    pub fn is_running(&self) -> bool {
        self.status.state == TaskState::Running
    }
}

/// Task spec reference
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskSpecRef {
    /// Plugin spec
    pub plugin_spec: Option<PluginSpecRef>,
    /// Container spec
    pub container_spec: Option<ContainerSpecRef>,
    /// Network attachment spec
    pub network_attachment_spec: Option<NetworkAttachmentSpecRef>,
    /// Resources
    pub resources: Option<ResourceRequirements>,
    /// Restart policy
    pub restart_policy: Option<RestartPolicy>,
    /// Placement
    pub placement: Option<Placement>,
    /// Force update
    pub force_update: Option<u64>,
    /// Runtime
    pub runtime: Option<String>,
    /// Networks
    pub networks: Vec<NetworkAttachmentConfig>,
    /// Log driver
    pub log_driver: Option<LogDriver>,
}

/// Plugin spec reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSpecRef {
    pub name: String,
    pub remote: String,
    pub disabled: bool,
}

/// Container spec reference
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContainerSpecRef {
    pub image: String,
    pub labels: HashMap<String, String>,
    pub command: Vec<String>,
    pub args: Vec<String>,
    pub hostname: Option<String>,
    pub env: Vec<String>,
    pub dir: Option<String>,
    pub user: Option<String>,
}

/// Network attachment spec reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAttachmentSpecRef {
    pub container_id: String,
}

/// Resource requirements
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub limits: Option<ResourceSpec>,
    pub reservations: Option<ResourceSpec>,
}

/// Resource specification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceSpec {
    pub nano_cpus: Option<i64>,
    pub memory_bytes: Option<i64>,
    pub pids: Option<i64>,
    pub generic_resources: Vec<GenericResource>,
}

/// Generic resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericResource {
    pub named_resource_spec: Option<NamedResourceSpec>,
    pub discrete_resource_spec: Option<DiscreteResourceSpec>,
}

/// Named resource spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedResourceSpec {
    pub kind: String,
    pub value: String,
}

/// Discrete resource spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscreteResourceSpec {
    pub kind: String,
    pub value: i64,
}

/// Restart policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartPolicy {
    pub condition: Option<String>,
    pub delay: Option<i64>,
    pub max_attempts: Option<u64>,
    pub window: Option<i64>,
}

/// Placement
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Placement {
    pub constraints: Vec<String>,
    pub preferences: Vec<PlacementPreference>,
    pub max_replicas: Option<u64>,
    pub platforms: Vec<Platform>,
}

/// Placement preference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementPreference {
    pub spread: Option<SpreadOver>,
}

/// Spread over
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadOver {
    pub spread_descriptor: String,
}

/// Platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Platform {
    pub architecture: Option<String>,
    pub os: Option<String>,
}

/// Network attachment config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAttachmentConfig {
    pub target: String,
    pub aliases: Vec<String>,
    pub driver_opts: HashMap<String, String>,
}

/// Log driver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogDriver {
    pub name: String,
    pub options: HashMap<String, String>,
}

/// Task status
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskStatus {
    /// Timestamp
    pub timestamp: Option<DateTime<Utc>>,
    /// State
    pub state: TaskState,
    /// Message
    pub message: String,
    /// Error
    pub err: Option<String>,
    /// Container status
    pub container_status: Option<ContainerStatus>,
    /// Port status
    pub port_status: Option<PortStatus>,
}

/// Container status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStatus {
    /// Container ID
    pub container_id: String,
    /// PID
    pub pid: Option<i64>,
    /// Exit code
    pub exit_code: Option<i64>,
}

/// Port status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortStatus {
    /// Ports
    pub ports: Vec<PortConfig>,
}

/// Port config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortConfig {
    pub name: Option<String>,
    pub protocol: Option<String>,
    pub target_port: u16,
    pub published_port: Option<u16>,
    pub publish_mode: Option<String>,
}

/// Network attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAttachment {
    /// Network
    pub network: NetworkRef,
    /// Addresses
    pub addresses: Vec<String>,
}

/// Network reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRef {
    /// ID
    pub id: String,
    /// Name
    pub name: String,
}

/// Task version
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskVersion {
    pub index: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_task() {
        let task = Task::new("service-123", Some(1));
        assert_eq!(task.service_id, "service-123");
        assert_eq!(task.slot, Some(1));
        assert_eq!(task.status.state, TaskState::New);
    }

    #[test]
    fn test_task_lifecycle() {
        let mut task = Task::new("service-123", Some(1));

        task.assign("node-456");
        assert_eq!(task.status.state, TaskState::Assigned);
        assert_eq!(task.node_id, Some("node-456".to_string()));

        task.start();
        assert_eq!(task.status.state, TaskState::Starting);

        task.set_running("container-789");
        assert!(task.is_running());

        task.complete(0);
        assert!(task.is_terminal());
    }

    #[test]
    fn test_task_failure() {
        let mut task = Task::new("service-123", None);

        task.assign("node-456");
        task.start();
        task.fail("Container crashed");

        assert_eq!(task.status.state, TaskState::Failed);
        assert_eq!(task.status.err, Some("Container crashed".to_string()));
        assert!(task.is_terminal());
    }
}
