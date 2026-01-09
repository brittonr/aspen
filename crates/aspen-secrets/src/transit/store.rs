//! Transit secrets store implementation.
//!
//! Provides encryption-as-a-service with key management, encryption/decryption,
//! signing/verification, and key rotation following HashiCorp Vault patterns.

use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use async_trait::async_trait;
use base64::Engine;
use tracing::debug;

use crate::backend::SecretsBackend;
use crate::constants::MAX_PLAINTEXT_SIZE;
use crate::constants::MAX_TRANSIT_KEY_NAME_LENGTH;
use crate::constants::TRANSIT_CIPHERTEXT_PREFIX;
use crate::error::Result;
use crate::error::SecretsError;
use crate::transit::types::*;

/// Transit secrets engine store.
///
/// Provides encryption as a service with:
/// - Key creation and management
/// - Encrypt/decrypt operations
/// - Sign/verify operations (for signing keys)
/// - Key rotation and versioning
/// - Data key generation
#[async_trait]
pub trait TransitStore: Send + Sync {
    /// Create a new transit key.
    async fn create_key(&self, request: CreateKeyRequest) -> Result<TransitKey>;

    /// Read a transit key (metadata only, not key material).
    async fn read_key(&self, name: &str) -> Result<Option<TransitKey>>;

    /// Delete a transit key.
    async fn delete_key(&self, name: &str) -> Result<bool>;

    /// List all transit keys.
    async fn list_keys(&self) -> Result<Vec<String>>;

    /// Rotate a transit key to a new version.
    async fn rotate_key(&self, name: &str) -> Result<TransitKey>;

    /// Update key configuration.
    async fn update_key_config(&self, request: UpdateKeyConfigRequest) -> Result<TransitKey>;

    /// Encrypt plaintext.
    async fn encrypt(&self, request: EncryptRequest) -> Result<EncryptResponse>;

    /// Decrypt ciphertext.
    async fn decrypt(&self, request: DecryptRequest) -> Result<DecryptResponse>;

    /// Sign data.
    async fn sign(&self, request: SignRequest) -> Result<SignResponse>;

    /// Verify a signature.
    async fn verify(&self, request: VerifyRequest) -> Result<VerifyResponse>;

    /// Rewrap ciphertext with the latest key version.
    async fn rewrap(&self, request: RewrapRequest) -> Result<RewrapResponse>;

    /// Generate a data key (for envelope encryption).
    async fn generate_data_key(&self, request: DataKeyRequest) -> Result<DataKeyResponse>;
}

/// Default Transit store implementation using SecretsBackend.
pub struct DefaultTransitStore {
    /// Storage backend.
    backend: Arc<dyn SecretsBackend>,
}

impl DefaultTransitStore {
    /// Create a new Transit store with the given backend.
    pub fn new(backend: Arc<dyn SecretsBackend>) -> Self {
        Self { backend }
    }

