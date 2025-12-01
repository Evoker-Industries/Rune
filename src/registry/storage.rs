//! Registry Storage Backend
//!
//! Implements storage for the OCI registry using the filesystem.

use crate::error::{Result, RuneError};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Registry storage backend
pub struct RegistryStorage {
    /// Root storage path
    root: PathBuf,
}

impl RegistryStorage {
    /// Create a new registry storage
    pub fn new(root: PathBuf) -> Result<Self> {
        // Create directory structure
        std::fs::create_dir_all(&root)?;
        std::fs::create_dir_all(root.join("blobs").join("sha256"))?;
        std::fs::create_dir_all(root.join("repositories"))?;
        std::fs::create_dir_all(root.join("uploads"))?;

        Ok(Self { root })
    }

    /// Get blob path
    fn blob_path(&self, digest: &str) -> PathBuf {
        let hash = digest.strip_prefix("sha256:").unwrap_or(digest);
        self.root.join("blobs").join("sha256").join(hash)
    }

    /// Get repository path
    fn repo_path(&self, name: &str) -> PathBuf {
        self.root.join("repositories").join(name)
    }

    /// Get manifest path
    fn manifest_path(&self, name: &str, reference: &str) -> PathBuf {
        let repo = self.repo_path(name);

        // If reference is a digest, store in _manifests/revisions
        if reference.starts_with("sha256:") {
            let hash = reference.strip_prefix("sha256:").unwrap_or(reference);
            repo.join("_manifests")
                .join("revisions")
                .join("sha256")
                .join(hash)
        } else {
            // Tag reference
            repo.join("_manifests")
                .join("tags")
                .join(reference)
                .join("current")
        }
    }

    /// Get upload path
    fn upload_path(&self, uuid: &str) -> PathBuf {
        self.root.join("uploads").join(uuid)
    }

