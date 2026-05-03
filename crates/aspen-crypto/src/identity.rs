//! Node identity key lifecycle management.
//!
//! Centralizes the generation, loading, and derivation of a node's
//! `iroh_base::SecretKey` — the root identity for all Aspen operations.
//!
//! # Usage
//!
//! ```rust,ignore
//! use aspen_crypto::identity::NodeIdentityProvider;
//!
//! // Load from hex string (config file)
//! let provider = NodeIdentityProvider::from_hex("abcdef...")?;
//!
//! // Generate a new identity
//! let provider = NodeIdentityProvider::generate();
//!
//! // Load from file or generate if missing
//! let provider = NodeIdentityProvider::load_or_generate(Path::new("/etc/aspen/identity")).await?;
//!
//! // Use the key
//! let public = provider.public_key();
//! let secret = provider.secret_key();
//! ```

use std::path::Path;
use std::path::PathBuf;

use iroh_base::PublicKey;
use iroh_base::SecretKey;
#[cfg(unix)]
use tokio::fs::OpenOptions;
#[cfg(unix)]
use tokio::io::AsyncWriteExt;

/// Required length for a hex-encoded Ed25519 secret key (32 bytes = 64 hex chars).
pub const SECRET_KEY_HEX_LENGTH: usize = 64;

/// Errors from identity key operations.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// Failed to parse a hex-encoded signing key.
    #[error("invalid signing key: {reason}")]
    ParseSigningKey { reason: String },

    /// Failed to load or save an identity key file.
    #[error("identity key at {path}: {reason}")]
    KeyFile { path: PathBuf, reason: String },
}

/// Manages the node's root identity key.
///
/// The node identity key is an Ed25519 keypair (via `iroh_base::SecretKey`) used for:
/// - QUIC endpoint identity (peer authentication)
/// - Gossip protocol participation
/// - DHT announcements (when global-discovery is enabled)
/// - Token signing (when used as a signing key)
///
/// # Tiger Style
///
/// The secret key is held in memory. Use `zeroize` on drop if required
/// by your threat model (iroh::SecretKey already implements Drop cleanup).
pub struct NodeIdentityProvider {
    secret_key: SecretKey,
}

impl NodeIdentityProvider {
    /// Create a provider from an existing secret key.
    pub fn new(secret_key: SecretKey) -> Self {
        Self { secret_key }
    }

    /// Parse a node identity from a hex-encoded secret key string.
    ///
    /// # Errors
    ///
    /// Returns `IdentityError::ParseSigningKey` if the hex string is invalid
    /// or not exactly 64 hex characters (32 bytes).
    pub fn from_hex(hex: &str) -> Result<Self, IdentityError> {
        let hex = hex.trim();

        if hex.len() != SECRET_KEY_HEX_LENGTH {
            return Err(IdentityError::ParseSigningKey {
                reason: format!(
                    "secret key must be {} hex characters (32 bytes), got {}",
                    SECRET_KEY_HEX_LENGTH,
                    hex.len()
                ),
            });
        }

        let bytes = ::hex::decode(hex).map_err(|e| IdentityError::ParseSigningKey {
            reason: format!("invalid hex: {e}"),
        })?;

        let key = SecretKey::try_from(bytes.as_slice()).map_err(|e| IdentityError::ParseSigningKey {
            reason: format!("invalid Ed25519 secret key: {e}"),
        })?;

        Ok(Self { secret_key: key })
    }

    /// Generate a new random node identity.
    pub fn generate() -> Self {
        Self {
            secret_key: SecretKey::generate(&mut rand::rng()),
        }
    }

    /// Load a node identity from a file, or generate and save a new one if the file doesn't exist.
    ///
    /// The file format is a single line containing the hex-encoded secret key.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but contains an invalid key,
    /// or if the file cannot be read/written.
    pub async fn load_or_generate(path: &Path) -> Result<Self, IdentityError> {
        if path.exists() {
            let contents = tokio::fs::read_to_string(path).await.map_err(|e| IdentityError::KeyFile {
                path: path.to_path_buf(),
                reason: format!("failed to read identity file: {e}"),
            })?;

            return Self::from_hex(&contents);
        }

        // Generate new identity
        let provider = Self::generate();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| IdentityError::KeyFile {
                path: path.to_path_buf(),
                reason: format!("failed to create identity directory: {e}"),
            })?;
        }

        // Write hex-encoded key
        let hex_key = hex::encode(provider.secret_key.to_bytes());
        write_secret_key_file(path, hex_key.as_bytes()).await?;

        Ok(provider)
    }

    /// Get the node's public key (used as node ID).
    #[inline]
    pub fn public_key(&self) -> PublicKey {
        self.secret_key.public()
    }

    /// Get a reference to the node's secret key.
    #[inline]
    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    /// Consume the provider and return the secret key.
    pub fn into_secret_key(self) -> SecretKey {
        self.secret_key
    }

    /// Get the secret key as a hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.secret_key.to_bytes())
    }
}

