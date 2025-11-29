//! Mount management for containers
//!
//! Provides functionality for setting up container filesystems,
//! including pivot_root and bind mounts.

use crate::error::{Result, RuneError};
use super::syscall::{mount, umount2, pivot_root, chroot, chdir, mount_flags, umount_flags};
use std::fs;
use std::path::Path;

/// Mount entry for a container
#[derive(Debug, Clone)]
pub struct MountEntry {
    /// Source path (or "none" for virtual filesystems)
    pub source: Option<String>,
    /// Target path inside the container
    pub target: String,
    /// Filesystem type
    pub fs_type: Option<String>,
    /// Mount flags
    pub flags: u64,
    /// Mount options
    pub options: Option<String>,
}

impl MountEntry {
    /// Create a new mount entry
    pub fn new(target: &str) -> Self {
        Self {
            source: None,
            target: target.to_string(),
            fs_type: None,
            flags: 0,
            options: None,
        }
    }

    /// Set the source
    pub fn source(mut self, source: &str) -> Self {
        self.source = Some(source.to_string());
        self
    }

    /// Set the filesystem type
    pub fn fs_type(mut self, fs_type: &str) -> Self {
        self.fs_type = Some(fs_type.to_string());
        self
    }

    /// Add mount flags
    pub fn flags(mut self, flags: u64) -> Self {
        self.flags = flags;
        self
    }

    /// Set mount options
    pub fn options(mut self, options: &str) -> Self {
        self.options = Some(options.to_string());
        self
    }

    /// Create a bind mount
    pub fn bind(source: &str, target: &str) -> Self {
        Self {
            source: Some(source.to_string()),
            target: target.to_string(),
            fs_type: None,
            flags: mount_flags::MS_BIND,
            options: None,
        }
    }

    /// Create a recursive bind mount
    pub fn rbind(source: &str, target: &str) -> Self {
        Self {
            source: Some(source.to_string()),
            target: target.to_string(),
            fs_type: None,
            flags: mount_flags::MS_BIND | mount_flags::MS_REC,
            options: None,
        }
    }

    /// Create a tmpfs mount
    pub fn tmpfs(target: &str, size: Option<&str>) -> Self {
        let options = size.map(|s| format!("size={}", s));
        Self {
            source: Some("tmpfs".to_string()),
            target: target.to_string(),
            fs_type: Some("tmpfs".to_string()),
            flags: mount_flags::MS_NOSUID | mount_flags::MS_NODEV,
            options,
        }
    }

    /// Create a proc mount
    pub fn proc(target: &str) -> Self {
        Self {
            source: Some("proc".to_string()),
            target: target.to_string(),
            fs_type: Some("proc".to_string()),
            flags: mount_flags::MS_NOSUID | mount_flags::MS_NODEV | mount_flags::MS_NOEXEC,
            options: None,
        }
    }

    /// Create a sysfs mount
    pub fn sysfs(target: &str) -> Self {
        Self {
            source: Some("sysfs".to_string()),
            target: target.to_string(),
            fs_type: Some("sysfs".to_string()),
            flags: mount_flags::MS_NOSUID | mount_flags::MS_NODEV | mount_flags::MS_NOEXEC | mount_flags::MS_RDONLY,
            options: None,
        }
    }

    /// Create a devpts mount
    pub fn devpts(target: &str) -> Self {
        Self {
            source: Some("devpts".to_string()),
            target: target.to_string(),
            fs_type: Some("devpts".to_string()),
            flags: mount_flags::MS_NOSUID | mount_flags::MS_NOEXEC,
            options: Some("newinstance,ptmxmode=0666,mode=0620".to_string()),
        }
    }

    /// Create a cgroup mount
    pub fn cgroup(target: &str) -> Self {
        Self {
            source: Some("cgroup2".to_string()),
            target: target.to_string(),
            fs_type: Some("cgroup2".to_string()),
            flags: mount_flags::MS_NOSUID | mount_flags::MS_NODEV | mount_flags::MS_NOEXEC,
            options: None,
        }
    }
}

/// Mount manager for container filesystem setup
pub struct MountManager {
    /// List of default mounts
    default_mounts: Vec<MountEntry>,
}

impl MountManager {
    /// Create a new mount manager
    pub fn new() -> Self {
        Self {
            default_mounts: Self::create_default_mounts(),
        }
    }

