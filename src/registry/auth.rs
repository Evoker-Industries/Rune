//! Registry Authentication
//!
//! Implements authentication for the OCI registry.

use crate::error::{Result, RuneError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable authentication
    pub enabled: bool,
    /// Realm for WWW-Authenticate header
    pub realm: String,
    /// Service name
    pub service: String,
    /// Token issuer
    pub issuer: String,
    /// Token expiry in seconds
    pub token_expiry: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            realm: "Rune Registry".to_string(),
            service: "rune-registry".to_string(),
            issuer: "rune".to_string(),
            token_expiry: 3600,
        }
    }
}

/// User credentials
#[derive(Debug, Clone)]
pub struct User {
    /// Username
    pub username: String,
    /// Password hash
    pub password_hash: String,
    /// Permissions
    pub permissions: Vec<Permission>,
}

/// Permission for a repository
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permission {
    /// Repository pattern (supports wildcards)
    pub repository: String,
    /// Allowed actions
    pub actions: Vec<Action>,
}

/// Repository action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Pull,
    Push,
    Delete,
}

/// Token claim for JWT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaim {
    /// Issuer
    pub iss: String,
    /// Subject (username)
    pub sub: String,
    /// Audience
    pub aud: String,
    /// Expiration time
    pub exp: u64,
    /// Not before
    pub nbf: u64,
    /// Issued at
    pub iat: u64,
    /// JWT ID
    pub jti: String,
    /// Access permissions
    pub access: Vec<AccessClaim>,
}

/// Access claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessClaim {
    /// Resource type
    #[serde(rename = "type")]
    pub resource_type: String,
    /// Resource name
    pub name: String,
    /// Actions
    pub actions: Vec<String>,
}

/// Token response
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    /// Token
    pub token: String,
    /// Access token (alias for token)
    pub access_token: String,
    /// Expires in seconds
    pub expires_in: u64,
    /// Issued at (Unix timestamp)
    pub issued_at: String,
}

/// Registry authentication handler
pub struct RegistryAuth {
    /// Configuration
    config: AuthConfig,
    /// Users database
    users: RwLock<HashMap<String, User>>,
}

impl RegistryAuth {
    /// Create a new authentication handler
    pub fn new() -> Self {
        Self {
            config: AuthConfig::default(),
            users: RwLock::new(HashMap::new()),
        }
    }

    /// Create with configuration
    pub fn with_config(config: AuthConfig) -> Self {
        Self {
            config,
            users: RwLock::new(HashMap::new()),
        }
    }

    /// Add a user
    pub fn add_user(
        &self,
        username: &str,
        password: &str,
        permissions: Vec<Permission>,
    ) -> Result<()> {
        let mut users = self
            .users
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        let password_hash = hash_password(password);

        users.insert(
            username.to_string(),
            User {
                username: username.to_string(),
                password_hash,
                permissions,
            },
        );

        Ok(())
    }

    /// Remove a user
    pub fn remove_user(&self, username: &str) -> Result<()> {
        let mut users = self
            .users
            .write()
            .map_err(|_| RuneError::Lock("Failed to acquire write lock".to_string()))?;

        users.remove(username);
        Ok(())
    }

    /// Verify credentials
    pub fn verify_credentials(&self, username: &str, password: &str) -> Result<bool> {
        let users = self
            .users
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        if let Some(user) = users.get(username) {
            Ok(verify_password(password, &user.password_hash))
        } else {
            Ok(false)
        }
    }

