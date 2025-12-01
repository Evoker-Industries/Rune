//! OCI Registry Server Implementation
//!
//! Implements the OCI Distribution Specification for a Docker-compatible registry.

use super::auth::RegistryAuth;
use super::storage::RegistryStorage;
use crate::error::{Result, RuneError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// OCI Distribution API version
pub const API_VERSION: &str = "registry/2.0";

/// Supported media types
pub mod media_types {
    pub const MANIFEST_V2: &str = "application/vnd.docker.distribution.manifest.v2+json";
    pub const MANIFEST_LIST_V2: &str = "application/vnd.docker.distribution.manifest.list.v2+json";
    pub const OCI_MANIFEST_V1: &str = "application/vnd.oci.image.manifest.v1+json";
    pub const OCI_INDEX_V1: &str = "application/vnd.oci.image.index.v1+json";
    pub const OCI_CONFIG_V1: &str = "application/vnd.oci.image.config.v1+json";
    pub const OCI_LAYER_TAR_GZIP: &str = "application/vnd.oci.image.layer.v1.tar+gzip";
    pub const OCI_LAYER_TAR: &str = "application/vnd.oci.image.layer.v1.tar";
    pub const DOCKER_LAYER: &str = "application/vnd.docker.image.rootfs.diff.tar.gzip";
    pub const DOCKER_CONFIG: &str = "application/vnd.docker.container.image.v1+json";
}

/// Registry server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Server address
    pub address: String,
    /// Server port
    pub port: u16,
    /// Storage path
    pub storage_path: PathBuf,
    /// Enable TLS
    pub tls_enabled: bool,
    /// TLS certificate path
    pub tls_cert: Option<PathBuf>,
    /// TLS key path
    pub tls_key: Option<PathBuf>,
    /// Enable authentication
    pub auth_enabled: bool,
    /// Realm for authentication
    pub auth_realm: String,
    /// Allow anonymous pull
    pub anonymous_pull: bool,
    /// Allow anonymous push
    pub anonymous_push: bool,
    /// Enable delete operations
    pub delete_enabled: bool,
    /// Maximum manifest size
    pub max_manifest_size: usize,
    /// Maximum layer size
    pub max_layer_size: u64,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            address: "0.0.0.0".to_string(),
            port: 5000,
            storage_path: PathBuf::from("/var/lib/rune/registry"),
            tls_enabled: false,
            tls_cert: None,
            tls_key: None,
            auth_enabled: false,
            auth_realm: "Rune Registry".to_string(),
            anonymous_pull: true,
            anonymous_push: false,
            delete_enabled: true,
            max_manifest_size: 4 * 1024 * 1024,      // 4MB
            max_layer_size: 10 * 1024 * 1024 * 1024, // 10GB
        }
    }
}

/// OCI Image Manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageManifest {
    /// Schema version (always 2)
    pub schema_version: u32,
    /// Media type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    /// Artifact type (OCI 1.1+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_type: Option<String>,
    /// Config descriptor
    pub config: Descriptor,
    /// Layer descriptors
    pub layers: Vec<Descriptor>,
    /// Subject (OCI 1.1+ referrers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Descriptor>,
    /// Annotations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub annotations: HashMap<String, String>,
}

/// OCI Image Index (manifest list)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageIndex {
    /// Schema version (always 2)
    pub schema_version: u32,
    /// Media type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    /// Artifact type (OCI 1.1+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_type: Option<String>,
    /// Manifests
    pub manifests: Vec<Descriptor>,
    /// Subject (OCI 1.1+ referrers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Descriptor>,
    /// Annotations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub annotations: HashMap<String, String>,
}

/// Content Descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    /// Media type
    pub media_type: String,
    /// Content digest
    pub digest: String,
    /// Content size
    pub size: u64,
    /// URLs for external content
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub urls: Vec<String>,
    /// Annotations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub annotations: HashMap<String, String>,
    /// Platform (for manifest lists)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<Platform>,
    /// Artifact type (OCI 1.1+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_type: Option<String>,
    /// Data (for small blobs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// Platform specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    /// Architecture
    pub architecture: String,
    /// Operating system
    pub os: String,
    /// OS version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    /// OS features
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub os_features: Vec<String>,
    /// CPU variant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

