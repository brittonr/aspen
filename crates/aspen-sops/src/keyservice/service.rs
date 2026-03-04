//! SOPS KeyService gRPC implementation backed by Aspen Transit.
//!
//! Translates SOPS Encrypt/Decrypt gRPC calls into Aspen Transit RPCs.
//! Only `VaultKey` key types are supported — all others return UNIMPLEMENTED.

use tracing::debug;
use tracing::warn;

use crate::client::TransitClient;

/// Generated proto types from `proto/keyservice.proto`.
#[allow(clippy::enum_variant_names)]
pub mod proto {
    tonic::include_proto!("_");
}

use proto::DecryptRequest;
use proto::DecryptResponse;
use proto::EncryptRequest;
use proto::EncryptResponse;
use proto::key::KeyType;
use proto::key_service_server::KeyService;

/// Aspen Transit-backed SOPS key service.
///
/// Receives gRPC Encrypt/Decrypt requests from the Go `sops` binary
/// and forwards them to Aspen Transit via Iroh QUIC.
pub struct AspenKeyService {
    /// Cluster ticket for connecting to the Aspen cluster.
    cluster_ticket: String,
    /// Default Transit key name (fallback when VaultKey.key_name is empty).
    default_key: String,
    /// Default Transit mount (fallback when VaultKey.engine_path is empty).
    default_mount: String,
}

impl AspenKeyService {
    /// Create a new key service instance.
    pub fn new(cluster_ticket: String, default_key: String, default_mount: String) -> Self {
        Self {
            cluster_ticket,
            default_key,
            default_mount,
        }
    }

    /// Connect a Transit client for the given mount point.
    async fn connect_transit(&self, mount: &str) -> Result<TransitClient, tonic::Status> {
        TransitClient::connect(&self.cluster_ticket, Some(mount)).await.map_err(|e| {
            warn!(error = %e, "Failed to connect to Aspen Transit");
            tonic::Status::unavailable(format!("failed to connect to Aspen Transit: {e}"))
        })
    }

    /// Extract mount and key name from a VaultKey, using defaults for empty fields.
    fn resolve_vault_key(&self, vault_key: &proto::VaultKey) -> (String, String) {
        let mount = if vault_key.engine_path.is_empty() {
            self.default_mount.clone()
        } else {
            vault_key.engine_path.clone()
        };

        let key_name = if vault_key.key_name.is_empty() {
            self.default_key.clone()
        } else {
            vault_key.key_name.clone()
        };

        (mount, key_name)
    }
}

#[tonic::async_trait]
impl KeyService for AspenKeyService {
    /// Encrypt plaintext data (a data key) using Aspen Transit.
    ///
    /// SOPS sends the plaintext data key in the request. We encrypt it
    /// via Transit and return the ciphertext (e.g., `aspen:v1:base64...`).
    async fn encrypt(
        &self,
        request: tonic::Request<EncryptRequest>,
    ) -> Result<tonic::Response<EncryptResponse>, tonic::Status> {
        let req = request.into_inner();

        let key = req.key.ok_or_else(|| tonic::Status::invalid_argument("missing key in EncryptRequest"))?;

        let key_type = key.key_type.ok_or_else(|| tonic::Status::invalid_argument("missing key_type in Key"))?;

        match key_type {
            KeyType::VaultKey(vault_key) => {
                let (mount, key_name) = self.resolve_vault_key(&vault_key);

                debug!(mount = mount, key = key_name, plaintext_len = req.plaintext.len(), "Encrypting via Transit");

                let transit = self.connect_transit(&mount).await?;

                let (ciphertext, key_version) = transit.encrypt_data(&key_name, &req.plaintext).await.map_err(|e| {
                    warn!(error = %e, key = key_name, "Transit encrypt failed");
                    tonic::Status::internal(format!("Transit encrypt failed: {e}"))
                })?;

                debug!(key = key_name, version = key_version, "Encrypted data key via Transit");

                Ok(tonic::Response::new(EncryptResponse {
                    ciphertext: ciphertext.into_bytes(),
                }))
            }

            KeyType::KmsKey(_) => {
                Err(tonic::Status::unimplemented("AWS KMS not supported — use VaultKey with Aspen Transit"))
            }
            KeyType::PgpKey(_) => {
                Err(tonic::Status::unimplemented("PGP not supported — use VaultKey with Aspen Transit"))
            }
            KeyType::GcpKmsKey(_) => {
                Err(tonic::Status::unimplemented("GCP KMS not supported — use VaultKey with Aspen Transit"))
            }
            KeyType::AzureKeyvaultKey(_) => {
                Err(tonic::Status::unimplemented("Azure Key Vault not supported — use VaultKey with Aspen Transit"))
            }
            KeyType::AgeKey(_) => {
                Err(tonic::Status::unimplemented("Age not supported via keyservice — use VaultKey with Aspen Transit"))
            }
            KeyType::HckmsKey(_) => {
                Err(tonic::Status::unimplemented("HCKMS not supported — use VaultKey with Aspen Transit"))
            }
        }
    }

