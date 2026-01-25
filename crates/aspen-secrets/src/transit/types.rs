//! Transit secrets engine types.
//!
//! Data structures for encryption-as-a-service operations.

use serde::Deserialize;
use serde::Serialize;

/// Key type for Transit keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum KeyType {
    /// AES-256-GCM (symmetric encryption).
    Aes256Gcm,
    /// XChaCha20-Poly1305 (symmetric encryption).
    #[default]
    XChaCha20Poly1305,
    /// Ed25519 (asymmetric signing).
    Ed25519,
}

impl KeyType {
    /// Check if this key type supports encryption.
    pub fn supports_encryption(&self) -> bool {
        matches!(self, KeyType::Aes256Gcm | KeyType::XChaCha20Poly1305)
    }

    /// Check if this key type supports signing.
    pub fn supports_signing(&self) -> bool {
        matches!(self, KeyType::Ed25519)
    }

    /// Get the key size in bytes.
    pub fn key_size(&self) -> usize {
        match self {
            KeyType::Aes256Gcm => 32,
            KeyType::XChaCha20Poly1305 => 32,
            KeyType::Ed25519 => 32, // Secret key seed
        }
    }
}

/// A version of a transit key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyVersion {
    /// Version number (1-indexed).
    pub version: u32,
    /// Unix timestamp when this version was created.
    pub created_time_unix_ms: u64,
    /// The key material (encrypted at rest).
    /// For asymmetric keys, this is the private key.
    pub key_material: Vec<u8>,
    /// Public key for asymmetric keys (None for symmetric).
    pub public_key: Option<Vec<u8>>,
}

/// Transit key metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitKey {
    /// Key name.
    pub name: String,
    /// Key type.
    pub key_type: KeyType,
    /// Current version number.
    pub current_version: u32,
    /// Minimum version that can decrypt.
    pub min_decryption_version: u32,
    /// Minimum version for new encryptions.
    pub min_encryption_version: u32,
    /// Whether the key can be deleted.
    pub deletion_allowed: bool,
    /// Whether the key can be exported.
    pub exportable: bool,
    /// Whether plaintext backup is allowed.
    pub allow_plaintext_backup: bool,
    /// Unix timestamp when the key was created.
    pub created_time_unix_ms: u64,
    /// Unix timestamp when the key was last rotated.
    pub latest_version_time_unix_ms: u64,
    /// Whether to support convergent encryption.
    pub supports_convergent_encryption: bool,
    /// Key versions.
    pub versions: std::collections::HashMap<u32, KeyVersion>,
}

impl TransitKey {
    /// Create a new transit key.
    pub fn new(
        name: String,
        key_type: KeyType,
        created_time_unix_ms: u64,
        initial_key: Vec<u8>,
        public_key: Option<Vec<u8>>,
    ) -> Self {
        let mut versions = std::collections::HashMap::new();
        versions.insert(
            1,
            KeyVersion {
                version: 1,
                created_time_unix_ms,
                key_material: initial_key,
                public_key,
            },
        );

        Self {
            name,
            key_type,
            current_version: 1,
            min_decryption_version: 1,
            min_encryption_version: 0, // 0 means use current
            deletion_allowed: false,
            exportable: false,
            allow_plaintext_backup: false,
            created_time_unix_ms,
            latest_version_time_unix_ms: created_time_unix_ms,
            supports_convergent_encryption: false,
            versions,
        }
    }

    /// Get the current version's key material.
    pub fn current_key_material(&self) -> Option<&[u8]> {
        self.versions.get(&self.current_version).map(|v| v.key_material.as_slice())
    }

    /// Get a specific version's key material.
    pub fn key_material(&self, version: u32) -> Option<&[u8]> {
        self.versions.get(&version).map(|v| v.key_material.as_slice())
    }

    /// Get the effective encryption version.
    pub fn effective_encryption_version(&self) -> u32 {
        if self.min_encryption_version == 0 {
            self.current_version
        } else {
            self.min_encryption_version.max(self.current_version)
        }
    }

    /// Check if a version is valid for decryption.
    pub fn can_decrypt_version(&self, version: u32) -> bool {
        version >= self.min_decryption_version && self.versions.contains_key(&version)
    }

