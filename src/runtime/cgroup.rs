//! Cgroup (Control Groups) management
//!
//! Provides functionality for creating and managing cgroups
//! for container resource isolation and limits.

use crate::error::{Result, RuneError};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Cgroup version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupVersion {
    /// Cgroup v1 (legacy)
    V1,
    /// Cgroup v2 (unified)
    V2,
}

/// Cgroup controller types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupController {
    /// CPU controller
    Cpu,
    /// CPU accounting
    Cpuacct,
    /// CPU set (pin to CPUs)
    Cpuset,
    /// Memory controller
    Memory,
    /// Block I/O controller
    Blkio,
    /// PIDs controller
    Pids,
    /// Devices controller
    Devices,
    /// Freezer controller
    Freezer,
    /// Network class ID
    NetCls,
    /// Network priority
    NetPrio,
    /// Huge pages
    Hugetlb,
    /// Performance events
    Perf,
}

impl CgroupController {
    /// Get the controller name for cgroup v1
    pub fn v1_name(&self) -> &'static str {
        match self {
            CgroupController::Cpu => "cpu",
            CgroupController::Cpuacct => "cpuacct",
            CgroupController::Cpuset => "cpuset",
            CgroupController::Memory => "memory",
            CgroupController::Blkio => "blkio",
            CgroupController::Pids => "pids",
            CgroupController::Devices => "devices",
            CgroupController::Freezer => "freezer",
            CgroupController::NetCls => "net_cls",
            CgroupController::NetPrio => "net_prio",
            CgroupController::Hugetlb => "hugetlb",
            CgroupController::Perf => "perf_event",
        }
    }

    /// Get the controller name for cgroup v2
    pub fn v2_name(&self) -> &'static str {
        match self {
            CgroupController::Cpu => "cpu",
            CgroupController::Cpuacct => "cpu", // merged in v2
            CgroupController::Cpuset => "cpuset",
            CgroupController::Memory => "memory",
            CgroupController::Blkio => "io", // renamed in v2
            CgroupController::Pids => "pids",
            CgroupController::Devices => "devices",
            CgroupController::Freezer => "freezer",
            CgroupController::NetCls => "net_cls",
            CgroupController::NetPrio => "net_prio",
            CgroupController::Hugetlb => "hugetlb",
            CgroupController::Perf => "perf_event",
        }
    }
}

/// Cgroup configuration
#[derive(Debug, Clone, Default)]
pub struct CgroupConfig {
    /// Memory limit in bytes
    pub memory_limit: Option<u64>,
    /// Memory soft limit in bytes
    pub memory_reservation: Option<u64>,
    /// Memory swap limit in bytes
    pub memory_swap_limit: Option<i64>,
    /// CPU shares
    pub cpu_shares: Option<u64>,
    /// CPU quota (in microseconds per period)
    pub cpu_quota: Option<i64>,
    /// CPU period (in microseconds)
    pub cpu_period: Option<u64>,
    /// Number of CPUs (used to calculate quota)
    pub cpus: Option<f64>,
    /// CPUs to use (comma-separated list or range)
    pub cpuset_cpus: Option<String>,
    /// Memory nodes to use
    pub cpuset_mems: Option<String>,
    /// PIDs limit
    pub pids_limit: Option<i64>,
    /// Block I/O weight
    pub blkio_weight: Option<u16>,
    /// OOM kill disable
    pub oom_kill_disable: bool,
}

/// Cgroup manager for container resource limits
pub struct CgroupManager {
    /// Cgroup version in use
    version: CgroupVersion,
    /// Base path for cgroups
    base_path: PathBuf,
    /// Rune cgroup path
    rune_path: PathBuf,
}

impl CgroupManager {
    /// Create a new cgroup manager
    pub fn new() -> Result<Self> {
        let version = Self::detect_version()?;
        let base_path = match version {
            CgroupVersion::V1 => PathBuf::from("/sys/fs/cgroup"),
            CgroupVersion::V2 => PathBuf::from("/sys/fs/cgroup"),
        };
        let rune_path = base_path.join("rune");

        Ok(Self {
            version,
            base_path,
            rune_path,
        })
    }

    /// Detect the cgroup version in use
    fn detect_version() -> Result<CgroupVersion> {
        // Check for cgroup v2 unified hierarchy
        let cgroup2_path = Path::new("/sys/fs/cgroup/cgroup.controllers");
        if cgroup2_path.exists() {
            return Ok(CgroupVersion::V2);
        }

        // Check for cgroup v1
        let cgroup1_path = Path::new("/sys/fs/cgroup/memory");
        if cgroup1_path.exists() {
            return Ok(CgroupVersion::V1);
        }

        // Default to v2 if we can't detect
        Ok(CgroupVersion::V2)
    }

