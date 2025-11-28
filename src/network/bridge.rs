//! Bridge network implementation

use super::config::{IpAllocator, NetworkConfig, NetworkContainer, NetworkDriver};
use crate::error::{Result, RuneError};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Bridge network manager
pub struct BridgeNetwork {
    /// Network configuration
    pub config: NetworkConfig,
    /// IP allocator
    allocator: IpAllocator,
}

impl BridgeNetwork {
    /// Create a new bridge network
    pub fn new(config: NetworkConfig) -> Result<Self> {
        let subnet = config.ipam.config.first()
            .map(|c| c.subnet.as_str())
            .unwrap_or("172.17.0.0/16");

        let allocator = IpAllocator::new(subnet)?;

        Ok(Self {
            config,
            allocator,
        })
    }

    /// Connect a container to this network
    pub fn connect(&mut self, container_id: &str, container_name: &str) -> Result<NetworkContainer> {
        let ip = self.allocator.allocate()?;
        let endpoint_id = Uuid::new_v4().to_string().replace("-", "")[..12].to_string();

        let container = NetworkContainer {
            container_id: container_id.to_string(),
            name: container_name.to_string(),
            endpoint_id,
            mac_address: generate_mac_address(),
            ipv4_address: Some(format!("{}/16", ip)),
            ipv6_address: None,
        };

        self.config.containers.insert(container_id.to_string(), container.clone());

        Ok(container)
    }

    /// Disconnect a container from this network
    pub fn disconnect(&mut self, container_id: &str) -> Result<()> {
        let container = self.config.containers.remove(container_id)
            .ok_or_else(|| RuneError::Container(format!(
                "Container {} not connected to network {}",
                container_id,
                self.config.name
            )))?;

        if let Some(ip_str) = container.ipv4_address {
            if let Some(ip) = ip_str.split('/').next() {
                if let Ok(ip) = ip.parse() {
                    self.allocator.release(ip);
                }
            }
        }

        Ok(())
    }

    /// Get connected containers
    pub fn containers(&self) -> &HashMap<String, NetworkContainer> {
        &self.config.containers
    }
}

/// Network manager for handling all networks
pub struct NetworkManager {
    /// Networks indexed by ID
    networks: Arc<RwLock<HashMap<String, BridgeNetwork>>>,
    /// Name to ID mapping
    names: Arc<RwLock<HashMap<String, String>>>,
}

impl NetworkManager {
    /// Create a new network manager
    pub fn new() -> Result<Self> {
        let manager = Self {
            networks: Arc::new(RwLock::new(HashMap::new())),
            names: Arc::new(RwLock::new(HashMap::new())),
        };

        // Create default networks
        manager.create_default_networks()?;

        Ok(manager)
    }

    /// Create default networks (bridge, host, none)
    fn create_default_networks(&self) -> Result<()> {
        // Default bridge network
        let bridge = NetworkConfig::new("bridge")
            .driver(NetworkDriver::Bridge)
            .subnet("172.17.0.0/16")
            .gateway("172.17.0.1");
        self.create(bridge)?;

        // Host network
        let mut host = NetworkConfig::new("host");
        host.driver = NetworkDriver::Host;
        self.create(host)?;

        // None network
        let mut none = NetworkConfig::new("none");
        none.driver = NetworkDriver::None;
        self.create(none)?;

        Ok(())
    }

    /// Create a new network
    pub fn create(&self, config: NetworkConfig) -> Result<String> {
        let id = config.id.clone();
        let name = config.name.clone();

        let network = BridgeNetwork::new(config)?;

        let mut networks = self.networks.write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;
        let mut names = self.names.write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        if names.contains_key(&name) {
            return Err(RuneError::Network(format!("Network {} already exists", name)));
        }

        networks.insert(id.clone(), network);
        names.insert(name, id.clone());

        Ok(id)
    }

    /// Remove a network
    pub fn remove(&self, id_or_name: &str) -> Result<()> {
        let mut networks = self.networks.write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;
        let mut names = self.names.write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        // Find network
        let id = if networks.contains_key(id_or_name) {
            id_or_name.to_string()
        } else if let Some(id) = names.get(id_or_name) {
            id.clone()
        } else {
            return Err(RuneError::NetworkNotFound(id_or_name.to_string()));
        };

        // Check if network has connected containers
        if let Some(network) = networks.get(&id) {
            if !network.config.containers.is_empty() {
                return Err(RuneError::Network(format!(
                    "Network {} has active endpoints",
                    id_or_name
                )));
            }

            // Remove name mapping
            names.remove(&network.config.name);
        }

        networks.remove(&id);

        Ok(())
    }