    /// Add a new key version.
    pub fn rotate(&mut self, new_key: Vec<u8>, public_key: Option<Vec<u8>>, created_time_unix_ms: u64) {
        self.current_version += 1;
        self.latest_version_time_unix_ms = created_time_unix_ms;
        self.versions.insert(
            self.current_version,
            KeyVersion {
                version: self.current_version,
                created_time_unix_ms,
                key_material: new_key,
                public_key,
            },
        );
    }
}

/// Request to create a new transit key.
#[derive(Debug, Clone)]
pub struct CreateKeyRequest {
    /// Key name.
    pub name: String,
    /// Key type.
    pub key_type: KeyType,
    /// Whether the key can be exported.
    pub exportable: bool,
    /// Whether deletion is allowed.
    pub deletion_allowed: bool,
    /// Whether to support convergent encryption.
    pub convergent_encryption: bool,
}

impl CreateKeyRequest {
    /// Create a new key creation request.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            key_type: KeyType::default(),
            exportable: false,
            deletion_allowed: false,
            convergent_encryption: false,
        }
    }

    /// Set the key type.
    pub fn with_type(mut self, key_type: KeyType) -> Self {
        self.key_type = key_type;
        self
    }

    /// Make the key exportable.
    pub fn exportable(mut self) -> Self {
        self.exportable = true;
        self
    }

    /// Allow key deletion.
    pub fn deletion_allowed(mut self) -> Self {
        self.deletion_allowed = true;
        self
    }
}

/// Request to encrypt data.
#[derive(Debug, Clone)]
pub struct EncryptRequest {
    /// Key name.
    pub key_name: String,
    /// Plaintext to encrypt.
    pub plaintext: Vec<u8>,
    /// Context for convergent encryption (optional).
    pub context: Option<Vec<u8>>,
    /// Key version to use (0 = latest).
    pub key_version: Option<u32>,
}

impl EncryptRequest {
    /// Create a new encryption request.
    pub fn new(key_name: impl Into<String>, plaintext: Vec<u8>) -> Self {
        Self {
            key_name: key_name.into(),
            plaintext,
            context: None,
            key_version: None,
        }
    }

    /// Set convergent encryption context.
    pub fn with_context(mut self, context: Vec<u8>) -> Self {
        self.context = Some(context);
        self
    }
}

/// Response from encryption.
#[derive(Debug, Clone)]
pub struct EncryptResponse {
    /// Ciphertext in format: `aspen:v<version>:<base64-data>`
    pub ciphertext: String,
    /// Key version used for encryption.
    pub key_version: u32,
}

/// Request to decrypt data.
#[derive(Debug, Clone)]
pub struct DecryptRequest {
    /// Key name.
    pub key_name: String,
    /// Ciphertext to decrypt.
    pub ciphertext: String,
    /// Context for convergent encryption (optional).
    pub context: Option<Vec<u8>>,
}

impl DecryptRequest {
    /// Create a new decryption request.
    pub fn new(key_name: impl Into<String>, ciphertext: String) -> Self {
        Self {
            key_name: key_name.into(),
            ciphertext,
            context: None,
        }
    }
}

/// Response from decryption.
#[derive(Debug, Clone)]
pub struct DecryptResponse {
    /// Decrypted plaintext.
    pub plaintext: Vec<u8>,
}

/// Request to sign data.
#[derive(Debug, Clone)]
pub struct SignRequest {
    /// Key name.
    pub key_name: String,
    /// Data to sign.
    pub input: Vec<u8>,
    /// Hash algorithm (ignored for Ed25519).
    pub hash_algorithm: Option<String>,
    /// Whether input is prehashed.
    pub prehashed: bool,
    /// Key version to use (0 = latest).
    pub key_version: Option<u32>,
}

impl SignRequest {
    /// Create a new sign request.
    pub fn new(key_name: impl Into<String>, input: Vec<u8>) -> Self {
        Self {
            key_name: key_name.into(),
            input,
            hash_algorithm: None,
            prehashed: false,
            key_version: None,
        }
    }
}

/// Response from signing.
#[derive(Debug, Clone)]
pub struct SignResponse {
    /// Signature in format: `aspen:v<version>:<base64-signature>`
    pub signature: String,
    /// Key version used for signing.
    pub key_version: u32,
}