/// Catalog response
#[derive(Debug, Serialize, Deserialize)]
pub struct CatalogResponse {
    pub repositories: Vec<String>,
}

/// Tags list response
#[derive(Debug, Serialize, Deserialize)]
pub struct TagsListResponse {
    pub name: String,
    pub tags: Vec<String>,
}

/// Error response (OCI spec)
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub errors: Vec<RegistryError>,
}

/// Registry error
#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

/// Error codes per OCI spec
pub mod error_codes {
    pub const BLOB_UNKNOWN: &str = "BLOB_UNKNOWN";
    pub const BLOB_UPLOAD_INVALID: &str = "BLOB_UPLOAD_INVALID";
    pub const BLOB_UPLOAD_UNKNOWN: &str = "BLOB_UPLOAD_UNKNOWN";
    pub const DIGEST_INVALID: &str = "DIGEST_INVALID";
    pub const MANIFEST_BLOB_UNKNOWN: &str = "MANIFEST_BLOB_UNKNOWN";
    pub const MANIFEST_INVALID: &str = "MANIFEST_INVALID";
    pub const MANIFEST_UNKNOWN: &str = "MANIFEST_UNKNOWN";
    pub const NAME_INVALID: &str = "NAME_INVALID";
    pub const NAME_UNKNOWN: &str = "NAME_UNKNOWN";
    pub const SIZE_INVALID: &str = "SIZE_INVALID";
    pub const UNAUTHORIZED: &str = "UNAUTHORIZED";
    pub const DENIED: &str = "DENIED";
    pub const UNSUPPORTED: &str = "UNSUPPORTED";
    pub const TOOMANYREQUESTS: &str = "TOOMANYREQUESTS";
}

/// Upload session
#[derive(Debug, Clone)]
pub struct UploadSession {
    /// Session UUID
    pub uuid: String,
    /// Repository name
    pub repository: String,
    /// Current offset
    pub offset: u64,
    /// Started at
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Last chunk received
    pub last_chunk_at: chrono::DateTime<chrono::Utc>,
}

/// Registry server
pub struct RegistryServer {
    /// Configuration
    config: RegistryConfig,
    /// Storage backend
    storage: Arc<RegistryStorage>,
    /// Authentication
    auth: Arc<RegistryAuth>,
    /// Active upload sessions
    uploads: Arc<RwLock<HashMap<String, UploadSession>>>,
}

