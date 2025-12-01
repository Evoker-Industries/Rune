//! Swarm node management

use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Node role in the swarm
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeRole {
    /// Worker node
    #[default]
    Worker,
    /// Manager node
    Manager,
}

/// Node state
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeState {
    /// Node is unknown
    #[default]
    Unknown,
    /// Node is down
    Down,
    /// Node is ready
    Ready,
    /// Node is disconnected
    Disconnected,
}

/// Node availability
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeAvailability {
    /// Node is active
    #[default]
    Active,
    /// Node is paused
    Pause,
    /// Node is draining
    Drain,
}

/// Swarm node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Node ID
    pub id: String,
    /// Node hostname
    pub hostname: String,
    /// Node role
    pub role: NodeRole,
    /// Node state
    pub state: NodeState,
    /// Node availability
    pub availability: String,
    /// Node address
    pub addr: String,
    /// Node labels
    pub labels: HashMap<String, String>,
    /// Node description
    pub description: NodeDescription,
    /// Manager status (if manager)
    pub manager_status: Option<ManagerStatus>,
    /// Node status
    pub status: NodeStatus,
    /// Node version
    pub version: NodeVersion,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl Node {
    /// Create a new local node
    pub fn new_local(role: NodeRole) -> Self {
        let id = Uuid::new_v4().to_string();
        let hostname = gethostname::gethostname().to_string_lossy().to_string();

        let now = Utc::now();

        let manager_status = if role == NodeRole::Manager {
            Some(ManagerStatus {
                leader: true,
                reachability: "reachable".to_string(),
                addr: "127.0.0.1:2377".to_string(),
            })
        } else {
            None
        };

        Self {
            id,
            hostname: hostname.clone(),
            role,
            state: NodeState::Ready,
            availability: "active".to_string(),
            addr: "127.0.0.1".to_string(),
            labels: HashMap::new(),
            description: NodeDescription {
                hostname,
                platform: Platform {
                    architecture: std::env::consts::ARCH.to_string(),
                    os: std::env::consts::OS.to_string(),
                },
                resources: Resources {
                    nano_cpus: num_cpus::get() as i64 * 1_000_000_000,
                    memory_bytes: get_total_memory(),
                    generic_resources: Vec::new(),
                },
                engine: EngineDescription {
                    engine_version: env!("CARGO_PKG_VERSION").to_string(),
                    labels: HashMap::new(),
                    plugins: Vec::new(),
                },
                tls_info: None,
            },
            manager_status,
            status: NodeStatus {
                state: NodeState::Ready,
                message: String::new(),
                addr: "127.0.0.1".to_string(),
            },
            version: NodeVersion { index: 1 },
            created_at: now,
            updated_at: now,
        }
    }

    /// Promote node to manager
    pub fn promote(&mut self) -> Result<()> {
        self.role = NodeRole::Manager;
        self.manager_status = Some(ManagerStatus {
            leader: false,
            reachability: "reachable".to_string(),
            addr: format!("{}:2377", self.addr),
        });
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Demote node to worker
    pub fn demote(&mut self) -> Result<()> {
        self.role = NodeRole::Worker;
        self.manager_status = None;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Set node availability
    pub fn set_availability(&mut self, availability: &str) {
        self.availability = availability.to_string();
        self.updated_at = Utc::now();
    }

    /// Add label
    pub fn add_label(&mut self, key: &str, value: &str) {
        self.labels.insert(key.to_string(), value.to_string());
        self.updated_at = Utc::now();
    }

    /// Remove label
    pub fn remove_label(&mut self, key: &str) {
        self.labels.remove(key);
        self.updated_at = Utc::now();
    }

    /// Check if node is a manager
    pub fn is_manager(&self) -> bool {
        self.role == NodeRole::Manager
    }

    /// Check if node is leader
    pub fn is_leader(&self) -> bool {
        self.manager_status
            .as_ref()
            .map(|s| s.leader)
            .unwrap_or(false)
    }

    /// Check if node is ready
    pub fn is_ready(&self) -> bool {
        self.state == NodeState::Ready
    }

    /// Check if node is available for scheduling
    pub fn is_available(&self) -> bool {
        self.is_ready() && self.availability == "active"
    }
}

/// Node description
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeDescription {
    /// Hostname
    pub hostname: String,
    /// Platform info
    pub platform: Platform,
    /// Resources
    pub resources: Resources,
    /// Engine description
    pub engine: EngineDescription,
    /// TLS info
    pub tls_info: Option<TlsInfo>,
}

/// Platform information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Platform {
    /// CPU architecture
    pub architecture: String,
    /// Operating system
    pub os: String,
}

/// Node resources
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Resources {
    /// CPU in nanoseconds
    pub nano_cpus: i64,
    /// Memory in bytes
    pub memory_bytes: i64,
    /// Generic resources
    pub generic_resources: Vec<GenericResource>,
}

/// Generic resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericResource {
    /// Named resource count
    pub named_resource_spec: Option<NamedResourceSpec>,
    /// Discrete resource count
    pub discrete_resource_spec: Option<DiscreteResourceSpec>,
}

/// Named resource specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedResourceSpec {
    /// Kind
    pub kind: String,
    /// Value
    pub value: String,
}

/// Discrete resource specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscreteResourceSpec {
    /// Kind
    pub kind: String,
    /// Value
    pub value: i64,
}

/// Engine description
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EngineDescription {
    /// Engine version
    pub engine_version: String,
    /// Labels
    pub labels: HashMap<String, String>,
    /// Plugins
    pub plugins: Vec<PluginDescription>,
}

/// Plugin description
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDescription {
    /// Plugin type
    #[serde(rename = "Type")]
    pub plugin_type: String,
    /// Plugin name
    #[serde(rename = "Name")]
    pub name: String,
}

/// TLS information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsInfo {
    /// Trust root
    pub trust_root: String,
    /// Certificate issuer subject
    pub cert_issuer_subject: String,
    /// Certificate issuer public key
    pub cert_issuer_public_key: String,
}

/// Manager status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerStatus {
    /// Is leader
    pub leader: bool,
    /// Reachability
    pub reachability: String,
    /// Address
    pub addr: String,
}

/// Node status
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeStatus {
    /// State
    pub state: NodeState,
    /// Message
    pub message: String,
    /// Address
    pub addr: String,
}

/// Node version
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeVersion {
    /// Index
    pub index: u64,
}

/// Get total system memory (placeholder)
fn get_total_memory() -> i64 {
    // In a real implementation, we would use sysinfo or similar
    8 * 1024 * 1024 * 1024 // 8GB default
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_local_node() {
        let node = Node::new_local(NodeRole::Manager);
        assert!(node.is_manager());
        assert!(node.is_ready());
        assert!(node.is_available());
    }

    #[test]
    fn test_promote_demote() {
        let mut node = Node::new_local(NodeRole::Worker);
        assert!(!node.is_manager());

        node.promote().unwrap();
        assert!(node.is_manager());

        node.demote().unwrap();
        assert!(!node.is_manager());
    }

    #[test]
    fn test_labels() {
        let mut node = Node::new_local(NodeRole::Worker);

        node.add_label("env", "production");
        assert_eq!(node.labels.get("env"), Some(&"production".to_string()));

        node.remove_label("env");
        assert!(!node.labels.contains_key("env"));
    }
}
