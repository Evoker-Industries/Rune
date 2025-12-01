//! Swarm cluster management

use super::node::{Node, NodeRole, NodeState};
use super::service::Service;
use crate::error::{Result, RuneError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Swarm cluster configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    /// Cluster name
    pub name: String,
    /// Listen address for swarm communication
    pub listen_addr: String,
    /// Advertise address
    pub advertise_addr: String,
    /// Data path address for data traffic
    pub data_path_addr: Option<String>,
    /// Data path port
    pub data_path_port: Option<u16>,
    /// Default address pool
    pub default_addr_pool: Vec<String>,
    /// Subnet size for networks
    pub subnet_size: u8,
    /// Force new cluster (ignore existing state)
    pub force_new_cluster: bool,
    /// Availability (active, pause, drain)
    pub availability: String,
    /// Raft configuration
    pub raft: RaftConfig,
    /// Dispatcher configuration
    pub dispatcher: DispatcherConfig,
    /// CA configuration
    pub ca_config: CaConfig,
    /// Encryption configuration
    pub encryption_config: EncryptionConfig,
    /// Task history retention limit
    pub task_history_retention_limit: i64,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            listen_addr: "0.0.0.0:2377".to_string(),
            advertise_addr: String::new(),
            data_path_addr: None,
            data_path_port: Some(4789),
            default_addr_pool: vec!["10.0.0.0/8".to_string()],
            subnet_size: 24,
            force_new_cluster: false,
            availability: "active".to_string(),
            raft: RaftConfig::default(),
            dispatcher: DispatcherConfig::default(),
            ca_config: CaConfig::default(),
            encryption_config: EncryptionConfig::default(),
            task_history_retention_limit: 5,
        }
    }
}

/// Raft consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftConfig {
    /// Snapshot interval
    pub snapshot_interval: u64,
    /// Keep old snapshots
    pub keep_old_snapshots: u64,
    /// Log entries for slow followers
    pub log_entries_for_slow_followers: u64,
    /// Election tick
    pub election_tick: u32,
    /// Heartbeat tick
    pub heartbeat_tick: u32,
}

impl Default for RaftConfig {
    fn default() -> Self {
        Self {
            snapshot_interval: 10000,
            keep_old_snapshots: 0,
            log_entries_for_slow_followers: 500,
            election_tick: 10,
            heartbeat_tick: 1,
        }
    }
}

/// Dispatcher configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatcherConfig {
    /// Heartbeat period
    pub heartbeat_period: String,
}

impl Default for DispatcherConfig {
    fn default() -> Self {
        Self {
            heartbeat_period: "5s".to_string(),
        }
    }
}

/// CA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaConfig {
    /// Node certificate expiry
    pub node_cert_expiry: String,
    /// External CAs
    pub external_cas: Vec<ExternalCa>,
    /// Signing CA certificate
    pub signing_ca_cert: Option<String>,
    /// Signing CA key
    pub signing_ca_key: Option<String>,
    /// Force rotate
    pub force_rotate: u64,
}

impl Default for CaConfig {
    fn default() -> Self {
        Self {
            node_cert_expiry: "2160h0m0s".to_string(), // 90 days
            external_cas: Vec::new(),
            signing_ca_cert: None,
            signing_ca_key: None,
            force_rotate: 0,
        }
    }
}

/// External CA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalCa {
    /// Protocol
    pub protocol: String,
    /// URL
    pub url: String,
    /// Options
    pub options: HashMap<String, String>,
    /// CA certificate
    pub ca_cert: String,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Auto-lock managers
    pub auto_lock_managers: bool,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            auto_lock_managers: false,
        }
    }
}

/// Swarm cluster state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SwarmState {
    /// Not part of a swarm
    Inactive,
    /// Pending join
    Pending,
    /// Active in swarm
    Active,
    /// Locked
    Locked,
    /// Error state
    Error,
}

/// Join token type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Worker,
    Manager,
}

/// Swarm cluster
pub struct SwarmCluster {
    /// Cluster ID
    id: String,
    /// Configuration
    config: SwarmConfig,
    /// Cluster state
    state: SwarmState,
    /// Nodes in the cluster
    nodes: Arc<RwLock<HashMap<String, Node>>>,
    /// Services
    services: Arc<RwLock<HashMap<String, Service>>>,
    /// Worker join token
    worker_token: String,
    /// Manager join token
    manager_token: String,
    /// Unlock key
    unlock_key: Option<String>,
    /// Created timestamp
    created_at: DateTime<Utc>,
    /// Updated timestamp
    updated_at: DateTime<Utc>,
    /// Root rotation in progress
    root_rotation_in_progress: bool,
}

impl SwarmCluster {
    /// Initialize a new swarm cluster
    pub fn init(config: SwarmConfig) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        let worker_token = generate_token(TokenType::Worker, &id);
        let manager_token = generate_token(TokenType::Manager, &id);

        let unlock_key = if config.encryption_config.auto_lock_managers {
            Some(generate_unlock_key())
        } else {
            None
        };