    /// Get the cgroup version
    pub fn version(&self) -> CgroupVersion {
        self.version
    }

    /// Create a cgroup for a container
    pub fn create(&self, container_id: &str, config: &CgroupConfig) -> Result<()> {
        match self.version {
            CgroupVersion::V1 => self.create_v1(container_id, config),
            CgroupVersion::V2 => self.create_v2(container_id, config),
        }
    }

    /// Create cgroup v1 hierarchy
    fn create_v1(&self, container_id: &str, config: &CgroupConfig) -> Result<()> {
        // Create memory cgroup
        if config.memory_limit.is_some() || config.memory_reservation.is_some() {
            let memory_path = self.base_path.join("memory/rune").join(container_id);
            self.create_cgroup_dir(&memory_path)?;

            if let Some(limit) = config.memory_limit {
                self.write_cgroup_file(
                    &memory_path.join("memory.limit_in_bytes"),
                    &limit.to_string(),
                )?;
            }
            if let Some(reservation) = config.memory_reservation {
                self.write_cgroup_file(
                    &memory_path.join("memory.soft_limit_in_bytes"),
                    &reservation.to_string(),
                )?;
            }
            if config.oom_kill_disable {
                self.write_cgroup_file(&memory_path.join("memory.oom_control"), "1")?;
            }
        }

        // Create CPU cgroup
        if config.cpu_shares.is_some() || config.cpu_quota.is_some() || config.cpus.is_some() {
            let cpu_path = self.base_path.join("cpu/rune").join(container_id);
            self.create_cgroup_dir(&cpu_path)?;

            if let Some(shares) = config.cpu_shares {
                self.write_cgroup_file(&cpu_path.join("cpu.shares"), &shares.to_string())?;
            }
            if let Some(quota) = config.cpu_quota {
                self.write_cgroup_file(&cpu_path.join("cpu.cfs_quota_us"), &quota.to_string())?;
            }
            if let Some(period) = config.cpu_period {
                self.write_cgroup_file(&cpu_path.join("cpu.cfs_period_us"), &period.to_string())?;
            }
            // Handle --cpus option by converting to quota
            if let Some(cpus) = config.cpus {
                let period = config.cpu_period.unwrap_or(100_000);
                let quota = (cpus * period as f64) as i64;
                self.write_cgroup_file(&cpu_path.join("cpu.cfs_period_us"), &period.to_string())?;
                self.write_cgroup_file(&cpu_path.join("cpu.cfs_quota_us"), &quota.to_string())?;
            }
        }

        // Create cpuset cgroup
        if config.cpuset_cpus.is_some() || config.cpuset_mems.is_some() {
            let cpuset_path = self.base_path.join("cpuset/rune").join(container_id);
            self.create_cgroup_dir(&cpuset_path)?;

            if let Some(ref cpus) = config.cpuset_cpus {
                self.write_cgroup_file(&cpuset_path.join("cpuset.cpus"), cpus)?;
            }
            if let Some(ref mems) = config.cpuset_mems {
                self.write_cgroup_file(&cpuset_path.join("cpuset.mems"), mems)?;
            }
        }

        // Create PIDs cgroup
        if let Some(limit) = config.pids_limit {
            let pids_path = self.base_path.join("pids/rune").join(container_id);
            self.create_cgroup_dir(&pids_path)?;
            self.write_cgroup_file(&pids_path.join("pids.max"), &limit.to_string())?;
        }

        // Create blkio cgroup
        if let Some(weight) = config.blkio_weight {
            let blkio_path = self.base_path.join("blkio/rune").join(container_id);
            self.create_cgroup_dir(&blkio_path)?;
            self.write_cgroup_file(&blkio_path.join("blkio.weight"), &weight.to_string())?;
        }

        Ok(())
    }

