//! Unix Socket Server for Rune Daemon
//!
//! Implements a Docker-compatible daemon that listens on a Unix socket.

use super::api::ApiHandler;
use crate::container::ContainerManager;
use crate::error::{Result, RuneError};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error, info};

/// Default socket path for the Rune daemon
pub const DEFAULT_SOCKET_PATH: &str = "/var/run/rune.sock";

/// Rune Daemon configuration
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Unix socket path
    pub socket_path: PathBuf,
    /// Data directory for containers, images, etc.
    pub data_dir: PathBuf,
    /// Enable debug logging
    pub debug: bool,
    /// PID file path
    pub pid_file: PathBuf,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from(DEFAULT_SOCKET_PATH),
            data_dir: PathBuf::from("/var/lib/rune"),
            debug: false,
            pid_file: PathBuf::from("/var/run/rune.pid"),
        }
    }
}

/// Rune Daemon - Unix socket server for container management
pub struct RuneDaemon {
    config: DaemonConfig,
    container_manager: Arc<ContainerManager>,
    api_handler: ApiHandler,
    listener: Option<UnixListener>,
}

impl RuneDaemon {
    /// Create a new daemon instance
    pub fn new(config: DaemonConfig) -> Result<Self> {
        // Create data directories
        fs::create_dir_all(&config.data_dir)?;
        fs::create_dir_all(config.data_dir.join("containers"))?;
        fs::create_dir_all(config.data_dir.join("images"))?;
        fs::create_dir_all(config.data_dir.join("volumes"))?;
        fs::create_dir_all(config.data_dir.join("networks"))?;

        let container_manager =
            Arc::new(ContainerManager::new(config.data_dir.join("containers"))?);

        let api_handler = ApiHandler::new(container_manager.clone());

        Ok(Self {
            config,
            container_manager,
            api_handler,
            listener: None,
        })
    }

    /// Start the daemon and listen for connections
    pub fn run(&mut self) -> Result<()> {
        // Remove existing socket if present
        if self.config.socket_path.exists() {
            fs::remove_file(&self.config.socket_path)?;
        }

        // Create parent directory for socket if needed
        if let Some(parent) = self.config.socket_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write PID file
        let pid = std::process::id();
        fs::write(&self.config.pid_file, pid.to_string())?;

        // Create Unix socket listener
        let listener = UnixListener::bind(&self.config.socket_path)?;

        // Set socket permissions (rw-rw-rw-)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o666);
            fs::set_permissions(&self.config.socket_path, permissions)?;
        }

        info!(
            "Rune daemon listening on {}",
            self.config.socket_path.display()
        );

        self.listener = Some(listener);

        // Accept connections
        self.accept_connections()
    }

    /// Accept and handle incoming connections
    fn accept_connections(&self) -> Result<()> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| RuneError::Daemon("Listener not initialized".to_string()))?;

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let api_handler = self.api_handler.clone();

                    // Handle connection in current thread for simplicity
                    // In production, this should spawn threads or use async
                    if let Err(e) = Self::handle_connection(&mut stream, &api_handler) {
                        error!("Error handling connection: {}", e);
                    }
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Handle a single connection
    fn handle_connection(
        stream: &mut std::os::unix::net::UnixStream,
        api_handler: &ApiHandler,
    ) -> Result<()> {
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;

        debug!("Received request: {}", request_line.trim());

        // Parse HTTP request line
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            Self::send_error(stream, 400, "Bad Request")?;
            return Ok(());
        }

        let method = parts[0];
        let path = parts[1];

        // Read headers
        let mut content_length = 0;
        loop {
            let mut header_line = String::new();
            reader.read_line(&mut header_line)?;
            if header_line.trim().is_empty() {
                break;
            }
            if header_line.to_lowercase().starts_with("content-length:") {
                if let Some(len) = header_line.split(':').nth(1) {
                    content_length = len.trim().parse().unwrap_or(0);
                }
            }
        }

        // Read body if present
        let body = if content_length > 0 {
            let mut buf = vec![0u8; content_length];
            reader.read_exact(&mut buf)?;
            String::from_utf8_lossy(&buf).to_string()
        } else {
            String::new()
        };

        // Route request to API handler
        let response = api_handler.handle_request(method, path, &body)?;

        // Send response
        Self::send_response(stream, &response)?;

        Ok(())
    }

    /// Send HTTP response
    fn send_response(stream: &mut std::os::unix::net::UnixStream, body: &str) -> Result<()> {
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             \r\n\
             {}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes())?;
        stream.flush()?;
        Ok(())
    }

    /// Send HTTP error response
    fn send_error(
        stream: &mut std::os::unix::net::UnixStream,
        code: u16,
        message: &str,
    ) -> Result<()> {
        let body = serde_json::json!({
            "message": message
        });
        let body_str = body.to_string();
        let response = format!(
            "HTTP/1.1 {} {}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             \r\n\
             {}",
            code,
            message,
            body_str.len(),
            body_str
        );
        stream.write_all(response.as_bytes())?;
        stream.flush()?;
        Ok(())
    }

    /// Stop the daemon
    pub fn stop(&self) -> Result<()> {
        // Remove PID file
        if self.config.pid_file.exists() {
            fs::remove_file(&self.config.pid_file)?;
        }

        // Remove socket file
        if self.config.socket_path.exists() {
            fs::remove_file(&self.config.socket_path)?;
        }

        info!("Rune daemon stopped");
        Ok(())
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &Path {
        &self.config.socket_path
    }

    /// Get the container manager
    pub fn container_manager(&self) -> Arc<ContainerManager> {
        self.container_manager.clone()
    }
}

impl Drop for RuneDaemon {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_daemon_config_default() {
        let config = DaemonConfig::default();
        assert_eq!(config.socket_path, PathBuf::from("/var/run/rune.sock"));
        assert_eq!(config.data_dir, PathBuf::from("/var/lib/rune"));
        assert!(!config.debug);
    }

    #[test]
    fn test_daemon_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            socket_path: temp_dir.path().join("rune.sock"),
            data_dir: temp_dir.path().join("data"),
            debug: false,
            pid_file: temp_dir.path().join("rune.pid"),
        };

        let daemon = RuneDaemon::new(config);
        assert!(daemon.is_ok());
    }
}