    /// Get a network by ID or name
    pub fn get(&self, id_or_name: &str) -> Result<NetworkConfig> {
        let networks = self.networks.read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;
        let names = self.names.read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        let id = if networks.contains_key(id_or_name) {
            id_or_name.to_string()
        } else if let Some(id) = names.get(id_or_name) {
            id.clone()
        } else {
            return Err(RuneError::NetworkNotFound(id_or_name.to_string()));
        };

        networks.get(&id)
            .map(|n| n.config.clone())
            .ok_or_else(|| RuneError::NetworkNotFound(id_or_name.to_string()))
    }

    /// List all networks
    pub fn list(&self) -> Result<Vec<NetworkConfig>> {
        let networks = self.networks.read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        Ok(networks.values().map(|n| n.config.clone()).collect())
    }

    /// Connect a container to a network
    pub fn connect(
        &self,
        network_id_or_name: &str,
        container_id: &str,
        container_name: &str,
    ) -> Result<NetworkContainer> {
        let mut networks = self.networks.write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;
        let names = self.names.read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        let id = if networks.contains_key(network_id_or_name) {
            network_id_or_name.to_string()
        } else if let Some(id) = names.get(network_id_or_name) {
            id.clone()
        } else {
            return Err(RuneError::NetworkNotFound(network_id_or_name.to_string()));
        };

        let network = networks.get_mut(&id)
            .ok_or_else(|| RuneError::NetworkNotFound(network_id_or_name.to_string()))?;

        network.connect(container_id, container_name)
    }

    /// Disconnect a container from a network
    pub fn disconnect(&self, network_id_or_name: &str, container_id: &str) -> Result<()> {
        let mut networks = self.networks.write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;
        let names = self.names.read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        let id = if networks.contains_key(network_id_or_name) {
            network_id_or_name.to_string()
        } else if let Some(id) = names.get(network_id_or_name) {
            id.clone()
        } else {
            return Err(RuneError::NetworkNotFound(network_id_or_name.to_string()));
        };

        let network = networks.get_mut(&id)
            .ok_or_else(|| RuneError::NetworkNotFound(network_id_or_name.to_string()))?;

        network.disconnect(container_id)
    }

    /// Prune unused networks
    pub fn prune(&self) -> Result<Vec<String>> {
        let networks = self.networks.read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        // Find networks with no containers (excluding default networks)
        let to_remove: Vec<String> = networks.iter()
            .filter(|(_, n)| {
                n.config.containers.is_empty()
                    && n.config.name != "bridge"
                    && n.config.name != "host"
                    && n.config.name != "none"
            })
            .map(|(id, _)| id.clone())
            .collect();

        drop(networks);

        // Remove networks
        for id in &to_remove {
            self.remove(id)?;
        }

        Ok(to_remove)
    }
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new().expect("Failed to create network manager")
    }
}

/// Generate a random MAC address
fn generate_mac_address() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Use locally administered, unicast MAC
    let bytes: [u8; 6] = [
        0x02, // Locally administered
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
    ];

    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_manager_default_networks() {
        let manager = NetworkManager::new().unwrap();
        
        // Should have default networks
        let bridge = manager.get("bridge").unwrap();
        assert_eq!(bridge.driver, NetworkDriver::Bridge);

        let host = manager.get("host").unwrap();
        assert_eq!(host.driver, NetworkDriver::Host);

        let none = manager.get("none").unwrap();
        assert_eq!(none.driver, NetworkDriver::None);
    }

    #[test]
    fn test_create_network() {
        let manager = NetworkManager::new().unwrap();

        let config = NetworkConfig::new("my-network")
            .subnet("10.0.0.0/24");

        let id = manager.create(config).unwrap();
        assert!(!id.is_empty());

        let network = manager.get("my-network").unwrap();
        assert_eq!(network.name, "my-network");
    }

    #[test]
    fn test_connect_container() {
        let manager = NetworkManager::new().unwrap();

        let config = NetworkConfig::new("test-network")
            .subnet("192.168.0.0/24");
        manager.create(config).unwrap();

        let container = manager.connect("test-network", "container1", "test-container").unwrap();
        assert!(container.ipv4_address.is_some());
    }
}
