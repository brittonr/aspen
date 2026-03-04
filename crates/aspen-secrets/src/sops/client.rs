//! Aspen Transit client for SOPS key management.
//!
//! Wraps `AspenClient` to provide typed Transit operations needed
//! by SOPS: data key generation, encryption, decryption, and rewrap.

use std::time::Duration;

use aspen_client::AspenClient;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use tracing::debug;
use zeroize::Zeroizing;

use super::sops_constants::DEFAULT_TRANSIT_MOUNT;
use super::sops_error::Result;
use super::sops_error::SopsError;

/// RPC timeout for Transit operations.
const RPC_TIMEOUT: Duration = Duration::from_secs(30);

/// Transit client wrapping the Aspen RPC client.
///
/// Provides typed operations for SOPS data key management:
/// - Generate data keys (envelope encryption)
/// - Encrypt/decrypt data keys
/// - Rewrap data keys after key rotation
pub struct TransitClient {
    client: AspenClient,
    mount: String,
}

impl TransitClient {
    /// Connect to an Aspen cluster.
    ///
    /// # Arguments
    /// * `cluster_ticket` - Aspen cluster ticket string
    /// * `mount` - Transit mount point (default: "transit")
    pub async fn connect(cluster_ticket: &str, mount: Option<&str>) -> Result<Self> {
        let client = AspenClient::connect(cluster_ticket, RPC_TIMEOUT, None)
            .await
            .map_err(|e| SopsError::TransitConnect { reason: e.to_string() })?;

        Ok(Self {
            client,
            mount: mount.unwrap_or(DEFAULT_TRANSIT_MOUNT).to_string(),
        })
    }

    /// Create from an existing `AspenClient`.
    pub fn from_client(client: AspenClient, mount: impl Into<String>) -> Self {
        Self {
            client,
            mount: mount.into(),
        }
    }

    /// Generate a data key for SOPS encryption.
    ///
    /// Returns `(plaintext_data_key, encrypted_data_key, key_version)`.
    /// The plaintext is wrapped in `Zeroizing` for automatic memory cleanup.
    pub async fn generate_data_key(&self, key_name: &str) -> Result<(Zeroizing<Vec<u8>>, String, u32)> {
        let request = ClientRpcRequest::SecretsTransitDatakey {
            mount: self.mount.clone(),
            name: key_name.to_string(),
            key_type: "aes256-gcm".to_string(),
        };

        let response = self.client.send(request).await.map_err(|e| SopsError::TransitEncrypt {
            key_name: key_name.to_string(),
            reason: e.to_string(),
        })?;

        match response {
            ClientRpcResponse::SecretsTransitDatakeyResult(r) if r.is_success => {
                let plaintext = r.plaintext.ok_or_else(|| SopsError::TransitEncrypt {
                    key_name: key_name.to_string(),
                    reason: "no plaintext in data key response".into(),
                })?;
                let ciphertext = r.ciphertext.ok_or_else(|| SopsError::TransitEncrypt {
                    key_name: key_name.to_string(),
                    reason: "no ciphertext in data key response".into(),
                })?;

                // Parse key version from ciphertext prefix "aspen:v<N>:..."
                let key_version = parse_key_version(&ciphertext)?;

                debug!(key = key_name, version = key_version, "Generated data key");

                Ok((Zeroizing::new(plaintext), ciphertext, key_version))
            }
            ClientRpcResponse::SecretsTransitDatakeyResult(r) => Err(SopsError::TransitEncrypt {
                key_name: key_name.to_string(),
                reason: r.error.unwrap_or_else(|| "unknown error".into()),
            }),
            ClientRpcResponse::Error(e) => Err(SopsError::TransitEncrypt {
                key_name: key_name.to_string(),
                reason: e.message,
            }),
            other => Err(SopsError::TransitEncrypt {
                key_name: key_name.to_string(),
                reason: format!("unexpected response: {:?}", other),
            }),
        }
    }

    /// Decrypt an encrypted data key.
    ///
    /// The returned plaintext is wrapped in `Zeroizing` for automatic cleanup.
    pub async fn decrypt_data_key(&self, key_name: &str, ciphertext: &str) -> Result<Zeroizing<Vec<u8>>> {
        let request = ClientRpcRequest::SecretsTransitDecrypt {
            mount: self.mount.clone(),
            name: key_name.to_string(),
            ciphertext: ciphertext.to_string(),
            context: None,
        };

        let response = self.client.send(request).await.map_err(|e| SopsError::TransitDecrypt {
            key_name: key_name.to_string(),
            reason: e.to_string(),
        })?;

        match response {
            ClientRpcResponse::SecretsTransitDecryptResult(r) if r.is_success => {
                let plaintext = r.plaintext.ok_or_else(|| SopsError::TransitDecrypt {
                    key_name: key_name.to_string(),
                    reason: "no plaintext in decrypt response".into(),
                })?;

                debug!(key = key_name, "Decrypted data key");
                Ok(Zeroizing::new(plaintext))
            }
            ClientRpcResponse::SecretsTransitDecryptResult(r) => Err(SopsError::TransitDecrypt {
                key_name: key_name.to_string(),
                reason: r.error.unwrap_or_else(|| "unknown error".into()),
            }),
            ClientRpcResponse::Error(e) => Err(SopsError::TransitDecrypt {
                key_name: key_name.to_string(),
                reason: e.message,
            }),
            other => Err(SopsError::TransitDecrypt {
                key_name: key_name.to_string(),
                reason: format!("unexpected response: {:?}", other),
            }),
        }
    }

