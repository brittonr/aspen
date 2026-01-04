//! Identity types for Forge.
//!
//! This module defines repository identity, author information, and delegate
//! management for Forge repositories.

use iroh::PublicKey;
use serde::Deserialize;
use serde::Serialize;

use crate::constants::MAX_DELEGATES;
use crate::constants::MAX_REPO_DESCRIPTION_LENGTH_BYTES;
use crate::constants::MAX_REPO_NAME_LENGTH_BYTES;
use crate::constants::MAX_THRESHOLD;
use crate::error::ForgeError;
use crate::error::ForgeResult;

/// A repository identifier.
///
/// The RepoId is the BLAKE3 hash of the repository's initial identity document.
/// This makes it content-addressed and globally unique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepoId(pub [u8; 32]);

impl RepoId {
    /// Create a RepoId from a BLAKE3 hash.
    pub fn from_hash(hash: blake3::Hash) -> Self {
        Self(*hash.as_bytes())
    }

    /// Get the hash as a blake3::Hash.
    pub fn to_hash(&self) -> blake3::Hash {
        blake3::Hash::from_bytes(self.0)
    }

    /// Get the RepoId as a hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse a RepoId from a hex string.
    pub fn from_hex(s: &str) -> ForgeResult<Self> {
        let bytes = hex::decode(s).map_err(|e| ForgeError::InvalidRepoIdentity {
            message: format!("invalid hex: {}", e),
        })?;

        if bytes.len() != 32 {
            return Err(ForgeError::InvalidRepoIdentity {
                message: format!("repo ID must be 32 bytes, got {}", bytes.len()),
            });
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl std::fmt::Display for RepoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Repository identity document.
///
/// This is the foundational document for a repository, defining its name,
/// delegates, and signature threshold. The BLAKE3 hash of this document
/// becomes the repository's permanent ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoIdentity {
    /// Repository name.
    pub name: String,

    /// Optional description.
    pub description: Option<String>,

    /// Default branch name (e.g., "main").
    pub default_branch: String,

    /// Delegates who can update canonical refs.
    pub delegates: Vec<PublicKey>,

    /// Number of delegate signatures required for canonical updates.
    ///
    /// Must be >= 1 and <= delegates.len().
    pub threshold: u32,

    /// Unix timestamp in milliseconds when this identity was created.
    pub created_at_ms: u64,
}

impl RepoIdentity {
    /// Create a new repository identity.
    ///
    /// # Errors
    ///
    /// - `ForgeError::InvalidRepoIdentity` if name is empty or too long
    /// - `ForgeError::TooManyDelegates` if delegates exceed limit
    /// - `ForgeError::InvalidThreshold` if threshold is invalid
    pub fn new(name: impl Into<String>, delegates: Vec<PublicKey>, threshold: u32) -> ForgeResult<Self> {
        let name = name.into();

        if name.is_empty() {
            return Err(ForgeError::InvalidRepoIdentity {
                message: "name cannot be empty".to_string(),
            });
        }

        if name.len() as u32 > MAX_REPO_NAME_LENGTH_BYTES {
            return Err(ForgeError::InvalidRepoIdentity {
                message: format!("name too long: {} > {}", name.len(), MAX_REPO_NAME_LENGTH_BYTES),
            });
        }

        if delegates.len() as u32 > MAX_DELEGATES {
            return Err(ForgeError::TooManyDelegates {
                count: delegates.len() as u32,
                max: MAX_DELEGATES,
            });
        }

        if delegates.is_empty() {
            return Err(ForgeError::InvalidRepoIdentity {
                message: "at least one delegate is required".to_string(),
            });
        }

        if threshold == 0 || threshold > delegates.len() as u32 {
            return Err(ForgeError::InvalidThreshold {
                threshold,
                delegates: delegates.len() as u32,
            });
        }

        if threshold > MAX_THRESHOLD {
            return Err(ForgeError::InvalidThreshold {
                threshold,
                delegates: delegates.len() as u32,
            });
        }

        let created_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_millis() as u64;

        Ok(Self {
            name,
            description: None,
            default_branch: "main".to_string(),
            delegates,
            threshold,
            created_at_ms,
        })
    }

    /// Set the description.
    ///
    /// # Errors
    ///
    /// - `ForgeError::InvalidRepoIdentity` if description is too long
    pub fn with_description(mut self, description: impl Into<String>) -> ForgeResult<Self> {
        let desc = description.into();

        if desc.len() as u32 > MAX_REPO_DESCRIPTION_LENGTH_BYTES {
            return Err(ForgeError::InvalidRepoIdentity {
                message: format!("description too long: {} > {}", desc.len(), MAX_REPO_DESCRIPTION_LENGTH_BYTES),
            });
        }

        self.description = Some(desc);
        Ok(self)
    }

    /// Set the default branch.
    pub fn with_default_branch(mut self, branch: impl Into<String>) -> Self {
        self.default_branch = branch.into();
        self
    }

    /// Compute the repository ID (BLAKE3 hash of the identity).
    pub fn repo_id(&self) -> RepoId {
        let bytes = postcard::to_allocvec(self).expect("serialization should not fail");
        RepoId::from_hash(blake3::hash(&bytes))
    }

    /// Check if a public key is a delegate.
    pub fn is_delegate(&self, key: &PublicKey) -> bool {
        self.delegates.contains(key)
    }
}

/// Author information for commits and changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Author {
    /// Display name.
    pub name: String,

    /// Email address (may be empty for anonymous authors).
    pub email: String,

    /// Optional public key linking this author to a Forge identity.
    pub public_key: Option<PublicKey>,

    /// Unix timestamp in milliseconds when this authorship occurred.
    pub timestamp_ms: u64,

    /// Timezone offset string (e.g., "+0000", "-0500").
    /// Preserved from git author lines for SHA-1 reproducibility.
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

/// Default timezone for backwards compatibility with existing serialized data.
fn default_timezone() -> String {
    "+0000".to_string()
}

impl Author {
    /// Create an author from a public key.
    ///
    /// Uses the key's short form as the name and a placeholder email.
    pub fn from_public_key(key: PublicKey) -> Self {
        let short = format!("{}", key).chars().take(16).collect::<String>();
        Self {
            name: short,
            email: format!("{}@forge.local", &format!("{}", key).chars().take(8).collect::<String>()),
            public_key: Some(key),
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time before unix epoch")
                .as_millis() as u64,
            timezone: "+0000".to_string(),
        }
    }

    /// Create an author with explicit name and email.
    pub fn new(name: impl Into<String>, email: impl Into<String>, key: Option<PublicKey>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
            public_key: key,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time before unix epoch")
                .as_millis() as u64,
            timezone: "+0000".to_string(),
        }
    }

    /// Create an author with explicit name, email, and timezone.
    pub fn with_timezone(
        name: impl Into<String>,
        email: impl Into<String>,
        timestamp_ms: u64,
        timezone: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
            public_key: None,
            timestamp_ms,
            timezone: timezone.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> PublicKey {
        let secret = iroh::SecretKey::generate(&mut rand::rng());
        secret.public()
    }

    #[test]
    fn test_repo_id_roundtrip() {
        let hash = blake3::hash(b"test repo");
        let id = RepoId::from_hash(hash);

        let hex_str = id.to_hex();
        let recovered = RepoId::from_hex(&hex_str).expect("should parse");

        assert_eq!(id, recovered);
        assert_eq!(id.to_hash(), hash);
    }

    #[test]
    fn test_repo_identity_creation() {
        let key = test_key();
        let identity = RepoIdentity::new("my-project", vec![key], 1).expect("should create");

        assert_eq!(identity.name, "my-project");
        assert_eq!(identity.threshold, 1);
        assert_eq!(identity.delegates.len(), 1);
        assert!(identity.is_delegate(&key));

        // Repo ID should be deterministic
        let id1 = identity.repo_id();
        let id2 = identity.repo_id();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_repo_identity_validation() {
        let key = test_key();

        // Empty name
        assert!(RepoIdentity::new("", vec![key], 1).is_err());

        // No delegates
        assert!(RepoIdentity::new("test", vec![], 1).is_err());

        // Threshold 0
        assert!(RepoIdentity::new("test", vec![key], 0).is_err());

        // Threshold > delegates
        assert!(RepoIdentity::new("test", vec![key], 2).is_err());
    }

    #[test]
    fn test_author_from_public_key() {
        let key = test_key();
        let author = Author::from_public_key(key);

        assert!(!author.name.is_empty());
        assert!(!author.email.is_empty());
        assert_eq!(author.public_key, Some(key));
    }

    #[test]
    fn test_author_timezone_serialization() {
        // Test that timezone field is preserved through postcard serialization
        let author = Author::with_timezone("Test User", "test@example.com", 1234567890000, "-0500");

        assert_eq!(author.timezone, "-0500");

        // Serialize with postcard
        let bytes = postcard::to_allocvec(&author).expect("should serialize");

        // Deserialize
        let recovered: Author = postcard::from_bytes(&bytes).expect("should deserialize");

        // Timezone should be preserved
        assert_eq!(recovered.timezone, "-0500", "timezone should be preserved through serialization");
        assert_eq!(recovered.name, "Test User");
        assert_eq!(recovered.email, "test@example.com");
        assert_eq!(recovered.timestamp_ms, 1234567890000);
    }
}
