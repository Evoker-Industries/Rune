//! Container registry client and server

use crate::error::{Result, RuneError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OCI Distribution Specification media types
pub mod media_types {
    pub const MANIFEST_V2: &str = "application/vnd.docker.distribution.manifest.v2+json";
    pub const MANIFEST_LIST_V2: &str = "application/vnd.docker.distribution.manifest.list.v2+json";
    pub const OCI_MANIFEST: &str = "application/vnd.oci.image.manifest.v1+json";
    pub const OCI_INDEX: &str = "application/vnd.oci.image.index.v1+json";
    pub const OCI_CONFIG: &str = "application/vnd.oci.image.config.v1+json";
    pub const OCI_LAYER: &str = "application/vnd.oci.image.layer.v1.tar+gzip";
    pub const DOCKER_CONFIG: &str = "application/vnd.docker.container.image.v1+json";
    pub const DOCKER_LAYER: &str = "application/vnd.docker.image.rootfs.diff.tar.gzip";
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Registry URL
    pub url: String,
    /// Username for authentication
    pub username: Option<String>,
    /// Password for authentication
    pub password: Option<String>,
    /// Use TLS
    pub tls: bool,
    /// Skip TLS verification
    pub insecure: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            url: "https://registry-1.docker.io".to_string(),
            username: None,
            password: None,
            tls: true,
            insecure: false,
        }
    }
}

/// Image manifest (OCI/Docker v2)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageManifest {
    /// Schema version
    pub schema_version: u32,
    /// Media type
    pub media_type: String,
    /// Config descriptor
    pub config: Descriptor,
    /// Layer descriptors
    pub layers: Vec<Descriptor>,
    /// Annotations
    #[serde(default)]
    pub annotations: HashMap<String, String>,
}

/// Manifest list (multi-arch)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestList {
    /// Schema version
    pub schema_version: u32,
    /// Media type
    pub media_type: String,
    /// Platform manifests
    pub manifests: Vec<PlatformManifest>,
}

/// Platform-specific manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformManifest {
    /// Media type
    pub media_type: String,
    /// Digest
    pub digest: String,
    /// Size in bytes
    pub size: u64,
    /// Platform
    pub platform: Platform,
}

/// Platform specification
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Variant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

/// Content descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    /// Media type
    pub media_type: String,
    /// Digest
    pub digest: String,
    /// Size in bytes
    pub size: u64,
    /// URLs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub urls: Vec<String>,
    /// Annotations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub annotations: HashMap<String, String>,
}

/// Registry client for pulling and pushing images
pub struct Registry {
    /// Registry configuration
    config: RegistryConfig,
    /// HTTP client
    client: reqwest::Client,
    /// Auth token
    token: Option<String>,
}

impl Registry {
    /// Create a new registry client
    pub fn new(config: RegistryConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(config.insecure)
            .build()
            .map_err(|e| RuneError::Network(e.to_string()))?;

        Ok(Self {
            config,
            client,
            token: None,
        })
    }

    /// Create a client for Docker Hub
    pub fn docker_hub() -> Result<Self> {
        Self::new(RegistryConfig::default())
    }

    /// Authenticate with the registry
    pub async fn authenticate(&mut self) -> Result<()> {
        // For Docker Hub, we need to get a token
        if self.config.url.contains("docker.io") {
            let token_url = "https://auth.docker.io/token";
            let params = [
                ("service", "registry.docker.io"),
                ("scope", "repository:library/alpine:pull"),
            ];

            let response = self.client
                .get(token_url)
                .query(&params)
                .send()
                .await
                .map_err(|e| RuneError::Network(e.to_string()))?;

            if response.status().is_success() {
                let token_response: TokenResponse = response.json()
                    .await
                    .map_err(|e| RuneError::Network(e.to_string()))?;
                self.token = Some(token_response.token);
            }
        }

        Ok(())
    }