/// Request to verify a signature.
#[derive(Debug, Clone)]
pub struct VerifyRequest {
    /// Key name.
    pub key_name: String,
    /// Original input data.
    pub input: Vec<u8>,
    /// Signature to verify.
    pub signature: String,
    /// Hash algorithm (ignored for Ed25519).
    pub hash_algorithm: Option<String>,
    /// Whether input is prehashed.
    pub prehashed: bool,
}

impl VerifyRequest {
    /// Create a new verify request.
    pub fn new(key_name: impl Into<String>, input: Vec<u8>, signature: String) -> Self {
        Self {
            key_name: key_name.into(),
            input,
            signature,
            hash_algorithm: None,
            prehashed: false,
        }
    }
}

/// Response from verification.
#[derive(Debug, Clone)]
pub struct VerifyResponse {
    /// Whether the signature is valid.
    pub valid: bool,
}

/// Request to rewrap ciphertext with latest key version.
#[derive(Debug, Clone)]
pub struct RewrapRequest {
    /// Key name.
    pub key_name: String,
    /// Ciphertext to rewrap.
    pub ciphertext: String,
    /// Context for convergent encryption (optional).
    pub context: Option<Vec<u8>>,
}

impl RewrapRequest {
    /// Create a new rewrap request.
    pub fn new(key_name: impl Into<String>, ciphertext: String) -> Self {
        Self {
            key_name: key_name.into(),
            ciphertext,
            context: None,
        }
    }
}

/// Response from rewrapping.
#[derive(Debug, Clone)]
pub struct RewrapResponse {
    /// New ciphertext encrypted with latest key version.
    pub ciphertext: String,
    /// New key version used.
    pub key_version: u32,
}

/// Request to generate a data key.
#[derive(Debug, Clone)]
pub struct DataKeyRequest {
    /// Key name.
    pub key_name: String,
    /// Number of bytes for the data key.
    pub bits: u32,
    /// Context for key derivation (optional).
    pub context: Option<Vec<u8>>,
}

impl DataKeyRequest {
    /// Create a new data key request.
    pub fn new(key_name: impl Into<String>, bits: u32) -> Self {
        Self {
            key_name: key_name.into(),
            bits,
            context: None,
        }
    }
}

/// Response with generated data key.
#[derive(Debug, Clone)]
pub struct DataKeyResponse {
    /// Plaintext data key (for use).
    pub plaintext: Vec<u8>,
    /// Wrapped data key (for storage).
    pub ciphertext: String,
    /// Key version used for wrapping.
    pub key_version: u32,
}

/// Request to update key configuration.
#[derive(Debug, Clone)]
pub struct UpdateKeyConfigRequest {
    /// Key name.
    pub name: String,
    /// Minimum decryption version (None = don't change).
    pub min_decryption_version: Option<u32>,
    /// Minimum encryption version (None = don't change).
    pub min_encryption_version: Option<u32>,
    /// Whether deletion is allowed (None = don't change).
    pub deletion_allowed: Option<bool>,
    /// Whether export is allowed (None = don't change).
    pub exportable: Option<bool>,
}

impl UpdateKeyConfigRequest {
    /// Create a new config update request.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            min_decryption_version: None,
            min_encryption_version: None,
            deletion_allowed: None,
            exportable: None,
        }
    }

    /// Set minimum decryption version.
    pub fn with_min_decryption_version(mut self, version: u32) -> Self {
        self.min_decryption_version = Some(version);
        self
    }

    /// Set deletion allowed.
    pub fn with_deletion_allowed(mut self, allowed: bool) -> Self {
        self.deletion_allowed = Some(allowed);
        self
    }
}

/// Batch encryption input.
#[derive(Debug, Clone)]
pub struct BatchEncryptInput {
    /// Plaintext to encrypt.
    pub plaintext: Vec<u8>,
    /// Context for convergent encryption.
    pub context: Option<Vec<u8>>,
}

/// Batch decryption input.
#[derive(Debug, Clone)]
pub struct BatchDecryptInput {
    /// Ciphertext to decrypt.
    pub ciphertext: String,
    /// Context for convergent encryption.
    pub context: Option<Vec<u8>>,
}

/// Batch result item.
#[derive(Debug, Clone)]
pub struct BatchResultItem<T> {
    /// The result if successful.
    pub result: Option<T>,
    /// Error message if failed.
    pub error: Option<String>,
}