    /// Create cgroup v2 unified hierarchy
    fn create_v2(&self, container_id: &str, config: &CgroupConfig) -> Result<()> {
        let container_path = self.rune_path.join(container_id);
        self.create_cgroup_dir(&container_path)?;

        // Enable controllers on the rune cgroup first
        // This may fail if controllers are already enabled or not available
        if let Err(e) = self.write_cgroup_file(
            &self.rune_path.join("cgroup.subtree_control"),
            "+cpu +memory +pids +io",
        ) {
            tracing::warn!(
                "Failed to enable cgroup controllers (may already be enabled): {}",
                e
            );
        }

        // Memory settings
        if let Some(limit) = config.memory_limit {
            self.write_cgroup_file(&container_path.join("memory.max"), &limit.to_string())?;
        }
        if let Some(reservation) = config.memory_reservation {
            self.write_cgroup_file(&container_path.join("memory.low"), &reservation.to_string())?;
        }
        if let Some(swap_limit) = config.memory_swap_limit {
            if swap_limit < 0 {
                self.write_cgroup_file(&container_path.join("memory.swap.max"), "max")?;
            } else {
                self.write_cgroup_file(
                    &container_path.join("memory.swap.max"),
                    &swap_limit.to_string(),
                )?;
            }
        }

        // CPU settings
        if let Some(cpus) = config.cpus {
            let period = config.cpu_period.unwrap_or(100_000);
            let quota = (cpus * period as f64) as u64;
            self.write_cgroup_file(
                &container_path.join("cpu.max"),
                &format!("{} {}", quota, period),
            )?;
        } else if let Some(quota) = config.cpu_quota {
            let period = config.cpu_period.unwrap_or(100_000);
            self.write_cgroup_file(
                &container_path.join("cpu.max"),
                &format!("{} {}", quota, period),
            )?;
        }
        if let Some(shares) = config.cpu_shares {
            // Convert shares to weight (shares 1024 = weight 100)
            let weight = ((shares * 100) / 1024).clamp(1, 10000);
            self.write_cgroup_file(&container_path.join("cpu.weight"), &weight.to_string())?;
        }

        // Cpuset settings
        if let Some(ref cpus) = config.cpuset_cpus {
            self.write_cgroup_file(&container_path.join("cpuset.cpus"), cpus)?;
        }
        if let Some(ref mems) = config.cpuset_mems {
            self.write_cgroup_file(&container_path.join("cpuset.mems"), mems)?;
        }

        // PIDs limit
        if let Some(limit) = config.pids_limit {
            self.write_cgroup_file(&container_path.join("pids.max"), &limit.to_string())?;
        }

        // IO weight
        if let Some(weight) = config.blkio_weight {
            // Convert to cgroup v2 io.weight (1-10000)
            let io_weight = (weight as u64).clamp(1, 10000);
            self.write_cgroup_file(&container_path.join("io.weight"), &io_weight.to_string())?;
        }

        Ok(())
    }

    /// Add a process to the cgroup
    pub fn add_process(&self, container_id: &str, pid: u32) -> Result<()> {
        match self.version {
            CgroupVersion::V1 => self.add_process_v1(container_id, pid),
            CgroupVersion::V2 => self.add_process_v2(container_id, pid),
        }
    }

    fn add_process_v1(&self, container_id: &str, pid: u32) -> Result<()> {
        let controllers = ["memory", "cpu", "cpuset", "pids", "blkio"];

        for controller in controllers {
            let cgroup_path = self
                .base_path
                .join(controller)
                .join("rune")
                .join(container_id);
            if cgroup_path.exists() {
                let procs_file = cgroup_path.join("cgroup.procs");
                if procs_file.exists() {
                    self.write_cgroup_file(&procs_file, &pid.to_string())?;
                }
            }
        }

        Ok(())
    }

    fn add_process_v2(&self, container_id: &str, pid: u32) -> Result<()> {
        let container_path = self.rune_path.join(container_id);
        let procs_file = container_path.join("cgroup.procs");

        if procs_file.exists() {
            self.write_cgroup_file(&procs_file, &pid.to_string())?;
        }

        Ok(())
    }

    /// Remove a cgroup
    pub fn remove(&self, container_id: &str) -> Result<()> {
        match self.version {
            CgroupVersion::V1 => self.remove_v1(container_id),
            CgroupVersion::V2 => self.remove_v2(container_id),
        }
    }

    fn remove_v1(&self, container_id: &str) -> Result<()> {
        let controllers = ["memory", "cpu", "cpuset", "pids", "blkio"];

        for controller in controllers {
            let cgroup_path = self
                .base_path
                .join(controller)
                .join("rune")
                .join(container_id);
            if cgroup_path.exists() {
                let _ = fs::remove_dir(&cgroup_path);
            }
        }

        Ok(())
    }

    fn remove_v2(&self, container_id: &str) -> Result<()> {
        let container_path = self.rune_path.join(container_id);
        if container_path.exists() {
            fs::remove_dir(&container_path)
                .map_err(|e| RuneError::Runtime(format!("Failed to remove cgroup: {}", e)))?;
        }
        Ok(())
    }

    /// Freeze all processes in a cgroup
    pub fn freeze(&self, container_id: &str) -> Result<()> {
        match self.version {
            CgroupVersion::V1 => {
                let freezer_path = self.base_path.join("freezer/rune").join(container_id);
                self.write_cgroup_file(&freezer_path.join("freezer.state"), "FROZEN")
            }
            CgroupVersion::V2 => {
                let container_path = self.rune_path.join(container_id);
                self.write_cgroup_file(&container_path.join("cgroup.freeze"), "1")
            }
        }
    }