    /// Pull an image manifest
    pub async fn pull_manifest(&self, name: &str, reference: &str) -> Result<ImageManifest> {
        let url = format!("{}/v2/{}/manifests/{}", self.config.url, name, reference);

        let mut request = self.client.get(&url)
            .header("Accept", media_types::OCI_MANIFEST)
            .header("Accept", media_types::MANIFEST_V2);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send()
            .await
            .map_err(|e| RuneError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RuneError::Image(format!(
                "Failed to pull manifest: {} {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let manifest: ImageManifest = response.json()
            .await
            .map_err(|e| RuneError::Network(e.to_string()))?;

        Ok(manifest)
    }

    /// Pull a blob (layer or config)
    pub async fn pull_blob(&self, name: &str, digest: &str) -> Result<Vec<u8>> {
        let url = format!("{}/v2/{}/blobs/{}", self.config.url, name, digest);

        let mut request = self.client.get(&url);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send()
            .await
            .map_err(|e| RuneError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RuneError::Image(format!(
                "Failed to pull blob: {}",
                response.status()
            )));
        }

        let bytes = response.bytes()
            .await
            .map_err(|e| RuneError::Network(e.to_string()))?;

        Ok(bytes.to_vec())
    }

    /// Push an image manifest
    pub async fn push_manifest(
        &self,
        name: &str,
        reference: &str,
        manifest: &ImageManifest,
    ) -> Result<String> {
        let url = format!("{}/v2/{}/manifests/{}", self.config.url, name, reference);

        let body = serde_json::to_string(manifest)?;

        let mut request = self.client.put(&url)
            .header("Content-Type", media_types::OCI_MANIFEST)
            .body(body);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send()
            .await
            .map_err(|e| RuneError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RuneError::Image(format!(
                "Failed to push manifest: {}",
                response.status()
            )));
        }

        // Get the digest from the response header
        let digest = response.headers()
            .get("Docker-Content-Digest")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        Ok(digest)
    }

    /// Push a blob
    pub async fn push_blob(&self, name: &str, data: Vec<u8>) -> Result<String> {
        // Start upload
        let url = format!("{}/v2/{}/blobs/uploads/", self.config.url, name);

        let mut request = self.client.post(&url);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send()
            .await
            .map_err(|e| RuneError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RuneError::Image(format!(
                "Failed to start blob upload: {}",
                response.status()
            )));
        }

        // Get upload URL
        let upload_url = response.headers()
            .get("Location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| RuneError::Image("No upload location provided".to_string()))?
            .to_string();

        // Calculate digest
        let digest = format!("sha256:{:x}", sha256_digest(&data));

        // Complete upload
        let url = format!("{}&digest={}", upload_url, digest);

        let mut request = self.client.put(&url)
            .header("Content-Type", "application/octet-stream")
            .body(data);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send()
            .await
            .map_err(|e| RuneError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RuneError::Image(format!(
                "Failed to complete blob upload: {}",
                response.status()
            )));
        }

        Ok(digest)
    }

    /// Check if a blob exists
    pub async fn blob_exists(&self, name: &str, digest: &str) -> Result<bool> {
        let url = format!("{}/v2/{}/blobs/{}", self.config.url, name, digest);

        let mut request = self.client.head(&url);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send()
            .await
            .map_err(|e| RuneError::Network(e.to_string()))?;

        Ok(response.status().is_success())
    }

    /// List tags for a repository
    pub async fn list_tags(&self, name: &str) -> Result<Vec<String>> {
        let url = format!("{}/v2/{}/tags/list", self.config.url, name);

        let mut request = self.client.get(&url);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send()
            .await
            .map_err(|e| RuneError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RuneError::Image(format!(
                "Failed to list tags: {}",
                response.status()
            )));
        }

        let tags_response: TagsResponse = response.json()
            .await
            .map_err(|e| RuneError::Network(e.to_string()))?;

        Ok(tags_response.tags)
    }

    /// Delete a manifest
    pub async fn delete_manifest(&self, name: &str, reference: &str) -> Result<()> {
        let url = format!("{}/v2/{}/manifests/{}", self.config.url, name, reference);

        let mut request = self.client.delete(&url);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send()
            .await
            .map_err(|e| RuneError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RuneError::Image(format!(
                "Failed to delete manifest: {}",
                response.status()
            )));
        }

        Ok(())
    }
}

/// Token response from auth server
#[derive(Debug, Deserialize)]
struct TokenResponse {
    token: String,
    #[serde(default)]
    expires_in: Option<u64>,
}

/// Tags list response
#[derive(Debug, Deserialize)]
struct TagsResponse {
    name: String,
    tags: Vec<String>,
}

/// Simple SHA256 hash (placeholder - in production use proper crypto)
fn sha256_digest(data: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_config_default() {
        let config = RegistryConfig::default();
        assert!(config.url.contains("docker.io"));
        assert!(config.tls);
    }

    #[test]
    fn test_descriptor_serialization() {
        let desc = Descriptor {
            media_type: media_types::OCI_LAYER.to_string(),
            digest: "sha256:abc123".to_string(),
            size: 1024,
            urls: vec![],
            annotations: HashMap::new(),
        };

        let json = serde_json::to_string(&desc).unwrap();
        assert!(json.contains("sha256:abc123"));
    }
}