    /// Encrypt plaintext data (for wrapping data keys).
    ///
    /// Returns `(ciphertext, key_version)`.
    pub async fn encrypt_data(&self, key_name: &str, plaintext: &[u8]) -> Result<(String, u32)> {
        let request = ClientRpcRequest::SecretsTransitEncrypt {
            mount: self.mount.clone(),
            name: key_name.to_string(),
            plaintext: plaintext.to_vec(),
            context: None,
        };

        let response = self.client.send(request).await.map_err(|e| SopsError::TransitEncrypt {
            key_name: key_name.to_string(),
            reason: e.to_string(),
        })?;

        match response {
            ClientRpcResponse::SecretsTransitEncryptResult(r) if r.is_success => {
                let ciphertext = r.ciphertext.ok_or_else(|| SopsError::TransitEncrypt {
                    key_name: key_name.to_string(),
                    reason: "no ciphertext in encrypt response".into(),
                })?;
                let key_version = parse_key_version(&ciphertext)?;

                debug!(key = key_name, version = key_version, "Encrypted data");
                Ok((ciphertext, key_version))
            }
            ClientRpcResponse::SecretsTransitEncryptResult(r) => Err(SopsError::TransitEncrypt {
                key_name: key_name.to_string(),
                reason: r.error.unwrap_or_else(|| "unknown error".into()),
            }),
            ClientRpcResponse::Error(e) => Err(SopsError::TransitEncrypt {
                key_name: key_name.to_string(),
                reason: e.message,
            }),
            other => Err(SopsError::TransitEncrypt {
                key_name: key_name.to_string(),
                reason: format!("unexpected response: {:?}", other),
            }),
        }
    }

    /// Rewrap a data key with the latest Transit key version.
    ///
    /// Returns `(new_ciphertext, new_key_version)`.
    pub async fn rewrap_data_key(&self, key_name: &str, ciphertext: &str) -> Result<(String, u32)> {
        let request = ClientRpcRequest::SecretsTransitRewrap {
            mount: self.mount.clone(),
            name: key_name.to_string(),
            ciphertext: ciphertext.to_string(),
            context: None,
        };

        let response = self.client.send(request).await.map_err(|e| SopsError::TransitEncrypt {
            key_name: key_name.to_string(),
            reason: e.to_string(),
        })?;

        // Rewrap returns an EncryptResult
        match response {
            ClientRpcResponse::SecretsTransitEncryptResult(r) if r.is_success => {
                let new_ciphertext = r.ciphertext.ok_or_else(|| SopsError::TransitEncrypt {
                    key_name: key_name.to_string(),
                    reason: "no ciphertext in rewrap response".into(),
                })?;
                let key_version = parse_key_version(&new_ciphertext)?;

                debug!(key = key_name, version = key_version, "Rewrapped data key");
                Ok((new_ciphertext, key_version))
            }
            ClientRpcResponse::SecretsTransitEncryptResult(r) => Err(SopsError::TransitEncrypt {
                key_name: key_name.to_string(),
                reason: r.error.unwrap_or_else(|| "unknown error".into()),
            }),
            ClientRpcResponse::Error(e) => Err(SopsError::TransitEncrypt {
                key_name: key_name.to_string(),
                reason: e.message,
            }),
            other => Err(SopsError::TransitEncrypt {
                key_name: key_name.to_string(),
                reason: format!("unexpected response: {:?}", other),
            }),
        }
    }
}

/// Parse key version from Transit ciphertext format `aspen:v<N>:<data>`.
fn parse_key_version(ciphertext: &str) -> Result<u32> {
    let rest = ciphertext.strip_prefix("aspen:v").ok_or_else(|| SopsError::InvalidCiphertext {
        reason: "expected 'aspen:v' prefix".into(),
    })?;

    let colon_pos = rest.find(':').ok_or_else(|| SopsError::InvalidCiphertext {
        reason: "missing version/data separator".into(),
    })?;

    rest[..colon_pos].parse::<u32>().map_err(|e| SopsError::InvalidCiphertext {
        reason: format!("invalid version number: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_version() {
        assert_eq!(parse_key_version("aspen:v1:abc123").unwrap(), 1);
        assert_eq!(parse_key_version("aspen:v42:data").unwrap(), 42);
        assert!(parse_key_version("invalid").is_err());
        assert!(parse_key_version("aspen:vx:data").is_err());
    }
}