    /// Decrypt ciphertext (a wrapped data key) using Aspen Transit.
    ///
    /// SOPS sends the Transit ciphertext (e.g., `aspen:v1:base64...`) as bytes.
    /// We decrypt it via Transit and return the plaintext data key.
    async fn decrypt(
        &self,
        request: tonic::Request<DecryptRequest>,
    ) -> Result<tonic::Response<DecryptResponse>, tonic::Status> {
        let req = request.into_inner();

        let key = req.key.ok_or_else(|| tonic::Status::invalid_argument("missing key in DecryptRequest"))?;

        let key_type = key.key_type.ok_or_else(|| tonic::Status::invalid_argument("missing key_type in Key"))?;

        match key_type {
            KeyType::VaultKey(vault_key) => {
                let (mount, key_name) = self.resolve_vault_key(&vault_key);

                // The ciphertext is a Transit-format string sent as bytes
                let ciphertext_str = String::from_utf8(req.ciphertext)
                    .map_err(|e| tonic::Status::invalid_argument(format!("ciphertext is not valid UTF-8: {e}")))?;

                debug!(mount = mount, key = key_name, "Decrypting via Transit");

                let transit = self.connect_transit(&mount).await?;

                let plaintext = transit.decrypt_data_key(&key_name, &ciphertext_str).await.map_err(|e| {
                    warn!(error = %e, key = key_name, "Transit decrypt failed");
                    tonic::Status::internal(format!("Transit decrypt failed: {e}"))
                })?;

                debug!(key = key_name, "Decrypted data key via Transit");

                Ok(tonic::Response::new(DecryptResponse {
                    plaintext: plaintext.to_vec(),
                }))
            }

            KeyType::KmsKey(_) => {
                Err(tonic::Status::unimplemented("AWS KMS not supported — use VaultKey with Aspen Transit"))
            }
            KeyType::PgpKey(_) => {
                Err(tonic::Status::unimplemented("PGP not supported — use VaultKey with Aspen Transit"))
            }
            KeyType::GcpKmsKey(_) => {
                Err(tonic::Status::unimplemented("GCP KMS not supported — use VaultKey with Aspen Transit"))
            }
            KeyType::AzureKeyvaultKey(_) => {
                Err(tonic::Status::unimplemented("Azure Key Vault not supported — use VaultKey with Aspen Transit"))
            }
            KeyType::AgeKey(_) => {
                Err(tonic::Status::unimplemented("Age not supported via keyservice — use VaultKey with Aspen Transit"))
            }
            KeyType::HckmsKey(_) => {
                Err(tonic::Status::unimplemented("HCKMS not supported — use VaultKey with Aspen Transit"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::proto::key::KeyType;
    use super::proto::*;

    #[test]
    fn test_encrypt_request_roundtrip() {
        use prost::Message;

        let req = EncryptRequest {
            key: Some(Key {
                key_type: Some(KeyType::VaultKey(VaultKey {
                    vault_address: String::new(),
                    engine_path: "transit".into(),
                    key_name: "sops-key".into(),
                })),
            }),
            plaintext: vec![1, 2, 3, 4],
        };

        let mut buf = Vec::new();
        req.encode(&mut buf).unwrap();

        let decoded = EncryptRequest::decode(&buf[..]).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_decrypt_request_roundtrip() {
        use prost::Message;

        let req = DecryptRequest {
            key: Some(Key {
                key_type: Some(KeyType::VaultKey(VaultKey {
                    vault_address: String::new(),
                    engine_path: "transit".into(),
                    key_name: "sops-key".into(),
                })),
            }),
            ciphertext: b"aspen:v1:base64data".to_vec(),
        };

        let mut buf = Vec::new();
        req.encode(&mut buf).unwrap();

        let decoded = DecryptRequest::decode(&buf[..]).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_unsupported_key_types() {
        // Verify that non-VaultKey types are recognized in the proto
        let kms = KeyType::KmsKey(KmsKey {
            arn: "arn:aws:kms:us-east-1:123:key/abc".into(),
            role: String::new(),
            context: Default::default(),
            aws_profile: String::new(),
        });
        assert!(matches!(kms, KeyType::KmsKey(_)));

        let pgp = KeyType::PgpKey(PgpKey {
            fingerprint: "ABCD1234".into(),
        });
        assert!(matches!(pgp, KeyType::PgpKey(_)));

        let age = KeyType::AgeKey(AgeKey {
            recipient: "age1...".into(),
        });
        assert!(matches!(age, KeyType::AgeKey(_)));
    }

    #[test]
    fn test_resolve_vault_key_defaults() {
        let svc = super::AspenKeyService::new("ticket".into(), "default-key".into(), "default-mount".into());

        // Explicit values
        let vk = VaultKey {
            vault_address: String::new(),
            engine_path: "custom-mount".into(),
            key_name: "custom-key".into(),
        };
        let (mount, key) = svc.resolve_vault_key(&vk);
        assert_eq!(mount, "custom-mount");
        assert_eq!(key, "custom-key");

        // Empty → defaults
        let vk = VaultKey {
            vault_address: String::new(),
            engine_path: String::new(),
            key_name: String::new(),
        };
        let (mount, key) = svc.resolve_vault_key(&vk);
        assert_eq!(mount, "default-mount");
        assert_eq!(key, "default-key");
    }
}
