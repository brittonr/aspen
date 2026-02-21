//! Ed25519 key management and trust store.

use std::path::Path;
use std::path::PathBuf;

use ed25519_dalek::SigningKey;
use serde::Deserialize;
use serde::Serialize;

use crate::error::SigningError;

/// Generate a new Ed25519 keypair for plugin signing.
pub fn generate_keypair() -> SigningKey {
    SigningKey::generate(&mut rand_core::OsRng)
}

/// Save a secret key to a file in hex format.
///
/// Sets file permissions to 0o600 on Unix.
pub fn save_secret_key(path: &Path, key: &SigningKey) -> Result<(), SigningError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let hex_key = hex::encode(key.to_bytes());
    std::fs::write(path, hex_key)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

/// Load a secret key from a hex-encoded file.
pub fn load_secret_key(path: &Path) -> Result<SigningKey, SigningError> {
    let hex_key = std::fs::read_to_string(path)?;
    let bytes = hex::decode(hex_key.trim())?;
    if bytes.len() != 32 {
        return Err(SigningError::InvalidPublicKeyLength(bytes.len()));
    }
    let array: [u8; 32] = bytes.try_into().expect("length checked");
    Ok(SigningKey::from_bytes(&array))
}

/// Get the public key hex string from a signing key.
pub fn public_key_hex(key: &SigningKey) -> String {
    hex::encode(key.verifying_key().as_bytes())
}

// ---------------------------------------------------------------------------
// Trusted Keys Store
// ---------------------------------------------------------------------------

/// A trusted plugin author.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustedAuthor {
    /// Ed25519 public key (64 hex chars).
    pub pubkey: String,
    /// Human-readable name.
    pub name: String,
    /// When this key was added (Unix milliseconds).
    pub added_at_ms: u64,
}

/// Allowlist of trusted plugin signing keys.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrustedKeys {
    /// List of trusted authors.
    pub authors: Vec<TrustedAuthor>,
}

impl TrustedKeys {
    /// Default path for the trusted keys file.
    pub fn default_path() -> PathBuf {
        dirs_or_default().join("plugin-keys.json")
    }

    /// Load trusted keys from a JSON file. Returns empty set if file doesn't exist.
    pub fn load(path: &Path) -> Result<Self, SigningError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&data)?)
    }

    /// Save trusted keys to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), SigningError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Add a trusted author. No-op if pubkey already exists.
    pub fn add(&mut self, pubkey: &str, name: &str) {
        if self.is_trusted(pubkey) {
            return;
        }
        let now_ms =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
        self.authors.push(TrustedAuthor {
            pubkey: pubkey.to_string(),
            name: name.to_string(),
            added_at_ms: now_ms,
        });
    }

    /// Remove a trusted author by pubkey. Returns whether it was found.
    pub fn remove(&mut self, pubkey: &str) -> bool {
        let before = self.authors.len();
        self.authors.retain(|a| a.pubkey != pubkey);
        self.authors.len() < before
    }

    /// Check if a pubkey is in the trusted list.
    pub fn is_trusted(&self, pubkey: &str) -> bool {
        self.authors.iter().any(|a| a.pubkey == pubkey)
    }
}

/// Get the config directory, falling back to a temp dir.
fn dirs_or_default() -> PathBuf {
    dirs::config_dir().map(|d| d.join("aspen")).unwrap_or_else(|| PathBuf::from("/tmp/aspen-config"))
}

// Provide dirs fallback since it may not be in workspace
mod dirs {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-key");

        let key = generate_keypair();
        save_secret_key(&path, &key).unwrap();

        let loaded = load_secret_key(&path).unwrap();
        assert_eq!(key.to_bytes(), loaded.to_bytes());
    }

    #[test]
    fn public_key_hex_format() {
        let key = generate_keypair();
        let hex_str = public_key_hex(&key);
        assert_eq!(hex_str.len(), 64);
        assert!(hex::decode(&hex_str).is_ok());
    }

    #[test]
    fn trusted_keys_add_remove() {
        let mut keys = TrustedKeys::default();
        assert!(!keys.is_trusted("aabb"));

        keys.add("aabb", "Test Author");
        assert!(keys.is_trusted("aabb"));

        // Duplicate add is no-op
        keys.add("aabb", "Test Author 2");
        assert_eq!(keys.authors.len(), 1);

        assert!(keys.remove("aabb"));
        assert!(!keys.is_trusted("aabb"));
        assert!(!keys.remove("aabb")); // Already removed
    }

    #[test]
    fn trusted_keys_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keys.json");

        let mut keys = TrustedKeys::default();
        keys.add("deadbeef", "Alice");
        keys.add("cafebabe", "Bob");
        keys.save(&path).unwrap();

        let loaded = TrustedKeys::load(&path).unwrap();
        assert_eq!(loaded.authors.len(), 2);
        assert!(loaded.is_trusted("deadbeef"));
        assert!(loaded.is_trusted("cafebabe"));
    }

    #[test]
    fn load_nonexistent_returns_empty() {
        let keys = TrustedKeys::load(Path::new("/tmp/nonexistent-aspen-keys.json")).unwrap();
        assert!(keys.authors.is_empty());
    }
}