    /// Check if action is allowed
    pub fn is_allowed(&self, username: &str, repository: &str, action: Action) -> Result<bool> {
        let users = self
            .users
            .read()
            .map_err(|_| RuneError::Lock("Failed to acquire read lock".to_string()))?;

        if let Some(user) = users.get(username) {
            for perm in &user.permissions {
                if matches_repository(&perm.repository, repository)
                    && perm.actions.contains(&action)
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Generate a token for authenticated user
    pub fn generate_token(&self, username: &str, scope: &str) -> Result<TokenResponse> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let access = parse_scope(scope);

        let claim = TokenClaim {
            iss: self.config.issuer.clone(),
            sub: username.to_string(),
            aud: self.config.service.clone(),
            exp: now + self.config.token_expiry,
            nbf: now,
            iat: now,
            jti: uuid::Uuid::new_v4().to_string(),
            access,
        };

        // In production, this would be a proper JWT
        // For now, we'll use a simple base64-encoded JSON
        let token = base64_encode(&serde_json::to_string(&claim)?);

        Ok(TokenResponse {
            token: token.clone(),
            access_token: token,
            expires_in: self.config.token_expiry,
            issued_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Verify a token
    pub fn verify_token(&self, token: &str) -> Result<TokenClaim> {
        let decoded = base64_decode(token)?;
        let claim: TokenClaim = serde_json::from_str(&decoded)?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if claim.exp < now {
            return Err(RuneError::PermissionDenied("Token expired".to_string()));
        }

        if claim.nbf > now {
            return Err(RuneError::PermissionDenied(
                "Token not yet valid".to_string(),
            ));
        }

        Ok(claim)
    }

    /// Get WWW-Authenticate header value
    pub fn www_authenticate(&self, scope: Option<&str>) -> String {
        let mut header = format!(
            r#"Bearer realm="{}",service="{}""#,
            self.config.realm, self.config.service
        );

        if let Some(s) = scope {
            header.push_str(&format!(r#",scope="{}""#, s));
        }

        header
    }

    /// Get configuration
    pub fn config(&self) -> &AuthConfig {
        &self.config
    }
}

impl Default for RegistryAuth {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse scope string into access claims
fn parse_scope(scope: &str) -> Vec<AccessClaim> {
    let mut claims = Vec::new();

    for part in scope.split(' ') {
        let parts: Vec<&str> = part.split(':').collect();
        if parts.len() >= 3 {
            claims.push(AccessClaim {
                resource_type: parts[0].to_string(),
                name: parts[1].to_string(),
                actions: parts[2].split(',').map(|s| s.to_string()).collect(),
            });
        }
    }

    claims
}

/// Check if repository matches pattern
fn matches_repository(pattern: &str, repository: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2];
        return repository.starts_with(prefix);
    }

    pattern == repository
}

/// Hash password using bcrypt for secure storage
fn hash_password(password: &str) -> String {
    // Use bcrypt with default cost factor (12)
    bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap_or_else(|_| {
        // Fallback to SHA-256 if bcrypt fails (should not happen)
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        format!("{:x}", hasher.finalize())
    })
}

/// Verify password against bcrypt hash
fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

/// Base64 encode
fn base64_encode(data: &str) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.encode(data.as_bytes())
}

/// Base64 decode
fn base64_decode(data: &str) -> Result<String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let bytes = STANDARD
        .decode(data)
        .map_err(|e| RuneError::InvalidConfig(format!("Invalid base64: {}", e)))?;
    String::from_utf8(bytes).map_err(|e| RuneError::InvalidConfig(format!("Invalid UTF-8: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_verify_user() {
        let auth = RegistryAuth::new();

        auth.add_user(
            "testuser",
            "testpass",
            vec![Permission {
                repository: "*".to_string(),
                actions: vec![Action::Pull, Action::Push],
            }],
        )
        .unwrap();

        assert!(auth.verify_credentials("testuser", "testpass").unwrap());
        assert!(!auth.verify_credentials("testuser", "wrongpass").unwrap());
    }

    #[test]
    fn test_permission_check() {
        let auth = RegistryAuth::new();

        auth.add_user(
            "testuser",
            "testpass",
            vec![Permission {
                repository: "library/*".to_string(),
                actions: vec![Action::Pull],
            }],
        )
        .unwrap();

        assert!(auth
            .is_allowed("testuser", "library/nginx", Action::Pull)
            .unwrap());
        assert!(!auth
            .is_allowed("testuser", "library/nginx", Action::Push)
            .unwrap());
        assert!(!auth
            .is_allowed("testuser", "private/repo", Action::Pull)
            .unwrap());
    }

    #[test]
    fn test_token_generation() {
        let auth = RegistryAuth::new();

        auth.add_user("testuser", "testpass", vec![]).unwrap();

        let token = auth
            .generate_token("testuser", "repository:library/nginx:pull")
            .unwrap();
        assert!(!token.token.is_empty());

        let claim = auth.verify_token(&token.token).unwrap();
        assert_eq!(claim.sub, "testuser");
    }

    #[test]
    fn test_matches_repository() {
        assert!(matches_repository("*", "anything"));
        assert!(matches_repository("library/*", "library/nginx"));
        assert!(!matches_repository("library/*", "private/repo"));
        assert!(matches_repository("library/nginx", "library/nginx"));
        assert!(!matches_repository("library/nginx", "library/alpine"));
    }
}