    /// Thaw (unfreeze) all processes in a cgroup
    pub fn thaw(&self, container_id: &str) -> Result<()> {
        match self.version {
            CgroupVersion::V1 => {
                let freezer_path = self.base_path.join("freezer/rune").join(container_id);
                self.write_cgroup_file(&freezer_path.join("freezer.state"), "THAWED")
            }
            CgroupVersion::V2 => {
                let container_path = self.rune_path.join(container_id);
                self.write_cgroup_file(&container_path.join("cgroup.freeze"), "0")
            }
        }
    }

    /// Get memory statistics
    pub fn get_memory_stats(&self, container_id: &str) -> Result<MemoryStats> {
        match self.version {
            CgroupVersion::V1 => self.get_memory_stats_v1(container_id),
            CgroupVersion::V2 => self.get_memory_stats_v2(container_id),
        }
    }

    fn get_memory_stats_v1(&self, container_id: &str) -> Result<MemoryStats> {
        let memory_path = self.base_path.join("memory/rune").join(container_id);

        let usage = self.read_cgroup_u64(&memory_path.join("memory.usage_in_bytes"))?;
        let limit = self.read_cgroup_u64(&memory_path.join("memory.limit_in_bytes"))?;
        let max_usage = self
            .read_cgroup_u64(&memory_path.join("memory.max_usage_in_bytes"))
            .unwrap_or(0);

        Ok(MemoryStats {
            usage,
            limit,
            max_usage,
        })
    }

    fn get_memory_stats_v2(&self, container_id: &str) -> Result<MemoryStats> {
        let container_path = self.rune_path.join(container_id);

        let usage = self.read_cgroup_u64(&container_path.join("memory.current"))?;
        let limit_str = fs::read_to_string(container_path.join("memory.max"))
            .map_err(|e| RuneError::Runtime(format!("Failed to read memory.max: {}", e)))?;
        let limit = if limit_str.trim() == "max" {
            u64::MAX
        } else {
            limit_str.trim().parse().unwrap_or(u64::MAX)
        };
        let max_usage = self
            .read_cgroup_u64(&container_path.join("memory.peak"))
            .unwrap_or(0);

        Ok(MemoryStats {
            usage,
            limit,
            max_usage,
        })
    }

    /// Create cgroup directory
    fn create_cgroup_dir(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            fs::create_dir_all(path).map_err(|e| {
                RuneError::Runtime(format!("Failed to create cgroup directory: {}", e))
            })?;
        }
        Ok(())
    }

    /// Write to a cgroup file
    fn write_cgroup_file(&self, path: &Path, value: &str) -> Result<()> {
        let mut file = OpenOptions::new().write(true).open(path).map_err(|e| {
            RuneError::Runtime(format!("Failed to open cgroup file {:?}: {}", path, e))
        })?;

        file.write_all(value.as_bytes()).map_err(|e| {
            RuneError::Runtime(format!("Failed to write cgroup file {:?}: {}", path, e))
        })?;

        Ok(())
    }

    /// Read a u64 from a cgroup file
    fn read_cgroup_u64(&self, path: &Path) -> Result<u64> {
        let content = fs::read_to_string(path).map_err(|e| {
            RuneError::Runtime(format!("Failed to read cgroup file {:?}: {}", path, e))
        })?;

        content
            .trim()
            .parse()
            .map_err(|e| RuneError::Runtime(format!("Failed to parse cgroup value: {}", e)))
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Current memory usage in bytes
    pub usage: u64,
    /// Memory limit in bytes
    pub limit: u64,
    /// Maximum memory usage in bytes
    pub max_usage: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cgroup_controller_names() {
        assert_eq!(CgroupController::Cpu.v1_name(), "cpu");
        assert_eq!(CgroupController::Memory.v1_name(), "memory");
        assert_eq!(CgroupController::Blkio.v1_name(), "blkio");
        assert_eq!(CgroupController::Blkio.v2_name(), "io");
    }

    #[test]
    fn test_cgroup_config_default() {
        let config = CgroupConfig::default();
        assert!(config.memory_limit.is_none());
        assert!(config.cpu_shares.is_none());
        assert!(config.pids_limit.is_none());
        assert!(!config.oom_kill_disable);
    }

    #[test]
    fn test_cgroup_manager_creation() {
        // This might fail in non-Linux environments, just ensure no panic
        let _ = CgroupManager::new();
    }
}