    /// List all repositories
    pub async fn list_repositories(&self) -> Result<Vec<String>> {
        let repos_dir = self.root.join("repositories");
        let mut repos = Vec::new();

        if !repos_dir.exists() {
            return Ok(repos);
        }

        let mut entries = fs::read_dir(&repos_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    // Check for nested repositories (e.g., library/nginx)
                    let nested = self.list_nested_repos(&entry.path(), name).await?;
                    if nested.is_empty() {
                        repos.push(name.to_string());
                    } else {
                        repos.extend(nested);
                    }
                }
            }
        }

        Ok(repos)
    }

    /// List nested repositories
    fn list_nested_repos<'a>(
        &'a self,
        path: &'a PathBuf,
        prefix: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>>> + Send + 'a>> {
        Box::pin(async move {
            let mut repos = Vec::new();

            // Check if this is a repo (has _manifests dir)
            if path.join("_manifests").exists() {
                repos.push(prefix.to_string());
            }

            let mut entries = fs::read_dir(path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let name = entry.file_name();
                let name_str = name.to_str().unwrap_or("");

                // Skip internal directories
                if name_str.starts_with('_') {
                    continue;
                }

                if entry.file_type().await?.is_dir() {
                    let nested_prefix = format!("{}/{}", prefix, name_str);
                    let nested = self
                        .list_nested_repos(&entry.path(), &nested_prefix)
                        .await?;
                    repos.extend(nested);
                }
            }

            Ok(repos)
        })
    }

    /// List tags for a repository
    pub async fn list_tags(&self, name: &str) -> Result<Vec<String>> {
        let tags_dir = self.repo_path(name).join("_manifests").join("tags");
        let mut tags = Vec::new();

        if !tags_dir.exists() {
            return Err(RuneError::ImageNotFound(name.to_string()));
        }

        let mut entries = fs::read_dir(&tags_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                if let Some(tag) = entry.file_name().to_str() {
                    tags.push(tag.to_string());
                }
            }
        }

        Ok(tags)
    }

    /// Get manifest info (content type and size)
    pub fn get_manifest_info<'a>(
        &'a self,
        name: &'a str,
        reference: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(String, u64)>> + Send + 'a>>
    {
        Box::pin(async move {
            let path = self.manifest_path(name, reference);

            if !path.exists() {
                // Try to resolve tag to digest
                let link_path = path.join("link");
                if link_path.exists() {
                    let digest = fs::read_to_string(&link_path).await?;
                    return self.get_manifest_info(name, digest.trim()).await;
                }
                return Err(RuneError::ImageNotFound(format!("{}:{}", name, reference)));
            }

            let content_type_path = path.with_extension("content-type");
            let content_type = if content_type_path.exists() {
                fs::read_to_string(&content_type_path).await?
            } else {
                "application/vnd.oci.image.manifest.v1+json".to_string()
            };

            let metadata = fs::metadata(&path.join("data"))
                .await
                .or_else(|_| std::fs::metadata(&path))?;

            Ok((content_type, metadata.len()))
        })
    }

    /// Get manifest content
    pub async fn get_manifest(&self, name: &str, reference: &str) -> Result<(String, Vec<u8>)> {
        let path = self.manifest_path(name, reference);

        // Check for tag link
        let link_path = if reference.starts_with("sha256:") {
            path.join("data")
        } else {
            let link = path.join("link");
            if link.exists() {
                let digest = fs::read_to_string(&link).await?;
                let hash = digest
                    .trim()
                    .strip_prefix("sha256:")
                    .unwrap_or(digest.trim());
                self.repo_path(name)
                    .join("_manifests")
                    .join("revisions")
                    .join("sha256")
                    .join(hash)
                    .join("data")
            } else {
                return Err(RuneError::ImageNotFound(format!("{}:{}", name, reference)));
            }
        };

        if !link_path.exists() {
            return Err(RuneError::ImageNotFound(format!("{}:{}", name, reference)));
        }

        let content = fs::read(&link_path).await?;

        let content_type_path = link_path.with_file_name("content-type");
        let content_type = if content_type_path.exists() {
            fs::read_to_string(&content_type_path).await?
        } else {
            "application/vnd.oci.image.manifest.v1+json".to_string()
        };

        Ok((content_type, content))
    }

    /// Store manifest
    pub async fn put_manifest(
        &self,
        name: &str,
        reference: &str,
        content_type: &str,
        body: &[u8],
    ) -> Result<String> {
        // Calculate digest
        let mut hasher = Sha256::new();
        hasher.update(body);
        let hash = hasher.finalize();
        let digest = format!("sha256:{:x}", hash);

        // Create repository structure
        let repo = self.repo_path(name);
        fs::create_dir_all(repo.join("_manifests").join("revisions").join("sha256")).await?;
        fs::create_dir_all(repo.join("_manifests").join("tags")).await?;

        // Store by digest
        let hash_str = format!("{:x}", hash);
        let revision_path = repo
            .join("_manifests")
            .join("revisions")
            .join("sha256")
            .join(&hash_str);
        fs::create_dir_all(&revision_path).await?;
        fs::write(revision_path.join("data"), body).await?;
        fs::write(revision_path.join("content-type"), content_type).await?;

        // If reference is a tag, create link
        if !reference.starts_with("sha256:") {
            let tag_path = repo
                .join("_manifests")
                .join("tags")
                .join(reference)
                .join("current");
            fs::create_dir_all(&tag_path).await?;
            fs::write(tag_path.join("link"), &digest).await?;

            // Also store in index
            let index_path = repo
                .join("_manifests")
                .join("tags")
                .join(reference)
                .join("index")
                .join("sha256")
                .join(&hash_str);
            fs::create_dir_all(&index_path).await?;
            fs::write(index_path.join("link"), &digest).await?;
        }

        Ok(digest)
    }

    /// Delete manifest
    pub async fn delete_manifest(&self, name: &str, reference: &str) -> Result<()> {
        let path = self.manifest_path(name, reference);

        if path.exists() {
            fs::remove_dir_all(&path).await?;
        }

        // If it's a tag, we don't delete the revision
        // If it's a digest, delete the revision
        if reference.starts_with("sha256:") {
            let hash = reference.strip_prefix("sha256:").unwrap_or(reference);
            let revision_path = self
                .repo_path(name)
                .join("_manifests")
                .join("revisions")
                .join("sha256")
                .join(hash);
            if revision_path.exists() {
                fs::remove_dir_all(&revision_path).await?;
            }
        }

        Ok(())
    }

    /// Check if blob exists
    pub async fn blob_exists(&self, _name: &str, digest: &str) -> Result<()> {
        let path = self.blob_path(digest);
        if path.exists() {
            Ok(())
        } else {
            Err(RuneError::ImageNotFound(digest.to_string()))
        }
    }

    /// Get blob size
    pub async fn get_blob_size(&self, _name: &str, digest: &str) -> Result<u64> {
        let path = self.blob_path(digest);
        let metadata = fs::metadata(&path)
            .await
            .map_err(|_| RuneError::ImageNotFound(digest.to_string()))?;
        Ok(metadata.len())
    }

    /// Get blob content
    pub async fn get_blob(&self, _name: &str, digest: &str) -> Result<Vec<u8>> {
        let path = self.blob_path(digest);
        fs::read(&path)
            .await
            .map_err(|_| RuneError::ImageNotFound(digest.to_string()))
    }

    /// Delete blob
    pub async fn delete_blob(&self, _name: &str, digest: &str) -> Result<()> {
        let path = self.blob_path(digest);
        fs::remove_file(&path)
            .await
            .map_err(|_| RuneError::ImageNotFound(digest.to_string()))
    }

    /// Mount blob from another repository (cross-repo mount)
    pub async fn mount_blob(&self, _from: &str, _to: &str, digest: &str) -> Result<()> {
        // Blobs are content-addressed and shared, so mounting is a no-op
        // Just verify the blob exists
        let path = self.blob_path(digest);
        if !path.exists() {
            return Err(RuneError::ImageNotFound(digest.to_string()));
        }
        Ok(())
    }

    /// Create upload session
    pub async fn create_upload(&self, uuid: &str) -> Result<()> {
        let path = self.upload_path(uuid);
        fs::create_dir_all(&path).await?;
        fs::write(path.join("data"), &[]).await?;
        Ok(())
    }

    /// Append data to upload
    pub async fn append_upload(&self, uuid: &str, data: &[u8]) -> Result<()> {
        let path = self.upload_path(uuid).join("data");
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .await
            .map_err(|_| RuneError::Internal(format!("Upload {} not found", uuid)))?;
        file.write_all(data).await?;
        Ok(())
    }

    /// Complete upload and move to blobs
    pub async fn complete_upload(
        &self,
        _name: &str,
        uuid: &str,
        expected_digest: &str,
    ) -> Result<String> {
        let upload_path = self.upload_path(uuid).join("data");

        // Read and hash the content
        let content = fs::read(&upload_path)
            .await
            .map_err(|_| RuneError::Internal(format!("Upload {} not found", uuid)))?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        let actual_digest = format!("sha256:{:x}", hash);

        // Verify digest
        if actual_digest != expected_digest {
            return Err(RuneError::InvalidConfig(format!(
                "Digest mismatch: expected {}, got {}",
                expected_digest, actual_digest
            )));
        }

        // Move to blobs
        let blob_path = self.blob_path(&actual_digest);
        fs::rename(&upload_path, &blob_path).await?;

        // Clean up upload directory
        let upload_dir = self.upload_path(uuid);
        fs::remove_dir_all(&upload_dir).await?;

        Ok(actual_digest)
    }

    /// Delete upload
    pub async fn delete_upload(&self, uuid: &str) -> Result<()> {
        let path = self.upload_path(uuid);
        if path.exists() {
            fs::remove_dir_all(&path).await?;
        }
        Ok(())
    }

    /// Garbage collect unreferenced blobs
    pub async fn garbage_collect(&self) -> Result<Vec<String>> {
        // Collect all referenced digests from manifests
        let mut referenced = std::collections::HashSet::new();

        let repos = self.list_repositories().await?;
        for repo in repos {
            let tags = match self.list_tags(&repo).await {
                Ok(t) => t,
                Err(_) => continue,
            };

            for tag in tags {
                if let Ok((_, content)) = self.get_manifest(&repo, &tag).await {
                    // Parse manifest and collect referenced blobs
                    if let Ok(manifest) =
                        serde_json::from_slice::<super::server::ImageManifest>(&content)
                    {
                        referenced.insert(manifest.config.digest.clone());
                        for layer in manifest.layers {
                            referenced.insert(layer.digest.clone());
                        }
                    }
                }
            }
        }

        // Find unreferenced blobs
        let blobs_dir = self.root.join("blobs").join("sha256");
        let mut deleted = Vec::new();

        if blobs_dir.exists() {
            let mut entries = fs::read_dir(&blobs_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                if let Some(hash) = entry.file_name().to_str() {
                    let digest = format!("sha256:{}", hash);
                    if !referenced.contains(&digest) {
                        if fs::remove_file(entry.path()).await.is_ok() {
                            deleted.push(digest);
                        }
                    }
                }
            }
        }

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_storage_creation() {
        let temp = tempdir().unwrap();
        let _storage = RegistryStorage::new(temp.path().to_path_buf()).unwrap();

        assert!(temp.path().join("blobs").join("sha256").exists());
        assert!(temp.path().join("repositories").exists());
        assert!(temp.path().join("uploads").exists());
    }

    #[tokio::test]
    async fn test_put_and_get_manifest() {
        let temp = tempdir().unwrap();
        let storage = RegistryStorage::new(temp.path().to_path_buf()).unwrap();

        let manifest = r#"{"schemaVersion":2,"config":{"mediaType":"test","digest":"sha256:abc","size":0},"layers":[]}"#;

        let digest = storage
            .put_manifest(
                "test/repo",
                "latest",
                "application/vnd.oci.image.manifest.v1+json",
                manifest.as_bytes(),
            )
            .await
            .unwrap();

        assert!(digest.starts_with("sha256:"));

        let (content_type, content) = storage.get_manifest("test/repo", "latest").await.unwrap();
        assert_eq!(content_type, "application/vnd.oci.image.manifest.v1+json");
        assert_eq!(content, manifest.as_bytes());
    }

    #[tokio::test]
    async fn test_upload_flow() {
        let temp = tempdir().unwrap();
        let storage = RegistryStorage::new(temp.path().to_path_buf()).unwrap();

        let uuid = "test-upload-123";
        storage.create_upload(uuid).await.unwrap();

        let data = b"hello world";
        storage.append_upload(uuid, data).await.unwrap();

        // Calculate expected digest
        let mut hasher = Sha256::new();
        hasher.update(data);
        let expected = format!("sha256:{:x}", hasher.finalize());

        let digest = storage
            .complete_upload("test/repo", uuid, &expected)
            .await
            .unwrap();
        assert_eq!(digest, expected);

        // Verify blob exists
        let content = storage.get_blob("test/repo", &digest).await.unwrap();
        assert_eq!(content, data);
    }
}