impl RegistryServer {
    /// Create a new registry server
    pub fn new(config: RegistryConfig) -> Result<Self> {
        let storage = Arc::new(RegistryStorage::new(config.storage_path.clone())?);
        let auth = Arc::new(RegistryAuth::new());

        Ok(Self {
            config,
            storage,
            auth,
            uploads: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get API version header value
    pub fn api_version() -> &'static str {
        API_VERSION
    }

    /// Check API version (GET /v2/)
    pub async fn check_api(&self) -> Result<()> {
        Ok(())
    }

    /// List repositories (GET /v2/_catalog)
    pub async fn list_repositories(
        &self,
        n: Option<usize>,
        last: Option<String>,
    ) -> Result<CatalogResponse> {
        let repos = self.storage.list_repositories().await?;

        let mut filtered: Vec<String> = repos
            .into_iter()
            .filter(|r| {
                if let Some(ref last) = last {
                    r > last
                } else {
                    true
                }
            })
            .collect();

        filtered.sort();

        if let Some(n) = n {
            filtered.truncate(n);
        }

        Ok(CatalogResponse {
            repositories: filtered,
        })
    }

    /// List tags (GET /v2/{name}/tags/list)
    pub async fn list_tags(
        &self,
        name: &str,
        n: Option<usize>,
        last: Option<String>,
    ) -> Result<TagsListResponse> {
        let tags = self.storage.list_tags(name).await?;

        let mut filtered: Vec<String> = tags
            .into_iter()
            .filter(|t| {
                if let Some(ref last) = last {
                    t > last
                } else {
                    true
                }
            })
            .collect();

        filtered.sort();

        if let Some(n) = n {
            filtered.truncate(n);
        }

        Ok(TagsListResponse {
            name: name.to_string(),
            tags: filtered,
        })
    }

    /// Check if manifest exists (HEAD /v2/{name}/manifests/{reference})
    pub async fn manifest_exists(&self, name: &str, reference: &str) -> Result<(String, u64)> {
        self.storage.get_manifest_info(name, reference).await
    }

    /// Get manifest (GET /v2/{name}/manifests/{reference})
    pub async fn get_manifest(&self, name: &str, reference: &str) -> Result<(String, Vec<u8>)> {
        self.storage.get_manifest(name, reference).await
    }

    /// Put manifest (PUT /v2/{name}/manifests/{reference})
    pub async fn put_manifest(
        &self,
        name: &str,
        reference: &str,
        content_type: &str,
        body: Vec<u8>,
    ) -> Result<String> {
        // Validate size
        if body.len() > self.config.max_manifest_size {
            return Err(RuneError::InvalidConfig(format!(
                "Manifest size {} exceeds maximum {}",
                body.len(),
                self.config.max_manifest_size
            )));
        }

        // Validate manifest
        self.validate_manifest(content_type, &body)?;

        // Store manifest
        let digest = self
            .storage
            .put_manifest(name, reference, content_type, &body)
            .await?;

        Ok(digest)
    }

    /// Delete manifest (DELETE /v2/{name}/manifests/{reference})
    pub async fn delete_manifest(&self, name: &str, reference: &str) -> Result<()> {
        if !self.config.delete_enabled {
            return Err(RuneError::PermissionDenied(
                "Delete operations are disabled".to_string(),
            ));
        }

        self.storage.delete_manifest(name, reference).await
    }

    /// Check if blob exists (HEAD /v2/{name}/blobs/{digest})
    pub async fn blob_exists(&self, name: &str, digest: &str) -> Result<u64> {
        self.storage.get_blob_size(name, digest).await
    }

    /// Get blob (GET /v2/{name}/blobs/{digest})
    pub async fn get_blob(&self, name: &str, digest: &str) -> Result<Vec<u8>> {
        self.storage.get_blob(name, digest).await
    }

    /// Delete blob (DELETE /v2/{name}/blobs/{digest})
    pub async fn delete_blob(&self, name: &str, digest: &str) -> Result<()> {
        if !self.config.delete_enabled {
            return Err(RuneError::PermissionDenied(
                "Delete operations are disabled".to_string(),
            ));
        }

        self.storage.delete_blob(name, digest).await
    }

    /// Start blob upload (POST /v2/{name}/blobs/uploads/)
    pub async fn start_upload(
        &self,
        name: &str,
        digest: Option<String>,
        mount_from: Option<String>,
    ) -> Result<(String, Option<String>)> {
        // Check for cross-repository mount
        if let (Some(ref d), Some(ref from)) = (&digest, &mount_from) {
            if self.storage.blob_exists(from, d).await.is_ok() {
                // Mount blob from another repository
                self.storage.mount_blob(from, name, d).await?;
                return Ok((String::new(), Some(d.clone())));
            }
        }

        // Check for single POST upload with digest
        if digest.is_some() {
            // Will be handled by complete_upload
        }

        // Create upload session
        let uuid = uuid::Uuid::new_v4().to_string();
        let session = UploadSession {
            uuid: uuid.clone(),
            repository: name.to_string(),
            offset: 0,
            started_at: chrono::Utc::now(),
            last_chunk_at: chrono::Utc::now(),
        };

        self.storage.create_upload(&uuid).await?;

        let mut uploads = self.uploads.write().await;
        uploads.insert(uuid.clone(), session);

        Ok((uuid, None))
    }

    /// Get upload status (GET /v2/{name}/blobs/uploads/{uuid})
    pub async fn get_upload_status(&self, _name: &str, uuid: &str) -> Result<u64> {
        let uploads = self.uploads.read().await;
        let session = uploads
            .get(uuid)
            .ok_or_else(|| RuneError::Internal(format!("Upload {} not found", uuid)))?;

        Ok(session.offset)
    }

    /// Upload chunk (PATCH /v2/{name}/blobs/uploads/{uuid})
    pub async fn upload_chunk(
        &self,
        _name: &str,
        uuid: &str,
        data: Vec<u8>,
        content_range: Option<(u64, u64)>,
    ) -> Result<u64> {
        let mut uploads = self.uploads.write().await;
        let session = uploads
            .get_mut(uuid)
            .ok_or_else(|| RuneError::Internal(format!("Upload {} not found", uuid)))?;

        // Validate range if provided
        if let Some((start, _end)) = content_range {
            if start != session.offset {
                return Err(RuneError::InvalidConfig(format!(
                    "Invalid content range start: expected {}, got {}",
                    session.offset, start
                )));
            }
        }

        // Append data to upload
        self.storage.append_upload(uuid, &data).await?;

        // Update session
        session.offset += data.len() as u64;
        session.last_chunk_at = chrono::Utc::now();

        Ok(session.offset)
    }

    /// Complete upload (PUT /v2/{name}/blobs/uploads/{uuid})
    pub async fn complete_upload(
        &self,
        name: &str,
        uuid: &str,
        digest: &str,
        data: Option<Vec<u8>>,
    ) -> Result<String> {
        // Append final data if provided
        if let Some(d) = data {
            self.upload_chunk(name, uuid, d, None).await?;
        }

        // Finalize upload
        let actual_digest = self.storage.complete_upload(name, uuid, digest).await?;

        // Verify digest
        if actual_digest != digest {
            return Err(RuneError::InvalidConfig(format!(
                "Digest mismatch: expected {}, got {}",
                digest, actual_digest
            )));
        }

        // Remove upload session
        let mut uploads = self.uploads.write().await;
        uploads.remove(uuid);

        Ok(actual_digest)
    }

    /// Cancel upload (DELETE /v2/{name}/blobs/uploads/{uuid})
    pub async fn cancel_upload(&self, _name: &str, uuid: &str) -> Result<()> {
        self.storage.delete_upload(uuid).await?;

        let mut uploads = self.uploads.write().await;
        uploads.remove(uuid);

        Ok(())
    }

    /// Validate manifest content
    fn validate_manifest(&self, content_type: &str, body: &[u8]) -> Result<()> {
        match content_type {
            media_types::OCI_MANIFEST_V1 | media_types::MANIFEST_V2 => {
                let _manifest: ImageManifest = serde_json::from_slice(body)
                    .map_err(|e| RuneError::InvalidConfig(format!("Invalid manifest: {}", e)))?;
            }
            media_types::OCI_INDEX_V1 | media_types::MANIFEST_LIST_V2 => {
                let _index: ImageIndex = serde_json::from_slice(body)
                    .map_err(|e| RuneError::InvalidConfig(format!("Invalid index: {}", e)))?;
            }
            _ => {
                // Accept unknown types
            }
        }
        Ok(())
    }

    /// Get configuration
    pub fn config(&self) -> &RegistryConfig {
        &self.config
    }

    /// Get storage
    pub fn storage(&self) -> &Arc<RegistryStorage> {
        &self.storage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_registry_server_creation() {
        let temp = tempdir().unwrap();
        let mut config = RegistryConfig::default();
        config.storage_path = temp.path().to_path_buf();

        let server = RegistryServer::new(config).unwrap();
        assert_eq!(server.config.port, 5000);
    }

    #[tokio::test]
    async fn test_check_api() {
        let temp = tempdir().unwrap();
        let mut config = RegistryConfig::default();
        config.storage_path = temp.path().to_path_buf();

        let server = RegistryServer::new(config).unwrap();
        assert!(server.check_api().await.is_ok());
    }

    #[test]
    fn test_manifest_serialization() {
        let manifest = ImageManifest {
            schema_version: 2,
            media_type: Some(media_types::OCI_MANIFEST_V1.to_string()),
            artifact_type: None,
            config: Descriptor {
                media_type: media_types::OCI_CONFIG_V1.to_string(),
                digest: "sha256:abc123".to_string(),
                size: 1024,
                urls: vec![],
                annotations: HashMap::new(),
                platform: None,
                artifact_type: None,
                data: None,
            },
            layers: vec![],
            subject: None,
            annotations: HashMap::new(),
        };

        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("schemaVersion"));
    }
}
