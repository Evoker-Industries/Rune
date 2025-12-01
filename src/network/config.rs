//! Network configuration

use crate::error::{Result, RuneError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use uuid::Uuid;

/// Network driver types
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkDriver {
    /// Bridge network (default)
    #[default]
    Bridge,
    /// Host network
    Host,
    /// No networking
    None,
    /// Overlay network (for Swarm)
    Overlay,
    /// Macvlan network
    Macvlan,
    /// IPvlan network
    Ipvlan,
}

impl std::fmt::Display for NetworkDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkDriver::Bridge => write!(f, "bridge"),
            NetworkDriver::Host => write!(f, "host"),
            NetworkDriver::None => write!(f, "none"),
            NetworkDriver::Overlay => write!(f, "overlay"),
            NetworkDriver::Macvlan => write!(f, "macvlan"),
            NetworkDriver::Ipvlan => write!(f, "ipvlan"),
        }
    }
}

/// Network scope
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkScope {
    /// Local to this node
    #[default]
    Local,
    /// Swarm-wide
    Swarm,
    /// Global
    Global,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Network ID
    pub id: String,
    /// Network name
    pub name: String,
    /// Network driver
    pub driver: NetworkDriver,
    /// Network scope
    pub scope: NetworkScope,
    /// Enable IPv6
    pub enable_ipv6: bool,
    /// IPAM configuration
    pub ipam: IpamConfig,
    /// Internal network (no external access)
    pub internal: bool,
    /// Attachable by containers
    pub attachable: bool,
    /// Ingress network
    pub ingress: bool,
    /// Driver options
    pub options: HashMap<String, String>,
    /// Network labels
    pub labels: HashMap<String, String>,
    /// Connected containers
    pub containers: HashMap<String, NetworkContainer>,
    /// Created timestamp
    pub created: DateTime<Utc>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string().replace("-", "")[..12].to_string(),
            name: String::new(),
            driver: NetworkDriver::default(),
            scope: NetworkScope::default(),
            enable_ipv6: false,
            ipam: IpamConfig::default(),
            internal: false,
            attachable: true,
            ingress: false,
            options: HashMap::new(),
            labels: HashMap::new(),
            containers: HashMap::new(),
            created: Utc::now(),
        }
    }
}

impl NetworkConfig {
    /// Create a new network configuration
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Self::default()
        }
    }

    /// Set network driver
    pub fn driver(mut self, driver: NetworkDriver) -> Self {
        self.driver = driver;
        self
    }

    /// Set subnet
    pub fn subnet(mut self, subnet: &str) -> Self {
        self.ipam.config.push(IpamPoolConfig {
            subnet: subnet.to_string(),
            gateway: None,
            ip_range: None,
            aux_addresses: HashMap::new(),
        });
        self
    }

    /// Set gateway
    pub fn gateway(mut self, gateway: &str) -> Self {
        if let Some(pool) = self.ipam.config.last_mut() {
            pool.gateway = Some(gateway.to_string());
        }
        self
    }

    /// Add label
    pub fn label(mut self, key: &str, value: &str) -> Self {
        self.labels.insert(key.to_string(), value.to_string());
        self
    }

    /// Set internal
    pub fn internal(mut self, internal: bool) -> Self {
        self.internal = internal;
        self
    }
}

/// IPAM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpamConfig {
    /// IPAM driver
    pub driver: String,
    /// IP pool configurations
    pub config: Vec<IpamPoolConfig>,
    /// Driver options
    pub options: HashMap<String, String>,
}

impl Default for IpamConfig {
    fn default() -> Self {
        Self {
            driver: "default".to_string(),
            config: vec![IpamPoolConfig::default()],
            options: HashMap::new(),
        }
    }
}

/// IPAM pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpamPoolConfig {
    /// Subnet in CIDR format
    pub subnet: String,
    /// Gateway address
    pub gateway: Option<String>,
    /// IP range
    pub ip_range: Option<String>,
    /// Auxiliary addresses
    pub aux_addresses: HashMap<String, String>,
}

impl Default for IpamPoolConfig {
    fn default() -> Self {
        Self {
            subnet: "172.17.0.0/16".to_string(),
            gateway: Some("172.17.0.1".to_string()),
            ip_range: None,
            aux_addresses: HashMap::new(),
        }
    }
}

/// Container network connection info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkContainer {
    /// Container ID
    pub container_id: String,
    /// Container name
    pub name: String,
    /// Endpoint ID
    pub endpoint_id: String,
    /// MAC address
    pub mac_address: String,
    /// IPv4 address
    pub ipv4_address: Option<String>,
    /// IPv6 address
    pub ipv6_address: Option<String>,
}

/// IP address allocator
#[allow(dead_code)]
pub struct IpAllocator {
    /// Network subnet
    subnet: String,
    /// Allocated addresses
    allocated: Vec<Ipv4Addr>,
    /// Next available address
    next: Ipv4Addr,
}

impl IpAllocator {
    /// Create a new IP allocator for a subnet
    pub fn new(subnet: &str) -> Result<Self> {
        // Parse subnet (e.g., "172.17.0.0/16")
        let parts: Vec<&str> = subnet.split('/').collect();
        if parts.len() != 2 {
            return Err(RuneError::Network(format!("Invalid subnet: {}", subnet)));
        }

        let base: Ipv4Addr = parts[0]
            .parse()
            .map_err(|_| RuneError::Network(format!("Invalid IP: {}", parts[0])))?;

        // Start from .2 (gateway is typically .1)
        let octets = base.octets();
        let next = Ipv4Addr::new(octets[0], octets[1], octets[2], 2);

        Ok(Self {
            subnet: subnet.to_string(),
            allocated: vec![Ipv4Addr::new(octets[0], octets[1], octets[2], 1)], // Reserve gateway
            next,
        })
    }

    /// Allocate an IP address
    pub fn allocate(&mut self) -> Result<Ipv4Addr> {
        let ip = self.next;

        if self.allocated.contains(&ip) {
            // Find next available
            let mut octets = ip.octets();
            loop {
                octets[3] = octets[3].wrapping_add(1);
                if octets[3] == 0 {
                    octets[2] = octets[2].wrapping_add(1);
                }
                let candidate = Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]);
                if !self.allocated.contains(&candidate) {
                    self.allocated.push(candidate);
                    self.next =
                        Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3].wrapping_add(1));
                    return Ok(candidate);
                }
            }
        }

        self.allocated.push(ip);
        let octets = ip.octets();
        self.next = Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3].wrapping_add(1));

        Ok(ip)
    }

    /// Release an IP address
    pub fn release(&mut self, ip: Ipv4Addr) {
        self.allocated.retain(|&a| a != ip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert_eq!(config.driver, NetworkDriver::Bridge);
        assert_eq!(config.scope, NetworkScope::Local);
    }

    #[test]
    fn test_network_config_builder() {
        let config = NetworkConfig::new("my-network")
            .driver(NetworkDriver::Overlay)
            .subnet("10.0.0.0/24")
            .gateway("10.0.0.1")
            .internal(true);

        assert_eq!(config.name, "my-network");
        assert_eq!(config.driver, NetworkDriver::Overlay);
        assert!(config.internal);
    }

    #[test]
    fn test_ip_allocator() {
        let mut allocator = IpAllocator::new("172.17.0.0/16").unwrap();

        let ip1 = allocator.allocate().unwrap();
        assert_eq!(ip1, Ipv4Addr::new(172, 17, 0, 2));

        let ip2 = allocator.allocate().unwrap();
        assert_eq!(ip2, Ipv4Addr::new(172, 17, 0, 3));

        allocator.release(ip1);
    }
}