    /// Create default mount entries for containers
    fn create_default_mounts() -> Vec<MountEntry> {
        vec![
            MountEntry::proc("/proc"),
            MountEntry::sysfs("/sys"),
            MountEntry::tmpfs("/dev", Some("65536k")),
            MountEntry::devpts("/dev/pts"),
            MountEntry::tmpfs("/dev/shm", None),
            MountEntry::tmpfs("/run", None),
        ]
    }

    /// Get default mounts
    pub fn default_mounts(&self) -> &[MountEntry] {
        &self.default_mounts
    }

    /// Setup the root filesystem for a container
    pub fn setup_rootfs(&self, rootfs: &str) -> Result<()> {
        // Make sure rootfs exists
        if !Path::new(rootfs).exists() {
            return Err(RuneError::Runtime(format!("Root filesystem does not exist: {}", rootfs)));
        }

        // Mount rootfs as a bind mount to itself (required for pivot_root)
        mount(
            Some(rootfs),
            rootfs,
            None,
            mount_flags::MS_BIND | mount_flags::MS_REC,
            None,
        ).map_err(|e| RuneError::Runtime(format!("Failed to bind mount rootfs: {}", e)))?;

        // Setup default mounts
        for entry in &self.default_mounts {
            let target = format!("{}{}", rootfs, entry.target);
            
            // Create mount point if it doesn't exist
            if !Path::new(&target).exists() {
                fs::create_dir_all(&target)
                    .map_err(|e| RuneError::Runtime(format!("Failed to create mount point {}: {}", target, e)))?;
            }

            let result = mount(
                entry.source.as_deref(),
                &target,
                entry.fs_type.as_deref(),
                entry.flags,
                entry.options.as_deref(),
            );

            // Some mounts might fail (e.g., sysfs without privilege), continue with others
            if let Err(e) = result {
                tracing::warn!("Failed to mount {}: {}", target, e);
            }
        }

        Ok(())
    }

    /// Pivot root to the new filesystem
    pub fn pivot_root(&self, new_root: &str, put_old: &str) -> Result<()> {
        // Create put_old directory
        let put_old_path = format!("{}{}", new_root, put_old);
        if !Path::new(&put_old_path).exists() {
            fs::create_dir_all(&put_old_path)
                .map_err(|e| RuneError::Runtime(format!("Failed to create put_old directory: {}", e)))?;
        }

        // Perform pivot_root
        pivot_root(new_root, &put_old_path)
            .map_err(|e| RuneError::Runtime(format!("Failed to pivot_root: {}", e)))?;

        // Change to new root
        chdir("/")
            .map_err(|e| RuneError::Runtime(format!("Failed to chdir to /: {}", e)))?;

        // Unmount old root
        umount2(put_old, umount_flags::MNT_DETACH)
            .map_err(|e| RuneError::Runtime(format!("Failed to unmount old root: {}", e)))?;

        // Remove put_old directory
        let _ = fs::remove_dir(put_old);

        Ok(())
    }

    /// Alternative: use chroot instead of pivot_root
    pub fn chroot_to(&self, new_root: &str) -> Result<()> {
        chroot(new_root)
            .map_err(|e| RuneError::Runtime(format!("Failed to chroot: {}", e)))?;

        chdir("/")
            .map_err(|e| RuneError::Runtime(format!("Failed to chdir to /: {}", e)))?;

        Ok(())
    }

    /// Mount a volume into the container
    pub fn mount_volume(&self, source: &str, target: &str, read_only: bool) -> Result<()> {
        let mut flags = mount_flags::MS_BIND;
        if read_only {
            flags |= mount_flags::MS_RDONLY;
        }

        // Create target directory if it doesn't exist
        if !Path::new(target).exists() {
            fs::create_dir_all(target)
                .map_err(|e| RuneError::Runtime(format!("Failed to create mount point {}: {}", target, e)))?;
        }

        mount(Some(source), target, None, flags, None)
            .map_err(|e| RuneError::Runtime(format!("Failed to mount volume {} to {}: {}", source, target, e)))?;

        // For read-only, need to remount with the flag
        if read_only {
            mount(None, target, None, flags | mount_flags::MS_REMOUNT, None)
                .map_err(|e| RuneError::Runtime(format!("Failed to make mount read-only: {}", e)))?;
        }

        Ok(())
    }