    /// Get current timestamp in milliseconds.
    fn now_unix_ms() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
    }

    /// Validate a key name.
    fn validate_key_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(SecretsError::TransitKeyNameTooLong { length: 0, max: 1 });
        }
        if name.len() > MAX_TRANSIT_KEY_NAME_LENGTH {
            return Err(SecretsError::TransitKeyNameTooLong {
                length: name.len(),
                max: MAX_TRANSIT_KEY_NAME_LENGTH,
            });
        }
        Ok(())
    }

    /// Get storage path for a key.
    fn key_path(name: &str) -> String {
        format!("keys/{}", name)
    }

    /// Load key from storage.
    async fn load_key(&self, name: &str) -> Result<Option<TransitKey>> {
        let path = Self::key_path(name);
        let data = self.backend.get(&path).await?;

        match data {
            Some(bytes) => {
                let key: TransitKey = postcard::from_bytes(&bytes).map_err(|e| SecretsError::Internal {
                    reason: format!("corrupted key data: {e}"),
                })?;
                Ok(Some(key))
            }
            None => Ok(None),
        }
    }

    /// Save key to storage.
    async fn save_key(&self, key: &TransitKey) -> Result<()> {
        let path = Self::key_path(&key.name);
        let bytes = postcard::to_allocvec(key).map_err(|e| SecretsError::Serialization { reason: e.to_string() })?;
        self.backend.put(&path, &bytes).await
    }

    /// Generate key material for a given key type.
    fn generate_key_material(key_type: KeyType) -> Result<(Vec<u8>, Option<Vec<u8>>)> {
        use rand::RngCore;

        let mut rng = rand::rng();

        match key_type {
            KeyType::Aes256Gcm | KeyType::XChaCha20Poly1305 => {
                let mut key = vec![0u8; key_type.key_size()];
                rng.fill_bytes(&mut key);
                Ok((key, None))
            }
            KeyType::Ed25519 => {
                // Generate Ed25519 keypair
                let mut seed = [0u8; 32];
                rng.fill_bytes(&mut seed);

                let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
                let verifying_key = signing_key.verifying_key();

                Ok((seed.to_vec(), Some(verifying_key.as_bytes().to_vec())))
            }
        }
    }

    /// Parse ciphertext format: `aspen:v<version>:<base64-data>`
    fn parse_ciphertext(ciphertext: &str) -> Result<(u32, Vec<u8>)> {
        if !ciphertext.starts_with(TRANSIT_CIPHERTEXT_PREFIX) {
            return Err(SecretsError::InvalidCiphertext {
                reason: format!("ciphertext must start with '{}'", TRANSIT_CIPHERTEXT_PREFIX),
            });
        }

        let rest = &ciphertext[TRANSIT_CIPHERTEXT_PREFIX.len()..];
        let colon_pos = rest.find(':').ok_or_else(|| SecretsError::InvalidCiphertext {
            reason: "missing version/data separator".into(),
        })?;

        let version_str = &rest[..colon_pos];
        let version: u32 = version_str.parse().map_err(|_| SecretsError::InvalidCiphertext {
            reason: format!("invalid version number: {version_str}"),
        })?;

        let b64_data = &rest[colon_pos + 1..];
        let data = base64::engine::general_purpose::STANDARD.decode(b64_data).map_err(|e| {
            SecretsError::InvalidCiphertext {
                reason: format!("invalid base64: {e}"),
            }
        })?;

        Ok((version, data))
    }

    /// Format ciphertext: `aspen:v<version>:<base64-data>`
    fn format_ciphertext(version: u32, data: &[u8]) -> String {
        let b64 = base64::engine::general_purpose::STANDARD.encode(data);
        format!("{}{}:{}", TRANSIT_CIPHERTEXT_PREFIX, version, b64)
    }

    /// Encrypt using XChaCha20-Poly1305.
    fn encrypt_xchacha(key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
        use chacha20poly1305::XChaCha20Poly1305;
        use chacha20poly1305::aead::Aead;
        use chacha20poly1305::aead::KeyInit;
        use chacha20poly1305::aead::generic_array::GenericArray;
        use rand::RngCore;

        let cipher = XChaCha20Poly1305::new(GenericArray::from_slice(key));

        let mut nonce = [0u8; 24];
        rand::rng().fill_bytes(&mut nonce);
        let nonce_ga = GenericArray::from_slice(&nonce);

        let ciphertext = cipher.encrypt(nonce_ga, plaintext).map_err(|e| SecretsError::Encryption {
            reason: format!("XChaCha20-Poly1305 encryption failed: {e}"),
        })?;

        // Prepend nonce to ciphertext
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// Decrypt using XChaCha20-Poly1305.
    fn decrypt_xchacha(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
        use chacha20poly1305::XChaCha20Poly1305;
        use chacha20poly1305::aead::Aead;
        use chacha20poly1305::aead::KeyInit;
        use chacha20poly1305::aead::generic_array::GenericArray;

        if data.len() < 24 {
            return Err(SecretsError::InvalidCiphertext {
                reason: "data too short for XChaCha20-Poly1305".into(),
            });
        }

        let (nonce, ciphertext) = data.split_at(24);
        let cipher = XChaCha20Poly1305::new(GenericArray::from_slice(key));
        let nonce_ga = GenericArray::from_slice(nonce);

        cipher.decrypt(nonce_ga, ciphertext).map_err(|e| SecretsError::Encryption {
            reason: format!("XChaCha20-Poly1305 decryption failed: {e}"),
        })
    }

    /// Encrypt using AES-256-GCM.
    fn encrypt_aes_gcm(key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::Aes256Gcm;
        use aes_gcm::aead::Aead;
        use aes_gcm::aead::KeyInit;
        use aes_gcm::aead::generic_array::GenericArray;
        use rand::RngCore;

        let cipher = Aes256Gcm::new(GenericArray::from_slice(key));

        let mut nonce = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce);
        let nonce_ga = GenericArray::from_slice(&nonce);

        let ciphertext = cipher.encrypt(nonce_ga, plaintext).map_err(|e| SecretsError::Encryption {
            reason: format!("AES-256-GCM encryption failed: {e}"),
        })?;

        // Prepend nonce to ciphertext
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// Decrypt using AES-256-GCM.
    fn decrypt_aes_gcm(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::Aes256Gcm;
        use aes_gcm::aead::Aead;
        use aes_gcm::aead::KeyInit;
        use aes_gcm::aead::generic_array::GenericArray;

        if data.len() < 12 {
            return Err(SecretsError::InvalidCiphertext {
                reason: "data too short for AES-256-GCM".into(),
            });
        }

        let (nonce, ciphertext) = data.split_at(12);
        let cipher = Aes256Gcm::new(GenericArray::from_slice(key));
        let nonce_ga = GenericArray::from_slice(nonce);

        cipher.decrypt(nonce_ga, ciphertext).map_err(|e| SecretsError::Encryption {
            reason: format!("AES-256-GCM decryption failed: {e}"),
        })
    }

    /// Sign with Ed25519.
    fn sign_ed25519(key: &[u8], message: &[u8]) -> Result<Vec<u8>> {
        use ed25519_dalek::Signer;

        if key.len() != 32 {
            return Err(SecretsError::Internal {
                reason: "invalid Ed25519 key length".into(),
            });
        }

        let mut seed = [0u8; 32];
        seed.copy_from_slice(key);

        let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
        let signature = signing_key.sign(message);

        Ok(signature.to_bytes().to_vec())
    }

    /// Verify Ed25519 signature.
    fn verify_ed25519(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<bool> {
        use ed25519_dalek::Verifier;

        if public_key.len() != 32 {
            return Ok(false);
        }
        if signature.len() != 64 {
            return Ok(false);
        }

        let verifying_key = match ed25519_dalek::VerifyingKey::from_bytes(public_key.try_into().map_err(|_| {
            SecretsError::Internal {
                reason: "invalid public key length".into(),
            }
        })?) {
            Ok(k) => k,
            Err(_) => return Ok(false),
        };

        let sig = ed25519_dalek::Signature::from_bytes(signature.try_into().map_err(|_| SecretsError::Internal {
            reason: "invalid signature length".into(),
        })?);

        Ok(verifying_key.verify(message, &sig).is_ok())
    }
}

#[async_trait]
impl TransitStore for DefaultTransitStore {
    async fn create_key(&self, request: CreateKeyRequest) -> Result<TransitKey> {
        Self::validate_key_name(&request.name)?;

        // Check if key already exists
        if self.load_key(&request.name).await?.is_some() {
            return Err(SecretsError::TransitKeyExists { name: request.name });
        }

        // Generate key material
        let (key_material, public_key) = Self::generate_key_material(request.key_type)?;
        let now = Self::now_unix_ms();

        let mut key = TransitKey::new(request.name.clone(), request.key_type, now, key_material, public_key);
        key.exportable = request.exportable;
        key.deletion_allowed = request.deletion_allowed;
        key.supports_convergent_encryption = request.convergent_encryption;

        self.save_key(&key).await?;

        debug!(name = %request.name, key_type = ?request.key_type, "Created transit key");

        Ok(key)
    }

    async fn read_key(&self, name: &str) -> Result<Option<TransitKey>> {
        Self::validate_key_name(name)?;
        self.load_key(name).await
    }

    async fn delete_key(&self, name: &str) -> Result<bool> {
        Self::validate_key_name(name)?;

        let key = match self.load_key(name).await? {
            Some(k) => k,
            None => return Ok(false),
        };

        if !key.deletion_allowed {
            return Err(SecretsError::KeyDeletionNotAllowed { name: name.into() });
        }

        let path = Self::key_path(name);
        self.backend.delete(&path).await?;

        debug!(name = %name, "Deleted transit key");

        Ok(true)
    }

    async fn list_keys(&self) -> Result<Vec<String>> {
        let keys = self.backend.list("keys/").await?;
        Ok(keys.into_iter().filter(|k| !k.ends_with('/')).collect())
    }

    async fn rotate_key(&self, name: &str) -> Result<TransitKey> {
        Self::validate_key_name(name)?;

        let mut key =
            self.load_key(name).await?.ok_or_else(|| SecretsError::TransitKeyNotFound { name: name.into() })?;

        let (key_material, public_key) = Self::generate_key_material(key.key_type)?;
        let now = Self::now_unix_ms();

        key.rotate(key_material, public_key, now);
        self.save_key(&key).await?;

        debug!(name = %name, version = key.current_version, "Rotated transit key");

        Ok(key)
    }

    async fn update_key_config(&self, request: UpdateKeyConfigRequest) -> Result<TransitKey> {
        Self::validate_key_name(&request.name)?;

        let mut key = self.load_key(&request.name).await?.ok_or_else(|| SecretsError::TransitKeyNotFound {
            name: request.name.clone(),
        })?;

        if let Some(min_dec) = request.min_decryption_version {
            // Can only increase min_decryption_version
            if min_dec > key.current_version {
                return Err(SecretsError::KeyVersionTooOld {
                    name: request.name,
                    version: min_dec,
                    min_version: key.current_version,
                });
            }
            key.min_decryption_version = min_dec;
        }

        if let Some(min_enc) = request.min_encryption_version {
            key.min_encryption_version = min_enc;
        }

        if let Some(allowed) = request.deletion_allowed {
            key.deletion_allowed = allowed;
        }

        if let Some(exportable) = request.exportable {
            // Can only make non-exportable keys exportable if they were created exportable
            if exportable && !key.exportable {
                return Err(SecretsError::KeyExportNotAllowed { name: request.name });
            }
        }

        self.save_key(&key).await?;

        debug!(name = %key.name, "Updated transit key config");

        Ok(key)
    }

    async fn encrypt(&self, request: EncryptRequest) -> Result<EncryptResponse> {
        Self::validate_key_name(&request.key_name)?;

        if request.plaintext.len() > MAX_PLAINTEXT_SIZE {
            return Err(SecretsError::PlaintextTooLarge {
                size: request.plaintext.len(),
                max: MAX_PLAINTEXT_SIZE,
            });
        }

        let key = self.load_key(&request.key_name).await?.ok_or_else(|| SecretsError::TransitKeyNotFound {
            name: request.key_name.clone(),
        })?;

        if !key.key_type.supports_encryption() {
            return Err(SecretsError::UnsupportedKeyType {
                key_type: format!("{:?} does not support encryption", key.key_type),
            });
        }

        let version = request.key_version.unwrap_or_else(|| key.effective_encryption_version());

        let key_material = key.key_material(version).ok_or_else(|| SecretsError::VersionNotFound {
            path: request.key_name.clone(),
            version: version as u64,
        })?;

        let ciphertext = match key.key_type {
            KeyType::XChaCha20Poly1305 => Self::encrypt_xchacha(key_material, &request.plaintext)?,
            KeyType::Aes256Gcm => Self::encrypt_aes_gcm(key_material, &request.plaintext)?,
            _ => unreachable!(),
        };

        let formatted = Self::format_ciphertext(version, &ciphertext);

        debug!(key = %request.key_name, version = version, "Encrypted data");

        Ok(EncryptResponse {
            ciphertext: formatted,
            key_version: version,
        })
    }

    async fn decrypt(&self, request: DecryptRequest) -> Result<DecryptResponse> {
        Self::validate_key_name(&request.key_name)?;

        let key = self.load_key(&request.key_name).await?.ok_or_else(|| SecretsError::TransitKeyNotFound {
            name: request.key_name.clone(),
        })?;

        let (version, data) = Self::parse_ciphertext(&request.ciphertext)?;

        if !key.can_decrypt_version(version) {
            return Err(SecretsError::KeyVersionTooOld {
                name: request.key_name,
                version,
                min_version: key.min_decryption_version,
            });
        }

        let key_material = key.key_material(version).ok_or_else(|| SecretsError::VersionNotFound {
            path: request.key_name.clone(),
            version: version as u64,
        })?;

        let plaintext = match key.key_type {
            KeyType::XChaCha20Poly1305 => Self::decrypt_xchacha(key_material, &data)?,
            KeyType::Aes256Gcm => Self::decrypt_aes_gcm(key_material, &data)?,
            _ => {
                return Err(SecretsError::UnsupportedKeyType {
                    key_type: format!("{:?} does not support decryption", key.key_type),
                });
            }
        };

        debug!(key = %request.key_name, version = version, "Decrypted data");

        Ok(DecryptResponse { plaintext })
    }

    async fn sign(&self, request: SignRequest) -> Result<SignResponse> {
        Self::validate_key_name(&request.key_name)?;

        let key = self.load_key(&request.key_name).await?.ok_or_else(|| SecretsError::TransitKeyNotFound {
            name: request.key_name.clone(),
        })?;

        if !key.key_type.supports_signing() {
            return Err(SecretsError::UnsupportedKeyType {
                key_type: format!("{:?} does not support signing", key.key_type),
            });
        }

        let version = request.key_version.unwrap_or(key.current_version);

        let key_material = key.key_material(version).ok_or_else(|| SecretsError::VersionNotFound {
            path: request.key_name.clone(),
            version: version as u64,
        })?;

        let signature = match key.key_type {
            KeyType::Ed25519 => Self::sign_ed25519(key_material, &request.input)?,
            _ => unreachable!(),
        };

        let formatted = Self::format_ciphertext(version, &signature);

        debug!(key = %request.key_name, version = version, "Signed data");

        Ok(SignResponse {
            signature: formatted,
            key_version: version,
        })
    }

    async fn verify(&self, request: VerifyRequest) -> Result<VerifyResponse> {
        Self::validate_key_name(&request.key_name)?;

        let key = self.load_key(&request.key_name).await?.ok_or_else(|| SecretsError::TransitKeyNotFound {
            name: request.key_name.clone(),
        })?;

        let (version, signature) = Self::parse_ciphertext(&request.signature)?;

        let public_key = key.versions.get(&version).and_then(|v| v.public_key.as_ref()).ok_or_else(|| {
            SecretsError::VersionNotFound {
                path: request.key_name.clone(),
                version: version as u64,
            }
        })?;

        let valid = match key.key_type {
            KeyType::Ed25519 => Self::verify_ed25519(public_key, &request.input, &signature)?,
            _ => {
                return Err(SecretsError::UnsupportedKeyType {
                    key_type: format!("{:?} does not support verification", key.key_type),
                });
            }
        };

        if !valid {
            return Err(SecretsError::SignatureVerificationFailed { name: request.key_name });
        }

        debug!(key = %request.key_name, version = version, valid = valid, "Verified signature");

        Ok(VerifyResponse { valid })
    }

    async fn rewrap(&self, request: RewrapRequest) -> Result<RewrapResponse> {
        // Decrypt with old key version
        let decrypt_req = DecryptRequest::new(&request.key_name, request.ciphertext);
        let decrypt_res = self.decrypt(decrypt_req).await?;

        // Encrypt with new key version
        let encrypt_req = EncryptRequest::new(&request.key_name, decrypt_res.plaintext);
        let encrypt_res = self.encrypt(encrypt_req).await?;

        debug!(key = %request.key_name, new_version = encrypt_res.key_version, "Rewrapped ciphertext");

        Ok(RewrapResponse {
            ciphertext: encrypt_res.ciphertext,
            key_version: encrypt_res.key_version,
        })
    }

    async fn generate_data_key(&self, request: DataKeyRequest) -> Result<DataKeyResponse> {
        use rand::RngCore;

        Self::validate_key_name(&request.key_name)?;

        // Validate bits (must be multiple of 8, reasonable size)
        if !request.bits.is_multiple_of(8) || request.bits < 128 || request.bits > 512 {
            return Err(SecretsError::PlaintextTooLarge {
                size: request.bits as usize,
                max: 512,
            });
        }

        let bytes = (request.bits / 8) as usize;
        let mut plaintext = vec![0u8; bytes];
        rand::rng().fill_bytes(&mut plaintext);

        // Encrypt the data key
        let encrypt_req = EncryptRequest::new(&request.key_name, plaintext.clone());
        let encrypt_res = self.encrypt(encrypt_req).await?;

        debug!(key = %request.key_name, bits = request.bits, "Generated data key");

        Ok(DataKeyResponse {
            plaintext,
            ciphertext: encrypt_res.ciphertext,
            key_version: encrypt_res.key_version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::InMemorySecretsBackend;

    fn make_store() -> DefaultTransitStore {
        let backend = Arc::new(InMemorySecretsBackend::new());
        DefaultTransitStore::new(backend)
    }

    #[tokio::test]
    async fn test_create_and_read_key() {
        let store = make_store();

        let req = CreateKeyRequest::new("test-key").with_type(KeyType::XChaCha20Poly1305);
        let key = store.create_key(req).await.unwrap();
        assert_eq!(key.name, "test-key");
        assert_eq!(key.key_type, KeyType::XChaCha20Poly1305);
        assert_eq!(key.current_version, 1);

        let read_key = store.read_key("test-key").await.unwrap().unwrap();
        assert_eq!(read_key.name, "test-key");
    }

    #[tokio::test]
    async fn test_encrypt_decrypt() {
        let store = make_store();

        store
            .create_key(CreateKeyRequest::new("enc-key").with_type(KeyType::XChaCha20Poly1305))
            .await
            .unwrap();

        let plaintext = b"secret data".to_vec();
        let enc_res = store.encrypt(EncryptRequest::new("enc-key", plaintext.clone())).await.unwrap();

        assert!(enc_res.ciphertext.starts_with("aspen:v1:"));
        assert_eq!(enc_res.key_version, 1);

        let dec_res = store.decrypt(DecryptRequest::new("enc-key", enc_res.ciphertext)).await.unwrap();
        assert_eq!(dec_res.plaintext, plaintext);
    }

    #[tokio::test]
    async fn test_sign_verify() {
        let store = make_store();

        store.create_key(CreateKeyRequest::new("sign-key").with_type(KeyType::Ed25519)).await.unwrap();

        let message = b"message to sign".to_vec();
        let sign_res = store.sign(SignRequest::new("sign-key", message.clone())).await.unwrap();

        assert!(sign_res.signature.starts_with("aspen:v1:"));

        let verify_res = store.verify(VerifyRequest::new("sign-key", message, sign_res.signature)).await.unwrap();
        assert!(verify_res.valid);
    }

    #[tokio::test]
    async fn test_key_rotation() {
        let store = make_store();

        store
            .create_key(CreateKeyRequest::new("rotate-key").with_type(KeyType::XChaCha20Poly1305))
            .await
            .unwrap();

        // Encrypt with v1
        let plaintext = b"data".to_vec();
        let enc1 = store.encrypt(EncryptRequest::new("rotate-key", plaintext.clone())).await.unwrap();
        assert_eq!(enc1.key_version, 1);

        // Rotate key
        let rotated = store.rotate_key("rotate-key").await.unwrap();
        assert_eq!(rotated.current_version, 2);

        // New encryption uses v2
        let enc2 = store.encrypt(EncryptRequest::new("rotate-key", plaintext.clone())).await.unwrap();
        assert_eq!(enc2.key_version, 2);

        // Can still decrypt v1
        let dec1 = store.decrypt(DecryptRequest::new("rotate-key", enc1.ciphertext)).await.unwrap();
        assert_eq!(dec1.plaintext, plaintext);
    }

    #[tokio::test]
    async fn test_rewrap() {
        let store = make_store();

        store
            .create_key(CreateKeyRequest::new("rewrap-key").with_type(KeyType::XChaCha20Poly1305))
            .await
            .unwrap();

        let plaintext = b"data".to_vec();
        let enc1 = store.encrypt(EncryptRequest::new("rewrap-key", plaintext.clone())).await.unwrap();

        // Rotate key
        store.rotate_key("rewrap-key").await.unwrap();

        // Rewrap
        let rewrap_res = store.rewrap(RewrapRequest::new("rewrap-key", enc1.ciphertext)).await.unwrap();
        assert_eq!(rewrap_res.key_version, 2);

        // Can decrypt rewrapped ciphertext
        let dec = store.decrypt(DecryptRequest::new("rewrap-key", rewrap_res.ciphertext)).await.unwrap();
        assert_eq!(dec.plaintext, plaintext);
    }

    #[tokio::test]
    async fn test_generate_data_key() {
        let store = make_store();

        store
            .create_key(CreateKeyRequest::new("data-key").with_type(KeyType::XChaCha20Poly1305))
            .await
            .unwrap();

        let dk = store.generate_data_key(DataKeyRequest::new("data-key", 256)).await.unwrap();

        assert_eq!(dk.plaintext.len(), 32);
        assert!(dk.ciphertext.starts_with("aspen:v1:"));

        // Can decrypt to get the same key
        let dec = store.decrypt(DecryptRequest::new("data-key", dk.ciphertext)).await.unwrap();
        assert_eq!(dec.plaintext, dk.plaintext);
    }

    #[tokio::test]
    async fn test_min_decryption_version() {
        let store = make_store();

        store
            .create_key(CreateKeyRequest::new("min-key").with_type(KeyType::XChaCha20Poly1305))
            .await
            .unwrap();

        // Encrypt with v1
        let plaintext = b"data".to_vec();
        let enc1 = store.encrypt(EncryptRequest::new("min-key", plaintext.clone())).await.unwrap();

        // Rotate twice
        store.rotate_key("min-key").await.unwrap();
        store.rotate_key("min-key").await.unwrap();

        // Set min_decryption_version to 2
        store
            .update_key_config(UpdateKeyConfigRequest::new("min-key").with_min_decryption_version(2))
            .await
            .unwrap();

        // Can't decrypt v1 anymore
        let res = store.decrypt(DecryptRequest::new("min-key", enc1.ciphertext)).await;
        assert!(matches!(res.unwrap_err(), SecretsError::KeyVersionTooOld { .. }));
    }

    #[tokio::test]
    async fn test_delete_key() {
        let store = make_store();

        // Create with deletion allowed
        store.create_key(CreateKeyRequest::new("del-key").deletion_allowed()).await.unwrap();

        // Delete succeeds
        assert!(store.delete_key("del-key").await.unwrap());

        // Key no longer exists
        assert!(store.read_key("del-key").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_key_not_allowed() {
        let store = make_store();

        // Create without deletion allowed
        store.create_key(CreateKeyRequest::new("nodelete-key")).await.unwrap();

        // Delete fails
        let res = store.delete_key("nodelete-key").await;
        assert!(matches!(res.unwrap_err(), SecretsError::KeyDeletionNotAllowed { .. }));
    }
}
