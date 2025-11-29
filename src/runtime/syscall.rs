//! Raw system call bindings for Linux
//!
//! This module provides direct syscall wrappers without depending on external crates.
//! All syscalls are made using Rust's inline assembly or libc bindings.

use std::io;

/// Clone flags for namespace creation
pub mod clone_flags {
    /// Create new mount namespace
    pub const CLONE_NEWNS: i32 = 0x00020000;
    /// Create new UTS namespace
    pub const CLONE_NEWUTS: i32 = 0x04000000;
    /// Create new IPC namespace
    pub const CLONE_NEWIPC: i32 = 0x08000000;
    /// Create new network namespace
    pub const CLONE_NEWNET: i32 = 0x40000000;
    /// Create new PID namespace
    pub const CLONE_NEWPID: i32 = 0x20000000;
    /// Create new user namespace
    pub const CLONE_NEWUSER: i32 = 0x10000000;
    /// Create new cgroup namespace
    pub const CLONE_NEWCGROUP: i32 = 0x02000000;
}

/// Mount flags
pub mod mount_flags {
    /// Mount read-only
    pub const MS_RDONLY: u64 = 1;
    /// Ignore suid and sgid bits
    pub const MS_NOSUID: u64 = 2;
    /// Disallow access to device special files
    pub const MS_NODEV: u64 = 4;
    /// Disallow program execution
    pub const MS_NOEXEC: u64 = 8;
    /// Writes are synced at once
    pub const MS_SYNCHRONOUS: u64 = 16;
    /// Alter flags of a mounted FS
    pub const MS_REMOUNT: u64 = 32;
    /// Allow mandatory locks on an FS
    pub const MS_MANDLOCK: u64 = 64;
    /// Directory modifications are synchronous
    pub const MS_DIRSYNC: u64 = 128;
    /// Do not update access times
    pub const MS_NOATIME: u64 = 1024;
    /// Do not update directory access times
    pub const MS_NODIRATIME: u64 = 2048;
    /// Bind directory at different place
    pub const MS_BIND: u64 = 4096;
    /// Move a subtree
    pub const MS_MOVE: u64 = 8192;
    /// Recursive mount
    pub const MS_REC: u64 = 16384;
    /// Silent failure
    pub const MS_SILENT: u64 = 32768;
    /// VFS does not apply the umask
    pub const MS_POSIXACL: u64 = 1 << 16;
    /// Change to private propagation
    pub const MS_PRIVATE: u64 = 1 << 18;
    /// Change to slave propagation
    pub const MS_SLAVE: u64 = 1 << 19;
    /// Change to shared propagation
    pub const MS_SHARED: u64 = 1 << 20;
    /// Update atime relative to mtime/ctime
    pub const MS_RELATIME: u64 = 1 << 21;
}

/// Umount flags
pub mod umount_flags {
    /// Force unmount
    pub const MNT_FORCE: i32 = 1;
    /// Lazy unmount
    pub const MNT_DETACH: i32 = 2;
    /// Mark for expiry
    pub const MNT_EXPIRE: i32 = 4;
    /// Don't follow symlinks
    pub const UMOUNT_NOFOLLOW: i32 = 8;
}

/// Result type for syscall operations
pub type SyscallResult<T> = std::result::Result<T, io::Error>;