    /// Unmount a path
    pub fn unmount(&self, target: &str) -> Result<()> {
        umount2(target, umount_flags::MNT_DETACH)
            .map_err(|e| RuneError::Runtime(format!("Failed to unmount {}: {}", target, e)))
    }

    /// Make a mount private (propagation)
    pub fn make_private(&self, target: &str) -> Result<()> {
        mount(None, target, None, mount_flags::MS_REC | mount_flags::MS_PRIVATE, None)
            .map_err(|e| RuneError::Runtime(format!("Failed to make mount private: {}", e)))
    }

    /// Make a mount shared (propagation)
    pub fn make_shared(&self, target: &str) -> Result<()> {
        mount(None, target, None, mount_flags::MS_REC | mount_flags::MS_SHARED, None)
            .map_err(|e| RuneError::Runtime(format!("Failed to make mount shared: {}", e)))
    }

    /// Make a mount slave (propagation)
    pub fn make_slave(&self, target: &str) -> Result<()> {
        mount(None, target, None, mount_flags::MS_REC | mount_flags::MS_SLAVE, None)
            .map_err(|e| RuneError::Runtime(format!("Failed to make mount slave: {}", e)))
    }

    /// Create device nodes in /dev
    pub fn create_devices(&self, rootfs: &str) -> Result<()> {
        let dev_path = format!("{}/dev", rootfs);
        
        // These devices are typically needed
        let devices = [
            ("null", 1, 3, 0o666),
            ("zero", 1, 5, 0o666),
            ("full", 1, 7, 0o666),
            ("random", 1, 8, 0o666),
            ("urandom", 1, 9, 0o666),
            ("tty", 5, 0, 0o666),
        ];

        for (name, major, minor, mode) in devices {
            let path = format!("{}/{}", dev_path, name);
            
            // Skip if already exists
            if Path::new(&path).exists() {
                continue;
            }

            // We would normally use mknod here, but that requires root
            // For now, we'll bind mount from the host
            let host_path = format!("/dev/{}", name);
            if Path::new(&host_path).exists() {
                // Create an empty file to bind mount to
                match fs::File::create(&path) {
                    Ok(_) => {
                        if let Err(e) = mount(Some(&host_path), &path, None, mount_flags::MS_BIND, None) {
                            tracing::warn!("Failed to bind mount device {}: {}", name, e);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create device file {}: {}", path, e);
                    }
                }
            }
        }

        // Create symlinks
        let symlinks = [
            ("fd", "/proc/self/fd"),
            ("stdin", "/proc/self/fd/0"),
            ("stdout", "/proc/self/fd/1"),
            ("stderr", "/proc/self/fd/2"),
            ("ptmx", "pts/ptmx"),
        ];

        for (name, target) in symlinks {
            let path = format!("{}/{}", dev_path, name);
            if !Path::new(&path).exists() {
                let _ = std::os::unix::fs::symlink(target, &path);
            }
        }

        Ok(())
    }
}

impl Default for MountManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_entry_creation() {
        let entry = MountEntry::new("/test");
        assert_eq!(entry.target, "/test");
        assert!(entry.source.is_none());
        assert!(entry.fs_type.is_none());
    }

    #[test]
    fn test_bind_mount_entry() {
        let entry = MountEntry::bind("/source", "/target");
        assert_eq!(entry.source, Some("/source".to_string()));
        assert_eq!(entry.target, "/target");
        assert!(entry.flags & mount_flags::MS_BIND != 0);
    }

    #[test]
    fn test_tmpfs_mount_entry() {
        let entry = MountEntry::tmpfs("/tmp", Some("100M"));
        assert_eq!(entry.fs_type, Some("tmpfs".to_string()));
        assert_eq!(entry.options, Some("size=100M".to_string()));
    }

    #[test]
    fn test_proc_mount_entry() {
        let entry = MountEntry::proc("/proc");
        assert_eq!(entry.fs_type, Some("proc".to_string()));
        assert!(entry.flags & mount_flags::MS_NOSUID != 0);
    }

    #[test]
    fn test_mount_manager_default_mounts() {
        let manager = MountManager::new();
        let mounts = manager.default_mounts();
        
        // Should have several default mounts
        assert!(!mounts.is_empty());
        
        // Check for essential mounts
        assert!(mounts.iter().any(|m| m.target == "/proc"));
        assert!(mounts.iter().any(|m| m.target == "/sys"));
        assert!(mounts.iter().any(|m| m.target == "/dev"));
    }
}