#[cfg(unix)]
async fn write_secret_key_file(path: &Path, contents: &[u8]) -> Result<(), IdentityError> {
    let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(path).await.map_err(|e| {
        IdentityError::KeyFile {
            path: path.to_path_buf(),
            reason: format!("failed to create identity file: {e}"),
        }
    })?;

    file.write_all(contents).await.map_err(|e| IdentityError::KeyFile {
        path: path.to_path_buf(),
        reason: format!("failed to write identity file: {e}"),
    })?;

    file.flush().await.map_err(|e| IdentityError::KeyFile {
        path: path.to_path_buf(),
        reason: format!("failed to flush identity file: {e}"),
    })
}

#[cfg(not(unix))]
async fn write_secret_key_file(path: &Path, contents: &[u8]) -> Result<(), IdentityError> {
    tokio::fs::write(path, contents).await.map_err(|e| IdentityError::KeyFile {
        path: path.to_path_buf(),
        reason: format!("failed to write identity file: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_produces_valid_key() {
        let provider = NodeIdentityProvider::generate();
        let pk = provider.public_key();
        assert_eq!(pk.as_bytes().len(), 32);
    }

    #[test]
    fn test_from_hex_roundtrip() {
        let provider = NodeIdentityProvider::generate();
        let hex = provider.to_hex();
        let loaded = NodeIdentityProvider::from_hex(&hex).unwrap();
        assert_eq!(provider.public_key(), loaded.public_key());
    }

    #[test]
    fn test_from_hex_wrong_length() {
        let result = NodeIdentityProvider::from_hex("abcd");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_hex_invalid_hex() {
        let result = NodeIdentityProvider::from_hex(&"zz".repeat(32));
        assert!(result.is_err());
    }

    #[test]
    fn test_from_hex_trims_whitespace() {
        let provider = NodeIdentityProvider::generate();
        let hex_with_spaces = format!("  {}  \n", provider.to_hex());
        let loaded = NodeIdentityProvider::from_hex(&hex_with_spaces).unwrap();
        assert_eq!(provider.public_key(), loaded.public_key());
    }

    #[test]
    fn test_from_hex_rejects_empty() {
        let result = NodeIdentityProvider::from_hex("");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_hex_rejects_63_chars() {
        // 63 hex chars (one short of the required 64)
        let result = NodeIdentityProvider::from_hex(&"a".repeat(63));
        assert!(result.is_err());
    }

    #[test]
    fn test_from_hex_rejects_65_chars() {
        // 65 hex chars (one over the required 64)
        let result = NodeIdentityProvider::from_hex(&"a".repeat(65));
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_produces_unique_keys() {
        let p1 = NodeIdentityProvider::generate();
        let p2 = NodeIdentityProvider::generate();
        assert_ne!(p1.public_key(), p2.public_key());
    }

    #[tokio::test]
    async fn test_load_or_generate_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity.key");

        assert!(!path.exists());
        let provider = NodeIdentityProvider::load_or_generate(&path).await.unwrap();
        assert!(path.exists());

        // Loading again should give the same key
        let loaded = NodeIdentityProvider::load_or_generate(&path).await.unwrap();
        assert_eq!(provider.public_key(), loaded.public_key());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_load_or_generate_creates_file_with_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity.key");

        NodeIdentityProvider::load_or_generate(&path).await.unwrap();

        let metadata = tokio::fs::metadata(&path).await.unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[tokio::test]
    async fn test_load_or_generate_reads_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity.key");

        let original = NodeIdentityProvider::generate();
        tokio::fs::write(&path, original.to_hex().as_bytes()).await.unwrap();

        let loaded = NodeIdentityProvider::load_or_generate(&path).await.unwrap();
        assert_eq!(original.public_key(), loaded.public_key());
    }
}