/// Perform unshare syscall to create new namespaces
pub fn unshare(flags: i32) -> SyscallResult<()> {
    let result = unsafe { libc::unshare(flags) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Set the hostname
pub fn sethostname(name: &str) -> SyscallResult<()> {
    let result = unsafe {
        libc::sethostname(name.as_ptr() as *const libc::c_char, name.len())
    };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Set the domain name
pub fn setdomainname(name: &str) -> SyscallResult<()> {
    let result = unsafe {
        libc::setdomainname(name.as_ptr() as *const libc::c_char, name.len())
    };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Mount a filesystem
pub fn mount(
    source: Option<&str>,
    target: &str,
    fstype: Option<&str>,
    flags: u64,
    data: Option<&str>,
) -> SyscallResult<()> {
    use std::ffi::CString;

    let source_cstr = source.map(|s| CString::new(s)).transpose()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid source path"))?;
    let target_cstr = CString::new(target).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid target path"))?;
    let fstype_cstr = fstype.map(|s| CString::new(s)).transpose()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid fstype"))?;
    let data_cstr = data.map(|s| CString::new(s)).transpose()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid data"))?;

    let source_ptr = source_cstr.as_ref().map(|s| s.as_ptr()).unwrap_or(std::ptr::null());
    let fstype_ptr = fstype_cstr.as_ref().map(|s| s.as_ptr()).unwrap_or(std::ptr::null());
    let data_ptr = data_cstr.as_ref().map(|s| s.as_ptr() as *const libc::c_void).unwrap_or(std::ptr::null());

    let result = unsafe {
        libc::mount(
            source_ptr,
            target_cstr.as_ptr(),
            fstype_ptr,
            flags,
            data_ptr,
        )
    };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Unmount a filesystem
pub fn umount2(target: &str, flags: i32) -> SyscallResult<()> {
    use std::ffi::CString;

    let target_cstr = CString::new(target).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid target path"))?;

    let result = unsafe { libc::umount2(target_cstr.as_ptr(), flags) };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Pivot root filesystem
pub fn pivot_root(new_root: &str, put_old: &str) -> SyscallResult<()> {
    use std::ffi::CString;

    let new_root_cstr = CString::new(new_root).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid new_root path"))?;
    let put_old_cstr = CString::new(put_old).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid put_old path"))?;

    // pivot_root is not directly in libc, use syscall
    let result = unsafe {
        libc::syscall(
            libc::SYS_pivot_root,
            new_root_cstr.as_ptr(),
            put_old_cstr.as_ptr(),
        )
    };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Change root directory
pub fn chroot(path: &str) -> SyscallResult<()> {
    use std::ffi::CString;

    let path_cstr = CString::new(path).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?;

    let result = unsafe { libc::chroot(path_cstr.as_ptr()) };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Change current directory
pub fn chdir(path: &str) -> SyscallResult<()> {
    use std::ffi::CString;

    let path_cstr = CString::new(path).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?;

    let result = unsafe { libc::chdir(path_cstr.as_ptr()) };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Set user and group IDs
pub fn setuid(uid: u32) -> SyscallResult<()> {
    let result = unsafe { libc::setuid(uid) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn setgid(gid: u32) -> SyscallResult<()> {
    let result = unsafe { libc::setgid(gid) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Fork the current process
pub fn fork() -> SyscallResult<u32> {
    let result = unsafe { libc::fork() };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result as u32)
    }
}

/// Execute a program
pub fn execve(path: &str, args: &[&str], env: &[&str]) -> SyscallResult<()> {
    use std::ffi::CString;

    let path_cstr = CString::new(path).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?;
    
    let args_cstr: std::result::Result<Vec<CString>, _> = args.iter()
        .map(|s| CString::new(*s))
        .collect();
    let args_cstr = args_cstr.map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid argument"))?;
    let mut args_ptr: Vec<*const libc::c_char> = args_cstr.iter()
        .map(|s| s.as_ptr())
        .collect();
    args_ptr.push(std::ptr::null());

    let env_cstr: std::result::Result<Vec<CString>, _> = env.iter()
        .map(|s| CString::new(*s))
        .collect();
    let env_cstr = env_cstr.map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid environment variable"))?;
    let mut env_ptr: Vec<*const libc::c_char> = env_cstr.iter()
        .map(|s| s.as_ptr())
        .collect();
    env_ptr.push(std::ptr::null());

    let result = unsafe {
        libc::execve(
            path_cstr.as_ptr(),
            args_ptr.as_ptr(),
            env_ptr.as_ptr(),
        )
    };

    // execve only returns on error
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Wait for a child process
pub fn waitpid(pid: i32, options: i32) -> SyscallResult<(i32, i32)> {
    let mut status: i32 = 0;
    let result = unsafe { libc::waitpid(pid, &mut status, options) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok((result, status))
    }
}

/// Send a signal to a process
pub fn kill(pid: i32, signal: i32) -> SyscallResult<()> {
    let result = unsafe { libc::kill(pid, signal) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Get the current process ID
pub fn getpid() -> u32 {
    unsafe { libc::getpid() as u32 }
}

/// Get the parent process ID
pub fn getppid() -> u32 {
    unsafe { libc::getppid() as u32 }
}

/// Clone with namespaces
pub fn clone_with_namespaces(
    callback: extern "C" fn(*mut libc::c_void) -> i32,
    stack: &mut [u8],
    flags: i32,
    arg: *mut libc::c_void,
) -> SyscallResult<i32> {
    // Stack grows downward on most architectures
    let stack_top = stack.as_mut_ptr().wrapping_add(stack.len());

    let result = unsafe {
        libc::clone(
            callback,
            stack_top as *mut libc::c_void,
            flags | libc::SIGCHLD,
            arg,
        )
    };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result)
    }
}

/// Set process resource limits
pub fn setrlimit(resource: i32, soft: u64, hard: u64) -> SyscallResult<()> {
    let limit = libc::rlimit {
        rlim_cur: soft,
        rlim_max: hard,
    };

    let result = unsafe { libc::setrlimit(resource as u32, &limit) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Resource limit constants
pub mod rlimit {
    pub const RLIMIT_CPU: i32 = 0;
    pub const RLIMIT_FSIZE: i32 = 1;
    pub const RLIMIT_DATA: i32 = 2;
    pub const RLIMIT_STACK: i32 = 3;
    pub const RLIMIT_CORE: i32 = 4;
    pub const RLIMIT_RSS: i32 = 5;
    pub const RLIMIT_NPROC: i32 = 6;
    pub const RLIMIT_NOFILE: i32 = 7;
    pub const RLIMIT_MEMLOCK: i32 = 8;
    pub const RLIMIT_AS: i32 = 9;
    pub const RLIMIT_LOCKS: i32 = 10;
    pub const RLIMIT_SIGPENDING: i32 = 11;
    pub const RLIMIT_MSGQUEUE: i32 = 12;
    pub const RLIMIT_NICE: i32 = 13;
    pub const RLIMIT_RTPRIO: i32 = 14;
    pub const RLIMIT_RTTIME: i32 = 15;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_getpid() {
        let pid = getpid();
        assert!(pid > 0);
    }

    #[test]
    fn test_getppid() {
        let ppid = getppid();
        assert!(ppid > 0);
    }
}