        let now = Utc::now();

        let cluster = Self {
            id: id.clone(),
            config,
            state: SwarmState::Active,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            services: Arc::new(RwLock::new(HashMap::new())),
            worker_token,
            manager_token,
            unlock_key,
            created_at: now,
            updated_at: now,
            root_rotation_in_progress: false,
        };

        // Create the local node as first manager
        let local_node = Node::new_local(NodeRole::Manager);
        cluster.add_node(local_node)?;

        Ok(cluster)
    }

    /// Join an existing swarm
    pub fn join(
        join_token: &str,
        remote_addrs: Vec<String>,
        listen_addr: &str,
        advertise_addr: &str,
    ) -> Result<Self> {
        // Parse token to determine role
        let role = if join_token.contains("SWMTKN-1-") {
            if join_token.contains("-manager-") {
                NodeRole::Manager
            } else {
                NodeRole::Worker
            }
        } else {
            return Err(RuneError::Swarm("Invalid join token".to_string()));
        };

        // In a real implementation, we would connect to remote managers
        // and join the cluster via the Raft consensus protocol

        let config = SwarmConfig {
            listen_addr: listen_addr.to_string(),
            advertise_addr: advertise_addr.to_string(),
            ..Default::default()
        };

        let cluster = Self {
            id: extract_cluster_id(join_token)?,
            config,
            state: SwarmState::Active,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            services: Arc::new(RwLock::new(HashMap::new())),
            worker_token: String::new(),
            manager_token: String::new(),
            unlock_key: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            root_rotation_in_progress: false,
        };

        // Create local node
        let local_node = Node::new_local(role);
        cluster.add_node(local_node)?;

        Ok(cluster)
    }

    /// Leave the swarm
    pub fn leave(&mut self, force: bool) -> Result<()> {
        // Check if this is the last manager
        let nodes = self
            .nodes
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        let manager_count = nodes
            .values()
            .filter(|n| n.role == NodeRole::Manager && n.state == NodeState::Ready)
            .count();

        if manager_count <= 1 && !force {
            return Err(RuneError::Swarm(
                "Cannot leave swarm: this is the last manager. Use force to leave anyway."
                    .to_string(),
            ));
        }

        self.state = SwarmState::Inactive;
        Ok(())
    }

    /// Get cluster ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get cluster state
    pub fn state(&self) -> SwarmState {
        self.state
    }

    /// Get join token
    pub fn join_token(&self, token_type: TokenType) -> &str {
        match token_type {
            TokenType::Worker => &self.worker_token,
            TokenType::Manager => &self.manager_token,
        }
    }

    /// Rotate join token
    pub fn rotate_join_token(&mut self, token_type: TokenType) -> Result<String> {
        let new_token = generate_token(token_type, &self.id);

        match token_type {
            TokenType::Worker => self.worker_token = new_token.clone(),
            TokenType::Manager => self.manager_token = new_token.clone(),
        }

        self.updated_at = Utc::now();
        Ok(new_token)
    }

    /// Get unlock key
    pub fn unlock_key(&self) -> Option<&str> {
        self.unlock_key.as_deref()
    }

    /// Rotate unlock key
    pub fn rotate_unlock_key(&mut self) -> Result<String> {
        let new_key = generate_unlock_key();
        self.unlock_key = Some(new_key.clone());
        self.updated_at = Utc::now();
        Ok(new_key)
    }

    /// Lock the cluster
    pub fn lock(&mut self) -> Result<()> {
        if self.unlock_key.is_none() {
            return Err(RuneError::Swarm("Auto-lock is not enabled".to_string()));
        }
        self.state = SwarmState::Locked;
        Ok(())
    }

    /// Unlock the cluster
    pub fn unlock(&mut self, key: &str) -> Result<()> {
        if let Some(ref unlock_key) = self.unlock_key {
            if key == unlock_key {
                self.state = SwarmState::Active;
                Ok(())
            } else {
                Err(RuneError::Swarm("Invalid unlock key".to_string()))
            }
        } else {
            Err(RuneError::Swarm("Cluster is not locked".to_string()))
        }
    }

    /// Add a node to the cluster
    pub fn add_node(&self, node: Node) -> Result<()> {
        let mut nodes = self
            .nodes
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        nodes.insert(node.id.clone(), node);
        Ok(())
    }

    /// Remove a node from the cluster
    pub fn remove_node(&self, node_id: &str, force: bool) -> Result<()> {
        let mut nodes = self
            .nodes
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        let node = nodes
            .get(node_id)
            .ok_or_else(|| RuneError::NodeNotFound(node_id.to_string()))?;

        if node.state == NodeState::Ready && !force {
            return Err(RuneError::Swarm(
                "Cannot remove active node. Drain it first or use force.".to_string(),
            ));
        }

        nodes.remove(node_id);
        Ok(())
    }

    /// List all nodes
    pub fn list_nodes(&self) -> Result<Vec<Node>> {
        let nodes = self
            .nodes
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        Ok(nodes.values().cloned().collect())
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: &str) -> Result<Node> {
        let nodes = self
            .nodes
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        nodes
            .get(node_id)
            .cloned()
            .ok_or_else(|| RuneError::NodeNotFound(node_id.to_string()))
    }

    /// Update node
    pub fn update_node(&self, node_id: &str, updates: NodeUpdate) -> Result<()> {
        let mut nodes = self
            .nodes
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        let node = nodes
            .get_mut(node_id)
            .ok_or_else(|| RuneError::NodeNotFound(node_id.to_string()))?;

        if let Some(role) = updates.role {
            node.role = role;
        }
        if let Some(availability) = updates.availability {
            node.availability = availability;
        }
        if let Some(labels) = updates.labels {
            node.labels = labels;
        }

        Ok(())
    }

    /// Create a service
    pub fn create_service(&self, service: Service) -> Result<String> {
        let mut services = self
            .services
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        let id = service.id.clone();
        services.insert(id.clone(), service);

        Ok(id)
    }

    /// List services
    pub fn list_services(&self) -> Result<Vec<Service>> {
        let services = self
            .services
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        Ok(services.values().cloned().collect())
    }

    /// Get a service by ID or name
    pub fn get_service(&self, id_or_name: &str) -> Result<Service> {
        let services = self
            .services
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        // Try ID first
        if let Some(service) = services.get(id_or_name) {
            return Ok(service.clone());
        }

        // Try name
        for service in services.values() {
            if service.spec.name == id_or_name {
                return Ok(service.clone());
            }
        }

        Err(RuneError::ServiceNotFound(id_or_name.to_string()))
    }

    /// Remove a service
    pub fn remove_service(&self, id_or_name: &str) -> Result<()> {
        let mut services = self
            .services
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        // Try ID first
        if services.remove(id_or_name).is_some() {
            return Ok(());
        }

        // Try name
        let id = services
            .iter()
            .find(|(_, s)| s.spec.name == id_or_name)
            .map(|(id, _)| id.clone());

        if let Some(id) = id {
            services.remove(&id);
            Ok(())
        } else {
            Err(RuneError::ServiceNotFound(id_or_name.to_string()))
        }
    }

    /// Update cluster configuration
    pub fn update(&mut self, config: SwarmConfig) -> Result<()> {
        self.config = config;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Get cluster info
    pub fn info(&self) -> SwarmInfo {
        let nodes = self.nodes.read().unwrap();
        let services = self.services.read().unwrap();

        let manager_count = nodes
            .values()
            .filter(|n| n.role == NodeRole::Manager)
            .count();

        SwarmInfo {
            id: self.id.clone(),
            name: self.config.name.clone(),
            state: self.state,
            node_count: nodes.len(),
            manager_count,
            service_count: services.len(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Node update parameters
pub struct NodeUpdate {
    pub role: Option<NodeRole>,
    pub availability: Option<String>,
    pub labels: Option<HashMap<String, String>>,
}

/// Swarm cluster info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmInfo {
    pub id: String,
    pub name: String,
    pub state: SwarmState,
    pub node_count: usize,
    pub manager_count: usize,
    pub service_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Generate a join token
fn generate_token(token_type: TokenType, cluster_id: &str) -> String {
    let type_str = match token_type {
        TokenType::Worker => "worker",
        TokenType::Manager => "manager",
    };

    let random = Uuid::new_v4().to_string().replace("-", "");
    format!(
        "SWMTKN-1-{}-{}-{}",
        &cluster_id[..8],
        type_str,
        &random[..25]
    )
}

/// Generate an unlock key
fn generate_unlock_key() -> String {
    let random = Uuid::new_v4().to_string().replace("-", "");
    format!("SWMKEY-1-{}", random)
}

/// Extract cluster ID from token
fn extract_cluster_id(token: &str) -> Result<String> {
    if let Some(rest) = token.strip_prefix("SWMTKN-1-") {
        if let Some(idx) = rest.find('-') {
            return Ok(rest[..idx].to_string());
        }
    }
    Err(RuneError::Swarm("Invalid token format".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_cluster() {
        let config = SwarmConfig::default();
        let cluster = SwarmCluster::init(config).unwrap();

        assert_eq!(cluster.state(), SwarmState::Active);
        assert!(!cluster.join_token(TokenType::Worker).is_empty());
        assert!(!cluster.join_token(TokenType::Manager).is_empty());
    }

    #[test]
    fn test_rotate_token() {
        let config = SwarmConfig::default();
        let mut cluster = SwarmCluster::init(config).unwrap();

        let old_token = cluster.join_token(TokenType::Worker).to_string();
        let new_token = cluster.rotate_join_token(TokenType::Worker).unwrap();

        assert_ne!(old_token, new_token);
        assert_eq!(cluster.join_token(TokenType::Worker), new_token);
    }

    #[test]
    fn test_generate_token() {
        let token = generate_token(TokenType::Worker, "abc12345");
        assert!(token.starts_with("SWMTKN-1-"));
        assert!(token.contains("worker"));
    }
}
