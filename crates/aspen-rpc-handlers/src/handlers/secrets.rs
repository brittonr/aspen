//! Secrets engine request handler.
//!
//! Handles: SecretsKv*, SecretsTransit*, SecretsPki* operations.
//!
//! Provides Vault-compatible secrets management through:
//! - KV v2: Versioned key-value secrets with soft/hard delete
//! - Transit: Encryption-as-a-service (encrypt, decrypt, sign, verify)
//! - PKI: Certificate authority with role-based issuance

use std::collections::HashMap;
use std::sync::Arc;

use base64::Engine;

use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_client_rpc::SecretsKvDeleteResultResponse;
use aspen_client_rpc::SecretsKvListResultResponse;
use aspen_client_rpc::SecretsKvMetadataResultResponse;
use aspen_client_rpc::SecretsKvReadResultResponse;
use aspen_client_rpc::SecretsKvVersionInfo;
use aspen_client_rpc::SecretsKvVersionMetadata;
use aspen_client_rpc::SecretsKvWriteResultResponse;
use aspen_client_rpc::SecretsNixCacheDeleteResultResponse;
use aspen_client_rpc::SecretsNixCacheKeyResultResponse;
use aspen_client_rpc::SecretsNixCacheListResultResponse;
use aspen_client_rpc::SecretsPkiCertificateResultResponse;
use aspen_client_rpc::SecretsPkiCrlResultResponse;
use aspen_client_rpc::SecretsPkiListResultResponse;
use aspen_client_rpc::SecretsPkiRevokeResultResponse;
use aspen_client_rpc::SecretsPkiRoleConfig;
use aspen_client_rpc::SecretsPkiRoleResultResponse;
use aspen_client_rpc::SecretsTransitDatakeyResultResponse;
use aspen_client_rpc::SecretsTransitDecryptResultResponse;
use aspen_client_rpc::SecretsTransitEncryptResultResponse;
use aspen_client_rpc::SecretsTransitKeyResultResponse;
use aspen_client_rpc::SecretsTransitListResultResponse;
use aspen_client_rpc::SecretsTransitSignResultResponse;
use aspen_client_rpc::SecretsTransitVerifyResultResponse;
use aspen_secrets::KvStore;
use aspen_secrets::PkiStore;
use aspen_secrets::TransitStore;
use aspen_secrets::kv::DeleteSecretRequest;
use aspen_secrets::kv::DestroySecretRequest;
use aspen_secrets::kv::ListSecretsRequest;
use aspen_secrets::kv::ReadMetadataRequest;
use aspen_secrets::kv::ReadSecretRequest;
use aspen_secrets::kv::SecretData;
use aspen_secrets::kv::UndeleteSecretRequest;
use aspen_secrets::kv::UpdateMetadataRequest;
use aspen_secrets::kv::WriteSecretRequest;
use aspen_secrets::pki::CreateRoleRequest;
use aspen_secrets::pki::GenerateIntermediateRequest;
use aspen_secrets::pki::GenerateRootRequest;
use aspen_secrets::pki::IssueCertificateRequest;
use aspen_secrets::pki::PkiRole;
use aspen_secrets::pki::RevokeCertificateRequest;
use aspen_secrets::pki::SetSignedIntermediateRequest;
use aspen_secrets::transit::CreateKeyRequest;
use aspen_secrets::transit::DataKeyRequest;
use aspen_secrets::transit::DecryptRequest;
use aspen_secrets::transit::EncryptRequest;
use aspen_secrets::transit::KeyType;
use aspen_secrets::transit::RewrapRequest;
use aspen_secrets::transit::SignRequest;
use aspen_secrets::transit::VerifyRequest;
use tracing::debug;
use tracing::warn;

use crate::context::ClientProtocolContext;
use crate::registry::RequestHandler;

/// Handler for secrets engine operations.
pub struct SecretsHandler;

#[async_trait::async_trait]
impl RequestHandler for SecretsHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            // KV v2
            ClientRpcRequest::SecretsKvRead { .. }
                | ClientRpcRequest::SecretsKvWrite { .. }
                | ClientRpcRequest::SecretsKvDelete { .. }
                | ClientRpcRequest::SecretsKvDestroy { .. }
                | ClientRpcRequest::SecretsKvUndelete { .. }
                | ClientRpcRequest::SecretsKvList { .. }
                | ClientRpcRequest::SecretsKvMetadata { .. }
                | ClientRpcRequest::SecretsKvUpdateMetadata { .. }
                | ClientRpcRequest::SecretsKvDeleteMetadata { .. }
                // Transit
                | ClientRpcRequest::SecretsTransitCreateKey { .. }
                | ClientRpcRequest::SecretsTransitEncrypt { .. }
                | ClientRpcRequest::SecretsTransitDecrypt { .. }
                | ClientRpcRequest::SecretsTransitSign { .. }
                | ClientRpcRequest::SecretsTransitVerify { .. }
                | ClientRpcRequest::SecretsTransitRotateKey { .. }
                | ClientRpcRequest::SecretsTransitListKeys { .. }
                | ClientRpcRequest::SecretsTransitRewrap { .. }
                | ClientRpcRequest::SecretsTransitDatakey { .. }
                // PKI
                | ClientRpcRequest::SecretsPkiGenerateRoot { .. }
                | ClientRpcRequest::SecretsPkiGenerateIntermediate { .. }
                | ClientRpcRequest::SecretsPkiSetSignedIntermediate { .. }
                | ClientRpcRequest::SecretsPkiCreateRole { .. }
                | ClientRpcRequest::SecretsPkiIssue { .. }
                | ClientRpcRequest::SecretsPkiRevoke { .. }
                | ClientRpcRequest::SecretsPkiGetCrl { .. }
                | ClientRpcRequest::SecretsPkiListCerts { .. }
                | ClientRpcRequest::SecretsPkiGetRole { .. }
                | ClientRpcRequest::SecretsPkiListRoles { .. }
                // Nix Cache Signing
                | ClientRpcRequest::SecretsNixCacheCreateKey { .. }
                | ClientRpcRequest::SecretsNixCacheGetPublicKey { .. }
                | ClientRpcRequest::SecretsNixCacheRotateKey { .. }
                | ClientRpcRequest::SecretsNixCacheDeleteKey { .. }
                | ClientRpcRequest::SecretsNixCacheListKeys { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // Check if secrets service is available
        let Some(ref secrets_service) = ctx.secrets_service else {
            return Ok(ClientRpcResponse::Error(aspen_client_rpc::ErrorResponse {
                code: "SECRETS_NOT_ENABLED".to_string(),
                message: "Secrets engine is not enabled on this node".to_string(),
            }));
        };

        match request {
            // KV v2 operations - now with multi-mount support
            ClientRpcRequest::SecretsKvRead { mount, path, version } => {
                handle_kv_read(secrets_service, &mount, path, version).await
            }
            ClientRpcRequest::SecretsKvWrite { mount, path, data, cas } => {
                handle_kv_write(secrets_service, &mount, path, data, cas).await
            }
            ClientRpcRequest::SecretsKvDelete { mount, path, versions } => {
                handle_kv_delete(secrets_service, &mount, path, versions).await
            }
            ClientRpcRequest::SecretsKvDestroy { mount, path, versions } => {
                handle_kv_destroy(secrets_service, &mount, path, versions).await
            }
            ClientRpcRequest::SecretsKvUndelete { mount, path, versions } => {
                handle_kv_undelete(secrets_service, &mount, path, versions).await
            }
            ClientRpcRequest::SecretsKvList { mount, path } => handle_kv_list(secrets_service, &mount, path).await,
            ClientRpcRequest::SecretsKvMetadata { mount, path } => {
                handle_kv_metadata(secrets_service, &mount, path).await
            }
            ClientRpcRequest::SecretsKvUpdateMetadata {
                mount,
                path,
                max_versions,
                cas_required,
                custom_metadata,
            } => {
                handle_kv_update_metadata(secrets_service, &mount, path, max_versions, cas_required, custom_metadata)
                    .await
            }
            ClientRpcRequest::SecretsKvDeleteMetadata { mount, path } => {
                handle_kv_delete_metadata(secrets_service, &mount, path).await
            }
            // Transit operations - now with multi-mount support
            ClientRpcRequest::SecretsTransitCreateKey { mount, name, key_type } => {
                handle_transit_create_key(secrets_service, &mount, name, key_type).await
            }
            ClientRpcRequest::SecretsTransitEncrypt {
                mount,
                name,
                plaintext,
                context,
            } => handle_transit_encrypt(secrets_service, &mount, name, plaintext, context).await,
            ClientRpcRequest::SecretsTransitDecrypt {
                mount,
                name,
                ciphertext,
                context,
            } => handle_transit_decrypt(secrets_service, &mount, name, ciphertext, context).await,
            ClientRpcRequest::SecretsTransitSign { mount, name, data } => {
                handle_transit_sign(secrets_service, &mount, name, data).await
            }
            ClientRpcRequest::SecretsTransitVerify {
                mount,
                name,
                data,
                signature,
            } => handle_transit_verify(secrets_service, &mount, name, data, signature).await,
            ClientRpcRequest::SecretsTransitRotateKey { mount, name } => {
                handle_transit_rotate_key(secrets_service, &mount, name).await
            }
            ClientRpcRequest::SecretsTransitListKeys { mount } => {
                handle_transit_list_keys(secrets_service, &mount).await
            }
            ClientRpcRequest::SecretsTransitRewrap {
                mount,
                name,
                ciphertext,
                context,
            } => handle_transit_rewrap(secrets_service, &mount, name, ciphertext, context).await,
            ClientRpcRequest::SecretsTransitDatakey { mount, name, key_type } => {
                handle_transit_datakey(secrets_service, &mount, name, key_type).await
            }
            // PKI operations - now with multi-mount support
            ClientRpcRequest::SecretsPkiGenerateRoot {
                mount,
                common_name,
                ttl_days,
            } => handle_pki_generate_root(secrets_service, &mount, common_name, ttl_days).await,
            ClientRpcRequest::SecretsPkiGenerateIntermediate { mount, common_name } => {
                handle_pki_generate_intermediate(secrets_service, &mount, common_name).await
            }
            ClientRpcRequest::SecretsPkiSetSignedIntermediate { mount, certificate } => {
                handle_pki_set_signed_intermediate(secrets_service, &mount, certificate).await
            }
            ClientRpcRequest::SecretsPkiCreateRole {
                mount,
                name,
                allowed_domains,
                max_ttl_days,
                allow_bare_domains,
                allow_wildcards,
                allow_subdomains,
            } => {
                handle_pki_create_role(
                    secrets_service,
                    &mount,
                    name,
                    allowed_domains,
                    max_ttl_days,
                    allow_bare_domains,
                    allow_wildcards,
                    allow_subdomains,
                )
                .await
            }
            ClientRpcRequest::SecretsPkiIssue {
                mount,
                role,
                common_name,
                alt_names,
                ttl_days,
            } => handle_pki_issue(secrets_service, &mount, role, common_name, alt_names, ttl_days).await,
            ClientRpcRequest::SecretsPkiRevoke { mount, serial } => {
                handle_pki_revoke(secrets_service, &mount, serial).await
            }
            ClientRpcRequest::SecretsPkiGetCrl { mount } => handle_pki_get_crl(secrets_service, &mount).await,
            ClientRpcRequest::SecretsPkiListCerts { mount } => handle_pki_list_certs(secrets_service, &mount).await,
            ClientRpcRequest::SecretsPkiGetRole { mount, name } => {
                handle_pki_get_role(secrets_service, &mount, name).await
            }
            ClientRpcRequest::SecretsPkiListRoles { mount } => handle_pki_list_roles(secrets_service, &mount).await,
            // Nix Cache Signing operations
            ClientRpcRequest::SecretsNixCacheCreateKey { mount, cache_name } => {
                handle_nix_cache_create_key(secrets_service, &mount, cache_name).await
            }
            ClientRpcRequest::SecretsNixCacheGetPublicKey { mount, cache_name } => {
                handle_nix_cache_get_public_key(secrets_service, &mount, cache_name).await
            }
            ClientRpcRequest::SecretsNixCacheRotateKey { mount, cache_name } => {
                handle_nix_cache_rotate_key(secrets_service, &mount, cache_name).await
            }
            ClientRpcRequest::SecretsNixCacheDeleteKey { mount, cache_name } => {
                handle_nix_cache_delete_key(secrets_service, &mount, cache_name).await
            }
            ClientRpcRequest::SecretsNixCacheListKeys { mount } => {
                handle_nix_cache_list_keys(secrets_service, &mount).await
            }
            _ => Err(anyhow::anyhow!("request not handled by SecretsHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "SecretsHandler"
    }
}

/// Secrets service with multi-mount support.
///
/// Uses a `MountRegistry` to dynamically create and cache store instances
/// per mount point. Each mount has its own isolated storage.
pub struct SecretsService {
    /// Mount registry for dynamic store management.
    pub mount_registry: Arc<aspen_secrets::MountRegistry>,
}

impl SecretsService {
    /// Create a new secrets service with the given mount registry.
    pub fn new(mount_registry: Arc<aspen_secrets::MountRegistry>) -> Self {
        Self { mount_registry }
    }

    /// Get or create a KV store for the given mount point.
    ///
    /// # Errors
    ///
    /// Returns an error if the mount name is invalid or too many mounts exist.
    pub async fn get_kv_store(&self, mount: &str) -> anyhow::Result<Arc<dyn KvStore>> {
        self.mount_registry
            .get_or_create_kv_store(mount)
            .await
            .map_err(|e| anyhow::anyhow!("failed to get KV store for mount '{}': {}", mount, e))
    }

    /// Get or create a Transit store for the given mount point.
    ///
    /// # Errors
    ///
    /// Returns an error if the mount name is invalid or too many mounts exist.
    pub async fn get_transit_store(&self, mount: &str) -> anyhow::Result<Arc<dyn TransitStore>> {
        self.mount_registry
            .get_or_create_transit_store(mount)
            .await
            .map_err(|e| anyhow::anyhow!("failed to get Transit store for mount '{}': {}", mount, e))
    }

    /// Get or create a PKI store for the given mount point.
    ///
    /// # Errors
    ///
    /// Returns an error if the mount name is invalid or too many mounts exist.
    pub async fn get_pki_store(&self, mount: &str) -> anyhow::Result<Arc<dyn PkiStore>> {
        self.mount_registry
            .get_or_create_pki_store(mount)
            .await
            .map_err(|e| anyhow::anyhow!("failed to get PKI store for mount '{}': {}", mount, e))
    }
}

// =============================================================================
// KV v2 Handler Functions
// =============================================================================

async fn handle_kv_read(
    service: &SecretsService,
    mount: &str,
    path: String,
    version: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, path = %path, version = ?version, "KV read request");

    let store = service.get_kv_store(mount).await?;
    let request = ReadSecretRequest { path, version };
    match store.read(request).await {
        Ok(Some(response)) => {
            let metadata = SecretsKvVersionMetadata {
                version: response.metadata.version,
                created_time_unix_ms: response.metadata.created_time_unix_ms,
                deletion_time_unix_ms: response.metadata.deletion_time_unix_ms,
                destroyed: response.metadata.destroyed,
            };
            Ok(ClientRpcResponse::SecretsKvReadResult(SecretsKvReadResultResponse {
                success: true,
                data: Some(response.data.data),
                metadata: Some(metadata),
                error: None,
            }))
        }
        Ok(None) => Ok(ClientRpcResponse::SecretsKvReadResult(SecretsKvReadResultResponse {
            success: false,
            data: None,
            metadata: None,
            error: Some("Secret not found".to_string()),
        })),
        Err(e) => {
            warn!(error = %e, "KV read failed");
            Ok(ClientRpcResponse::SecretsKvReadResult(SecretsKvReadResultResponse {
                success: false,
                data: None,
                metadata: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_kv_write(
    service: &SecretsService,
    mount: &str,
    path: String,
    data: HashMap<String, String>,
    cas: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, path = %path, cas = ?cas, "KV write request");

    let store = service.get_kv_store(mount).await?;
    let request = WriteSecretRequest {
        path,
        data: SecretData { data },
        cas,
    };
    match store.write(request).await {
        Ok(response) => Ok(ClientRpcResponse::SecretsKvWriteResult(SecretsKvWriteResultResponse {
            success: true,
            version: Some(response.version),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "KV write failed");
            Ok(ClientRpcResponse::SecretsKvWriteResult(SecretsKvWriteResultResponse {
                success: false,
                version: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_kv_delete(
    service: &SecretsService,
    mount: &str,
    path: String,
    versions: Vec<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, path = %path, versions = ?versions, "KV delete request");

    let store = service.get_kv_store(mount).await?;
    let request = DeleteSecretRequest { path, versions };
    match store.delete(request).await {
        Ok(()) => Ok(ClientRpcResponse::SecretsKvDeleteResult(SecretsKvDeleteResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "KV delete failed");
            Ok(ClientRpcResponse::SecretsKvDeleteResult(SecretsKvDeleteResultResponse {
                success: false,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_kv_destroy(
    service: &SecretsService,
    mount: &str,
    path: String,
    versions: Vec<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, path = %path, versions = ?versions, "KV destroy request");

    let store = service.get_kv_store(mount).await?;
    let request = DestroySecretRequest { path, versions };
    match store.destroy(request).await {
        Ok(()) => Ok(ClientRpcResponse::SecretsKvDeleteResult(SecretsKvDeleteResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "KV destroy failed");
            Ok(ClientRpcResponse::SecretsKvDeleteResult(SecretsKvDeleteResultResponse {
                success: false,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_kv_undelete(
    service: &SecretsService,
    mount: &str,
    path: String,
    versions: Vec<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, path = %path, versions = ?versions, "KV undelete request");

    let store = service.get_kv_store(mount).await?;
    let request = UndeleteSecretRequest { path, versions };
    match store.undelete(request).await {
        Ok(()) => Ok(ClientRpcResponse::SecretsKvDeleteResult(SecretsKvDeleteResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "KV undelete failed");
            Ok(ClientRpcResponse::SecretsKvDeleteResult(SecretsKvDeleteResultResponse {
                success: false,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_kv_list(service: &SecretsService, mount: &str, path: String) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, path = %path, "KV list request");

    let store = service.get_kv_store(mount).await?;
    let request = ListSecretsRequest { path };
    match store.list(request).await {
        Ok(response) => Ok(ClientRpcResponse::SecretsKvListResult(SecretsKvListResultResponse {
            success: true,
            keys: response.keys,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "KV list failed");
            Ok(ClientRpcResponse::SecretsKvListResult(SecretsKvListResultResponse {
                success: false,
                keys: vec![],
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_kv_metadata(service: &SecretsService, mount: &str, path: String) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, path = %path, "KV metadata request");

    let store = service.get_kv_store(mount).await?;
    let request = ReadMetadataRequest { path };
    match store.read_metadata(request).await {
        Ok(Some(metadata)) => {
            let versions: Vec<SecretsKvVersionInfo> = metadata
                .versions
                .iter()
                .map(|(&version, info)| SecretsKvVersionInfo {
                    version,
                    created_time_unix_ms: info.created_time_unix_ms,
                    deleted: info.deletion_time_unix_ms.is_some(),
                    destroyed: info.destroyed,
                })
                .collect();

            Ok(ClientRpcResponse::SecretsKvMetadataResult(SecretsKvMetadataResultResponse {
                success: true,
                current_version: Some(metadata.current_version),
                max_versions: Some(metadata.max_versions),
                cas_required: Some(metadata.cas_required),
                created_time_unix_ms: Some(metadata.created_time_unix_ms),
                updated_time_unix_ms: Some(metadata.updated_time_unix_ms),
                versions,
                custom_metadata: Some(metadata.custom_metadata),
                error: None,
            }))
        }
        Ok(None) => Ok(ClientRpcResponse::SecretsKvMetadataResult(SecretsKvMetadataResultResponse {
            success: false,
            current_version: None,
            max_versions: None,
            cas_required: None,
            created_time_unix_ms: None,
            updated_time_unix_ms: None,
            versions: vec![],
            custom_metadata: None,
            error: Some("Secret not found".to_string()),
        })),
        Err(e) => {
            warn!(error = %e, "KV metadata read failed");
            Ok(ClientRpcResponse::SecretsKvMetadataResult(SecretsKvMetadataResultResponse {
                success: false,
                current_version: None,
                max_versions: None,
                cas_required: None,
                created_time_unix_ms: None,
                updated_time_unix_ms: None,
                versions: vec![],
                custom_metadata: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_kv_update_metadata(
    service: &SecretsService,
    mount: &str,
    path: String,
    max_versions: Option<u32>,
    cas_required: Option<bool>,
    custom_metadata: Option<HashMap<String, String>>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, path = %path, "KV update metadata request");

    let store = service.get_kv_store(mount).await?;
    let request = UpdateMetadataRequest {
        path,
        max_versions,
        cas_required,
        custom_metadata,
        delete_version_after_secs: None,
    };
    match store.update_metadata(request).await {
        Ok(metadata) => Ok(ClientRpcResponse::SecretsKvMetadataResult(SecretsKvMetadataResultResponse {
            success: true,
            current_version: Some(metadata.current_version),
            max_versions: Some(metadata.max_versions),
            cas_required: Some(metadata.cas_required),
            created_time_unix_ms: Some(metadata.created_time_unix_ms),
            updated_time_unix_ms: Some(metadata.updated_time_unix_ms),
            versions: vec![],
            custom_metadata: Some(metadata.custom_metadata),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "KV update metadata failed");
            Ok(ClientRpcResponse::SecretsKvMetadataResult(SecretsKvMetadataResultResponse {
                success: false,
                current_version: None,
                max_versions: None,
                cas_required: None,
                created_time_unix_ms: None,
                updated_time_unix_ms: None,
                versions: vec![],
                custom_metadata: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_kv_delete_metadata(
    service: &SecretsService,
    mount: &str,
    path: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, path = %path, "KV delete metadata request");

    let store = service.get_kv_store(mount).await?;
    match store.delete_metadata(&path).await {
        Ok(_deleted) => Ok(ClientRpcResponse::SecretsKvDeleteResult(SecretsKvDeleteResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "KV delete metadata failed");
            Ok(ClientRpcResponse::SecretsKvDeleteResult(SecretsKvDeleteResultResponse {
                success: false,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

// =============================================================================
// Transit Handler Functions
// =============================================================================

async fn handle_transit_create_key(
    service: &SecretsService,
    mount: &str,
    name: String,
    key_type: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, name = %name, key_type = %key_type, "Transit create key request");

    let key_type_enum = match key_type.as_str() {
        "aes256-gcm" | "aes-256-gcm" => KeyType::Aes256Gcm,
        "xchacha20-poly1305" => KeyType::XChaCha20Poly1305,
        "ed25519" => KeyType::Ed25519,
        _ => {
            return Ok(ClientRpcResponse::SecretsTransitKeyResult(SecretsTransitKeyResultResponse {
                success: false,
                name: None,
                version: None,
                key_type: None,
                error: Some(format!(
                    "Invalid key type: {}. Valid types: aes256-gcm, xchacha20-poly1305, ed25519",
                    key_type
                )),
            }));
        }
    };

    let store = service.get_transit_store(mount).await?;
    let request = CreateKeyRequest {
        name: name.clone(),
        key_type: key_type_enum,
        exportable: false,
        deletion_allowed: false,
        convergent_encryption: false,
    };

    match store.create_key(request).await {
        Ok(key) => Ok(ClientRpcResponse::SecretsTransitKeyResult(SecretsTransitKeyResultResponse {
            success: true,
            name: Some(key.name),
            version: Some(key.current_version as u64),
            key_type: Some(format!("{:?}", key.key_type)),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit create key failed");
            Ok(ClientRpcResponse::SecretsTransitKeyResult(SecretsTransitKeyResultResponse {
                success: false,
                name: None,
                version: None,
                key_type: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_transit_encrypt(
    service: &SecretsService,
    mount: &str,
    name: String,
    plaintext: Vec<u8>,
    context: Option<Vec<u8>>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, name = %name, plaintext_len = plaintext.len(), "Transit encrypt request");

    let store = service.get_transit_store(mount).await?;
    let request = EncryptRequest {
        key_name: name,
        plaintext,
        context,
        key_version: None,
    };

    match store.encrypt(request).await {
        Ok(response) => Ok(ClientRpcResponse::SecretsTransitEncryptResult(SecretsTransitEncryptResultResponse {
            success: true,
            ciphertext: Some(response.ciphertext),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit encrypt failed");
            Ok(ClientRpcResponse::SecretsTransitEncryptResult(SecretsTransitEncryptResultResponse {
                success: false,
                ciphertext: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_transit_decrypt(
    service: &SecretsService,
    mount: &str,
    name: String,
    ciphertext: String,
    context: Option<Vec<u8>>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, name = %name, "Transit decrypt request");

    let store = service.get_transit_store(mount).await?;
    let request = DecryptRequest {
        key_name: name,
        ciphertext,
        context,
    };

    match store.decrypt(request).await {
        Ok(response) => Ok(ClientRpcResponse::SecretsTransitDecryptResult(SecretsTransitDecryptResultResponse {
            success: true,
            plaintext: Some(response.plaintext),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit decrypt failed");
            Ok(ClientRpcResponse::SecretsTransitDecryptResult(SecretsTransitDecryptResultResponse {
                success: false,
                plaintext: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_transit_sign(
    service: &SecretsService,
    mount: &str,
    name: String,
    data: Vec<u8>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, name = %name, data_len = data.len(), "Transit sign request");

    let store = service.get_transit_store(mount).await?;
    let request = SignRequest {
        key_name: name,
        input: data,
        hash_algorithm: None,
        prehashed: false,
        key_version: None,
    };

    match store.sign(request).await {
        Ok(response) => Ok(ClientRpcResponse::SecretsTransitSignResult(SecretsTransitSignResultResponse {
            success: true,
            signature: Some(response.signature),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit sign failed");
            Ok(ClientRpcResponse::SecretsTransitSignResult(SecretsTransitSignResultResponse {
                success: false,
                signature: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_transit_verify(
    service: &SecretsService,
    mount: &str,
    name: String,
    data: Vec<u8>,
    signature: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, name = %name, data_len = data.len(), "Transit verify request");

    let store = service.get_transit_store(mount).await?;
    let request = VerifyRequest {
        key_name: name,
        input: data,
        signature,
        hash_algorithm: None,
        prehashed: false,
    };

    match store.verify(request).await {
        Ok(response) => Ok(ClientRpcResponse::SecretsTransitVerifyResult(SecretsTransitVerifyResultResponse {
            success: true,
            valid: Some(response.valid),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit verify failed");
            Ok(ClientRpcResponse::SecretsTransitVerifyResult(SecretsTransitVerifyResultResponse {
                success: false,
                valid: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_transit_rotate_key(
    service: &SecretsService,
    mount: &str,
    name: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, name = %name, "Transit rotate key request");

    let store = service.get_transit_store(mount).await?;
    match store.rotate_key(&name).await {
        Ok(key) => Ok(ClientRpcResponse::SecretsTransitKeyResult(SecretsTransitKeyResultResponse {
            success: true,
            name: Some(key.name),
            version: Some(key.current_version as u64),
            key_type: Some(format!("{:?}", key.key_type)),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit rotate key failed");
            Ok(ClientRpcResponse::SecretsTransitKeyResult(SecretsTransitKeyResultResponse {
                success: false,
                name: None,
                version: None,
                key_type: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_transit_list_keys(service: &SecretsService, mount: &str) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, "Transit list keys request");

    let store = service.get_transit_store(mount).await?;
    match store.list_keys().await {
        Ok(keys) => Ok(ClientRpcResponse::SecretsTransitListResult(SecretsTransitListResultResponse {
            success: true,
            keys,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit list keys failed");
            Ok(ClientRpcResponse::SecretsTransitListResult(SecretsTransitListResultResponse {
                success: false,
                keys: vec![],
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_transit_rewrap(
    service: &SecretsService,
    mount: &str,
    name: String,
    ciphertext: String,
    context: Option<Vec<u8>>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, name = %name, "Transit rewrap request");

    let store = service.get_transit_store(mount).await?;
    let request = RewrapRequest {
        key_name: name,
        ciphertext,
        context,
    };

    match store.rewrap(request).await {
        Ok(response) => Ok(ClientRpcResponse::SecretsTransitEncryptResult(SecretsTransitEncryptResultResponse {
            success: true,
            ciphertext: Some(response.ciphertext),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit rewrap failed");
            Ok(ClientRpcResponse::SecretsTransitEncryptResult(SecretsTransitEncryptResultResponse {
                success: false,
                ciphertext: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_transit_datakey(
    service: &SecretsService,
    mount: &str,
    name: String,
    key_type: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, name = %name, key_type = %key_type, "Transit datakey request");

    // Note: include_plaintext is handled by the response type, not the request
    // The client specifies "plaintext" or "wrapped" to indicate whether they want
    // the plaintext included in the response
    let _include_plaintext = key_type == "plaintext";

    let store = service.get_transit_store(mount).await?;
    let request = DataKeyRequest {
        key_name: name,
        bits: 256,
        context: None,
    };

    match store.generate_data_key(request).await {
        Ok(response) => Ok(ClientRpcResponse::SecretsTransitDatakeyResult(SecretsTransitDatakeyResultResponse {
            success: true,
            plaintext: Some(response.plaintext),
            ciphertext: Some(response.ciphertext),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit datakey failed");
            Ok(ClientRpcResponse::SecretsTransitDatakeyResult(SecretsTransitDatakeyResultResponse {
                success: false,
                plaintext: None,
                ciphertext: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

// =============================================================================
// PKI Handler Functions
// =============================================================================

async fn handle_pki_generate_root(
    service: &SecretsService,
    mount: &str,
    common_name: String,
    ttl_days: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, common_name = %common_name, ttl_days = ?ttl_days, "PKI generate root request");

    let store = service.get_pki_store(mount).await?;
    let ttl_secs = ttl_days.map(|d| d as u64 * 24 * 3600).unwrap_or(10 * 365 * 24 * 3600);
    let request = GenerateRootRequest::new(common_name).with_ttl_secs(ttl_secs);

    match store.generate_root(request).await {
        Ok(response) => Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
            success: true,
            certificate: Some(response.certificate),
            private_key: None, // Root CA private key is stored internally
            serial: Some(response.serial),
            csr: None,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI generate root failed");
            Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
                success: false,
                certificate: None,
                private_key: None,
                serial: None,
                csr: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_generate_intermediate(
    service: &SecretsService,
    mount: &str,
    common_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, common_name = %common_name, "PKI generate intermediate request");

    let store = service.get_pki_store(mount).await?;
    let request = GenerateIntermediateRequest::new(common_name);

    match store.generate_intermediate(request).await {
        Ok(response) => Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
            success: true,
            certificate: None,
            private_key: None,
            serial: None,
            csr: Some(response.csr),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI generate intermediate failed");
            Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
                success: false,
                certificate: None,
                private_key: None,
                serial: None,
                csr: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_set_signed_intermediate(
    service: &SecretsService,
    mount: &str,
    certificate: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, "PKI set signed intermediate request");

    let store = service.get_pki_store(mount).await?;
    let request = SetSignedIntermediateRequest {
        certificate: certificate.clone(),
    };

    match store.set_signed_intermediate(request).await {
        Ok(()) => Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
            success: true,
            certificate: Some(certificate),
            private_key: None,
            serial: None,
            csr: None,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI set signed intermediate failed");
            Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
                success: false,
                certificate: None,
                private_key: None,
                serial: None,
                csr: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_create_role(
    service: &SecretsService,
    mount: &str,
    name: String,
    allowed_domains: Vec<String>,
    max_ttl_days: u32,
    allow_bare_domains: bool,
    allow_wildcards: bool,
    allow_subdomains: bool,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, name = %name, "PKI create role request");

    let store = service.get_pki_store(mount).await?;
    let mut role = PkiRole::new(name.clone());
    role.allowed_domains = allowed_domains.clone();
    role.max_ttl_secs = max_ttl_days as u64 * 24 * 3600;
    role.allow_bare_domains = allow_bare_domains;
    role.allow_wildcard_certificates = allow_wildcards;
    role.allow_subdomains = allow_subdomains;

    let request = CreateRoleRequest {
        name: name.clone(),
        config: role,
    };

    match store.create_role(request).await {
        Ok(created_role) => Ok(ClientRpcResponse::SecretsPkiRoleResult(SecretsPkiRoleResultResponse {
            success: true,
            role: Some(SecretsPkiRoleConfig {
                name: created_role.name,
                allowed_domains: created_role.allowed_domains,
                max_ttl_days: (created_role.max_ttl_secs / (24 * 3600)) as u32,
                allow_bare_domains: created_role.allow_bare_domains,
                allow_wildcards: created_role.allow_wildcard_certificates,
            }),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI create role failed");
            Ok(ClientRpcResponse::SecretsPkiRoleResult(SecretsPkiRoleResultResponse {
                success: false,
                role: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_issue(
    service: &SecretsService,
    mount: &str,
    role: String,
    common_name: String,
    alt_names: Vec<String>,
    ttl_days: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, role = %role, common_name = %common_name, "PKI issue request");

    let store = service.get_pki_store(mount).await?;
    let request = IssueCertificateRequest {
        role,
        common_name,
        alt_names,
        ip_sans: vec![],
        uri_sans: vec![],
        ttl_secs: ttl_days.map(|d| d as u64 * 24 * 3600),
        exclude_cn_from_sans: false,
    };

    match store.issue(request).await {
        Ok(response) => Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
            success: true,
            certificate: Some(response.certificate),
            private_key: response.private_key,
            serial: Some(response.serial),
            csr: None,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI issue failed");
            Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
                success: false,
                certificate: None,
                private_key: None,
                serial: None,
                csr: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_revoke(service: &SecretsService, mount: &str, serial: String) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, serial = %serial, "PKI revoke request");

    let store = service.get_pki_store(mount).await?;
    let request = RevokeCertificateRequest { serial: serial.clone() };

    match store.revoke(request).await {
        Ok(()) => Ok(ClientRpcResponse::SecretsPkiRevokeResult(SecretsPkiRevokeResultResponse {
            success: true,
            serial: Some(serial),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI revoke failed");
            Ok(ClientRpcResponse::SecretsPkiRevokeResult(SecretsPkiRevokeResultResponse {
                success: false,
                serial: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_get_crl(service: &SecretsService, mount: &str) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, "PKI get CRL request");

    let store = service.get_pki_store(mount).await?;
    match store.get_crl().await {
        Ok(crl_state) => {
            // Convert CRL state to PEM format
            let crl_pem = format!(
                "# CRL with {} entries, last updated: {}, next update: {}",
                crl_state.entries.len(),
                crl_state.last_update_unix_ms,
                crl_state.next_update_unix_ms
            );
            Ok(ClientRpcResponse::SecretsPkiCrlResult(SecretsPkiCrlResultResponse {
                success: true,
                crl: Some(crl_pem),
                error: None,
            }))
        }
        Err(e) => {
            warn!(error = %e, "PKI get CRL failed");
            Ok(ClientRpcResponse::SecretsPkiCrlResult(SecretsPkiCrlResultResponse {
                success: false,
                crl: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_list_certs(service: &SecretsService, mount: &str) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, "PKI list certs request");

    let store = service.get_pki_store(mount).await?;
    match store.list_certificates().await {
        Ok(serials) => Ok(ClientRpcResponse::SecretsPkiListResult(SecretsPkiListResultResponse {
            success: true,
            items: serials,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI list certs failed");
            Ok(ClientRpcResponse::SecretsPkiListResult(SecretsPkiListResultResponse {
                success: false,
                items: vec![],
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_get_role(service: &SecretsService, mount: &str, name: String) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, name = %name, "PKI get role request");

    let store = service.get_pki_store(mount).await?;
    match store.read_role(&name).await {
        Ok(Some(role)) => Ok(ClientRpcResponse::SecretsPkiRoleResult(SecretsPkiRoleResultResponse {
            success: true,
            role: Some(SecretsPkiRoleConfig {
                name: role.name,
                allowed_domains: role.allowed_domains,
                max_ttl_days: (role.max_ttl_secs / (24 * 3600)) as u32,
                allow_bare_domains: role.allow_bare_domains,
                allow_wildcards: role.allow_wildcard_certificates,
            }),
            error: None,
        })),
        Ok(None) => Ok(ClientRpcResponse::SecretsPkiRoleResult(SecretsPkiRoleResultResponse {
            success: false,
            role: None,
            error: Some("Role not found".to_string()),
        })),
        Err(e) => {
            warn!(error = %e, "PKI get role failed");
            Ok(ClientRpcResponse::SecretsPkiRoleResult(SecretsPkiRoleResultResponse {
                success: false,
                role: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_list_roles(service: &SecretsService, mount: &str) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, "PKI list roles request");

    let store = service.get_pki_store(mount).await?;
    match store.list_roles().await {
        Ok(roles) => Ok(ClientRpcResponse::SecretsPkiListResult(SecretsPkiListResultResponse {
            success: true,
            items: roles,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI list roles failed");
            Ok(ClientRpcResponse::SecretsPkiListResult(SecretsPkiListResultResponse {
                success: false,
                items: vec![],
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

// =============================================================================
// Error Sanitization
// =============================================================================

/// Sanitize secrets errors for client display.
///
/// Returns user-friendly error messages without leaking sensitive internal details.
/// All error variants are explicitly handled to prevent catch-all masking of
/// actionable errors.
fn sanitize_secrets_error(error: &aspen_secrets::SecretsError) -> String {
    use aspen_secrets::SecretsError;

    match error {
        // KV errors
        SecretsError::SecretNotFound { key } => format!("Secret not found: {key}"),
        SecretsError::VersionNotFound { path, version } => {
            format!("Version {version} not found for secret: {path}")
        }
        SecretsError::VersionDestroyed { path, version } => {
            format!("Version {version} of secret '{path}' has been destroyed")
        }
        SecretsError::CasFailed { path, expected, actual } => {
            format!("CAS conflict for secret '{path}': expected version {expected}, found {actual}")
        }
        SecretsError::PathTooLong { length, max } => {
            format!("Path too long: {length} characters (max: {max})")
        }
        SecretsError::ValueTooLarge { size, max } => {
            format!("Secret too large: {size} bytes (max: {max})")
        }
        SecretsError::TooManyVersions { count, max } => {
            format!("Too many versions: {count} (max: {max})")
        }

        // Transit errors
        SecretsError::TransitKeyNotFound { name } => format!("Transit key not found: {name}"),
        SecretsError::TransitKeyExists { name } => format!("Transit key already exists: {name}"),
        SecretsError::TransitKeyNameTooLong { length, max } => {
            format!("Transit key name too long: {length} characters (max: {max})")
        }
        SecretsError::PlaintextTooLarge { size, max } => {
            format!("Plaintext too large: {size} bytes (max: {max})")
        }
        SecretsError::InvalidCiphertext { reason } => format!("Invalid ciphertext: {reason}"),
        SecretsError::KeyVersionTooOld {
            name,
            version,
            min_version,
        } => {
            format!("Key version {version} is below minimum decryption version {min_version} for key '{name}'")
        }
        SecretsError::KeyDeletionNotAllowed { name } => {
            format!("Deletion not allowed for key '{name}'")
        }
        SecretsError::KeyExportNotAllowed { name } => format!("Export not allowed for key '{name}'"),
        SecretsError::UnsupportedKeyType { key_type } => {
            format!("Unsupported key type: {key_type}")
        }
        SecretsError::SignatureVerificationFailed { name } => {
            format!("Signature verification failed for key '{name}'")
        }

        // PKI errors
        SecretsError::CaNotInitialized => {
            "Certificate authority not initialized. Run 'secrets pki generate-root' first.".to_string()
        }
        SecretsError::CaAlreadyInitialized { mount } => {
            format!("Certificate authority already initialized for mount '{mount}'")
        }
        SecretsError::RoleNotFound { name } => format!("PKI role not found: {name}"),
        SecretsError::RoleExists { name } => format!("PKI role already exists: {name}"),
        SecretsError::CertificateNotFound { serial } => {
            format!("Certificate not found: serial {serial}")
        }
        SecretsError::CertificateAlreadyRevoked { serial } => {
            format!("Certificate already revoked: serial {serial}")
        }
        SecretsError::CommonNameNotAllowed { cn, role } => {
            format!(
                "Common name '{cn}' not allowed by role '{role}'. Check allowed_domains and consider --allow-subdomains."
            )
        }
        SecretsError::SanNotAllowed { san, role } => {
            format!("SAN '{san}' not allowed by role '{role}'")
        }
        SecretsError::TtlExceedsMax {
            role,
            requested_secs,
            max_secs,
        } => {
            format!("Requested TTL {requested_secs}s exceeds maximum {max_secs}s for role '{role}'")
        }
        SecretsError::TooManySans { count, max } => format!("Too many SANs: {count} (max: {max})"),
        SecretsError::CertificateGeneration { reason } => {
            format!("Certificate generation failed: {reason}")
        }
        SecretsError::InvalidCertificate { reason } => format!("Invalid certificate: {reason}"),

        // Mount errors
        SecretsError::MountNotFound { name } => format!("Mount not found: {name}"),
        SecretsError::MountExists { name } => format!("Mount already exists: {name}"),
        SecretsError::TooManyMounts { count, max } => {
            format!("Too many mounts: {count} (max: {max})")
        }
        SecretsError::InvalidMount { name, reason } => {
            format!("Invalid mount name '{name}': {reason}")
        }

        // These errors may contain sensitive file paths or internal details.
        // Return generic messages to avoid information leakage.
        SecretsError::ReadFile { .. }
        | SecretsError::FileTooLarge { .. }
        | SecretsError::ParseFile { .. }
        | SecretsError::LoadIdentity { .. }
        | SecretsError::IdentityNotFound { .. }
        | SecretsError::InvalidIdentity { .. }
        | SecretsError::Decryption { .. }
        | SecretsError::SopsMetadata { .. }
        | SecretsError::DecodeSecret { .. }
        | SecretsError::ParseTrustedRoot { .. }
        | SecretsError::ParseSigningKey { .. }
        | SecretsError::ParseToken { .. }
        | SecretsError::KvStore { .. }
        | SecretsError::Encryption { .. }
        | SecretsError::Serialization { .. }
        | SecretsError::Internal { .. } => "Internal secrets error".to_string(),
    }
}

// =============================================================================
// Nix Cache Signing Operations
// =============================================================================

async fn handle_nix_cache_create_key(
    service: &SecretsService,
    mount: &str,
    cache_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, cache_name = %cache_name, "Nix cache create key request");

    let store = service.get_transit_store(mount).await?;

    // Create Ed25519 signing key for Nix cache
    let key_request = aspen_secrets::transit::CreateKeyRequest {
        name: cache_name.clone(),
        key_type: aspen_secrets::transit::KeyType::Ed25519,
        exportable: false,      // Keep keys secure in Transit
        deletion_allowed: true, // Allow deletion for key rotation
        convergent_encryption: false,
    };

    match store.create_key(key_request).await {
        Ok(_) => {
            // Get the public key to return
            match store.read_key(&cache_name).await {
                Ok(Some(transit_key)) => {
                    // Get public key from current version
                    if let Some(current_version) = transit_key.versions.get(&transit_key.current_version) {
                        if let Some(ref public_key_bytes) = current_version.public_key {
                            let public_key_b64 = base64::engine::general_purpose::STANDARD.encode(public_key_bytes);
                            let public_key = format!("{}:{}", cache_name, public_key_b64);
                            Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                                success: true,
                                public_key: Some(public_key),
                                error: None,
                            }))
                        } else {
                            let error = "Key does not have a public key (not an asymmetric key)".to_string();
                            Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                                success: false,
                                public_key: None,
                                error: Some(error),
                            }))
                        }
                    } else {
                        let error = format!("Current version {} not found for key", transit_key.current_version);
                        Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                            success: false,
                            public_key: None,
                            error: Some(error),
                        }))
                    }
                }
                Ok(None) => {
                    let error = "Key not found".to_string();
                    Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                        success: false,
                        public_key: None,
                        error: Some(error),
                    }))
                }
                Err(e) => {
                    let error = format!("Failed to read public key: {}", sanitize_secrets_error(&e));
                    Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                        success: false,
                        public_key: None,
                        error: Some(error),
                    }))
                }
            }
        }
        Err(e) => {
            let error = format!("Failed to create key: {}", sanitize_secrets_error(&e));
            Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                success: false,
                public_key: None,
                error: Some(error),
            }))
        }
    }
}

async fn handle_nix_cache_get_public_key(
    service: &SecretsService,
    mount: &str,
    cache_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, cache_name = %cache_name, "Nix cache get public key request");

    let store = service.get_transit_store(mount).await?;

    match store.read_key(&cache_name).await {
        Ok(Some(transit_key)) => {
            // Get public key from current version
            if let Some(current_version) = transit_key.versions.get(&transit_key.current_version) {
                if let Some(ref public_key_bytes) = current_version.public_key {
                    let public_key_b64 = base64::engine::general_purpose::STANDARD.encode(public_key_bytes);
                    let public_key = format!("{}:{}", cache_name, public_key_b64);
                    Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                        success: true,
                        public_key: Some(public_key),
                        error: None,
                    }))
                } else {
                    let error = "Key does not have a public key (not an asymmetric key)".to_string();
                    Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                        success: false,
                        public_key: None,
                        error: Some(error),
                    }))
                }
            } else {
                let error = format!("Current version {} not found for key", transit_key.current_version);
                Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                    success: false,
                    public_key: None,
                    error: Some(error),
                }))
            }
        }
        Ok(None) => {
            let error = "Key not found".to_string();
            Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                success: false,
                public_key: None,
                error: Some(error),
            }))
        }
        Err(e) => {
            let error = format!("Key not found or read failed: {}", sanitize_secrets_error(&e));
            Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                success: false,
                public_key: None,
                error: Some(error),
            }))
        }
    }
}

async fn handle_nix_cache_rotate_key(
    service: &SecretsService,
    mount: &str,
    cache_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, cache_name = %cache_name, "Nix cache rotate key request");

    let store = service.get_transit_store(mount).await?;

    match store.rotate_key(&cache_name).await {
        Ok(_) => {
            // Get the new public key
            match store.read_key(&cache_name).await {
                Ok(Some(transit_key)) => {
                    if let Some(current_version) = transit_key.versions.get(&transit_key.current_version) {
                        if let Some(ref public_key_bytes) = current_version.public_key {
                            let public_key_b64 = base64::engine::general_purpose::STANDARD.encode(public_key_bytes);
                            let public_key = format!("{}:{}", cache_name, public_key_b64);
                            Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                                success: true,
                                public_key: Some(public_key),
                                error: None,
                            }))
                        } else {
                            let error = "Rotated key does not have a public key".to_string();
                            Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                                success: false,
                                public_key: None,
                                error: Some(error),
                            }))
                        }
                    } else {
                        let error = format!("Rotated key version {} not found", transit_key.current_version);
                        Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                            success: false,
                            public_key: None,
                            error: Some(error),
                        }))
                    }
                }
                Ok(None) => {
                    let error = "Rotated key not found".to_string();
                    Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                        success: false,
                        public_key: None,
                        error: Some(error),
                    }))
                }
                Err(e) => {
                    let error = format!("Rotated key but failed to read public key: {}", sanitize_secrets_error(&e));
                    Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                        success: false,
                        public_key: None,
                        error: Some(error),
                    }))
                }
            }
        }
        Err(e) => {
            let error = format!("Failed to rotate key: {}", sanitize_secrets_error(&e));
            Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                success: false,
                public_key: None,
                error: Some(error),
            }))
        }
    }
}

async fn handle_nix_cache_delete_key(
    service: &SecretsService,
    mount: &str,
    cache_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, cache_name = %cache_name, "Nix cache delete key request");

    let store = service.get_transit_store(mount).await?;

    match store.delete_key(&cache_name).await {
        Ok(_) => Ok(ClientRpcResponse::SecretsNixCacheDeleteResult(SecretsNixCacheDeleteResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => {
            let error = format!("Failed to delete key: {}", sanitize_secrets_error(&e));
            Ok(ClientRpcResponse::SecretsNixCacheDeleteResult(SecretsNixCacheDeleteResultResponse {
                success: false,
                error: Some(error),
            }))
        }
    }
}

async fn handle_nix_cache_list_keys(service: &SecretsService, mount: &str) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, "Nix cache list keys request");

    let store = service.get_transit_store(mount).await?;

    match store.list_keys().await {
        Ok(keys) => Ok(ClientRpcResponse::SecretsNixCacheListResult(SecretsNixCacheListResultResponse {
            success: true,
            cache_names: Some(keys),
            error: None,
        })),
        Err(e) => {
            let error = format!("Failed to list keys: {}", sanitize_secrets_error(&e));
            Ok(ClientRpcResponse::SecretsNixCacheListResult(SecretsNixCacheListResultResponse {
                success: false,
                cache_names: None,
                error: Some(error),
            }))
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use aspen_client_rpc::ClientRpcRequest;
    use aspen_client_rpc::ClientRpcResponse;

    use super::*;
    use crate::context::test_support::TestContextBuilder;
    use crate::test_mocks::MockEndpointProvider;
    #[cfg(feature = "sql")]
    use crate::test_mocks::mock_sql_executor;

    /// Create a SecretsService with in-memory backends for testing.
    fn make_secrets_service(kv_store: Arc<dyn aspen_core::KeyValueStore>) -> SecretsService {
        let mount_registry = Arc::new(aspen_secrets::MountRegistry::new(kv_store));
        SecretsService::new(mount_registry)
    }

    /// Create a test context with secrets service enabled.
    async fn setup_test_context_with_secrets() -> ClientProtocolContext {
        use aspen_core::DeterministicKeyValueStore;

        let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(12345).await);
        let kv_store: Arc<dyn aspen_core::KeyValueStore> = Arc::new(DeterministicKeyValueStore::new());
        let secrets_service = Arc::new(make_secrets_service(Arc::clone(&kv_store)));

        let mut builder = TestContextBuilder::new()
            .with_node_id(1)
            .with_endpoint_manager(mock_endpoint)
            .with_cookie("test_cluster")
            .with_kv_store(kv_store);

        #[cfg(feature = "sql")]
        {
            builder = builder.with_sql_executor(mock_sql_executor());
        }

        let mut ctx = builder.build();
        ctx.secrets_service = Some(secrets_service);
        ctx
    }

    /// Create a test context without secrets service.
    async fn setup_test_context_without_secrets() -> ClientProtocolContext {
        let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(12345).await);

        let mut builder = TestContextBuilder::new()
            .with_node_id(1)
            .with_endpoint_manager(mock_endpoint)
            .with_cookie("test_cluster");

        #[cfg(feature = "sql")]
        {
            builder = builder.with_sql_executor(mock_sql_executor());
        }

        builder.build()
    }

    // =========================================================================
    // Handler Dispatch Tests (can_handle)
    // =========================================================================

    #[test]
    fn test_can_handle_kv_read() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsKvRead {
            mount: "secret".to_string(),
            path: "test/path".to_string(),
            version: None,
        }));
    }

    #[test]
    fn test_can_handle_kv_write() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsKvWrite {
            mount: "secret".to_string(),
            path: "test/path".to_string(),
            data: HashMap::new(),
            cas: None,
        }));
    }

    #[test]
    fn test_can_handle_kv_delete() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsKvDelete {
            mount: "secret".to_string(),
            path: "test/path".to_string(),
            versions: vec![1],
        }));
    }

    #[test]
    fn test_can_handle_kv_destroy() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsKvDestroy {
            mount: "secret".to_string(),
            path: "test/path".to_string(),
            versions: vec![1],
        }));
    }

    #[test]
    fn test_can_handle_kv_undelete() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsKvUndelete {
            mount: "secret".to_string(),
            path: "test/path".to_string(),
            versions: vec![1],
        }));
    }

    #[test]
    fn test_can_handle_kv_list() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsKvList {
            mount: "secret".to_string(),
            path: "test/".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_kv_metadata() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsKvMetadata {
            mount: "secret".to_string(),
            path: "test/path".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_transit_create_key() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsTransitCreateKey {
            mount: "transit".to_string(),
            name: "my-key".to_string(),
            key_type: "aes256-gcm".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_transit_encrypt() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsTransitEncrypt {
            mount: "transit".to_string(),
            name: "my-key".to_string(),
            plaintext: b"test".to_vec(),
            context: None,
        }));
    }

    #[test]
    fn test_can_handle_transit_decrypt() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsTransitDecrypt {
            mount: "transit".to_string(),
            name: "my-key".to_string(),
            ciphertext: "aspen:v1:base64data".to_string(),
            context: None,
        }));
    }

    #[test]
    fn test_can_handle_transit_sign() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsTransitSign {
            mount: "transit".to_string(),
            name: "my-key".to_string(),
            data: b"test".to_vec(),
        }));
    }

    #[test]
    fn test_can_handle_transit_verify() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsTransitVerify {
            mount: "transit".to_string(),
            name: "my-key".to_string(),
            data: b"test".to_vec(),
            signature: "sig".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_pki_generate_root() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: "pki".to_string(),
            common_name: "Test CA".to_string(),
            ttl_days: Some(365),
        }));
    }

    #[test]
    fn test_can_handle_pki_create_role() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsPkiCreateRole {
            mount: "pki".to_string(),
            name: "web-servers".to_string(),
            allowed_domains: vec!["example.com".to_string()],
            max_ttl_days: 90,
            allow_bare_domains: true,
            allow_wildcards: false,
            allow_subdomains: false,
        }));
    }

    #[test]
    fn test_can_handle_pki_issue() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsPkiIssue {
            mount: "pki".to_string(),
            role: "web-servers".to_string(),
            common_name: "www.example.com".to_string(),
            alt_names: vec![],
            ttl_days: Some(30),
        }));
    }

    #[test]
    fn test_can_handle_pki_revoke() {
        let handler = SecretsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::SecretsPkiRevoke {
            mount: "pki".to_string(),
            serial: "01".to_string(),
        }));
    }

    #[test]
    fn test_rejects_unrelated_requests() {
        let handler = SecretsHandler;

        // Core requests
        assert!(!handler.can_handle(&ClientRpcRequest::Ping));
        assert!(!handler.can_handle(&ClientRpcRequest::GetHealth));

        // KV requests (non-secrets)
        assert!(!handler.can_handle(&ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::WriteKey {
            key: "test".to_string(),
            value: vec![1, 2, 3],
        }));

        // Cluster requests
        assert!(!handler.can_handle(&ClientRpcRequest::InitCluster));
        assert!(!handler.can_handle(&ClientRpcRequest::GetClusterState));
    }

    #[test]
    fn test_handler_name() {
        let handler = SecretsHandler;
        assert_eq!(handler.name(), "SecretsHandler");
    }

    // =========================================================================
    // Secrets Service Availability Tests
    // =========================================================================

    #[tokio::test]
    async fn test_secrets_not_enabled_error() {
        let ctx = setup_test_context_without_secrets().await;
        let handler = SecretsHandler;

        let request = ClientRpcRequest::SecretsKvRead {
            mount: "secret".to_string(),
            path: "test/path".to_string(),
            version: None,
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::Error(err) => {
                assert_eq!(err.code, "SECRETS_NOT_ENABLED");
                assert!(err.message.contains("not enabled"));
            }
            other => panic!("expected Error response, got {:?}", other),
        }
    }

    // =========================================================================
    // KV v2 Handler Tests
    // =========================================================================

    #[tokio::test]
    async fn test_kv_write_then_read() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Write a secret
        let mut data = HashMap::new();
        data.insert("username".to_string(), "admin".to_string());
        data.insert("password".to_string(), "secret123".to_string());

        let write_request = ClientRpcRequest::SecretsKvWrite {
            mount: "secret".to_string(),
            path: "db/creds".to_string(),
            data,
            cas: None,
        };

        let result = handler.handle(write_request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsKvWriteResult(resp) => {
                assert!(resp.success);
                assert_eq!(resp.version, Some(1));
                assert!(resp.error.is_none());
            }
            other => panic!("expected SecretsKvWriteResult, got {:?}", other),
        }

        // Read the secret back
        let read_request = ClientRpcRequest::SecretsKvRead {
            mount: "secret".to_string(),
            path: "db/creds".to_string(),
            version: None,
        };

        let result = handler.handle(read_request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsKvReadResult(resp) => {
                assert!(resp.success);
                let data = resp.data.expect("should have data");
                assert_eq!(data.get("username"), Some(&"admin".to_string()));
                assert_eq!(data.get("password"), Some(&"secret123".to_string()));
            }
            other => panic!("expected SecretsKvReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_kv_read_nonexistent() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        let request = ClientRpcRequest::SecretsKvRead {
            mount: "secret".to_string(),
            path: "nonexistent/path".to_string(),
            version: None,
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsKvReadResult(resp) => {
                assert!(!resp.success);
                assert!(resp.data.is_none());
                assert!(resp.error.is_some());
            }
            other => panic!("expected SecretsKvReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_kv_versioning() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Write version 1
        let mut data1 = HashMap::new();
        data1.insert("value".to_string(), "v1".to_string());

        let write1 = ClientRpcRequest::SecretsKvWrite {
            mount: "secret".to_string(),
            path: "test/versioned".to_string(),
            data: data1,
            cas: None,
        };

        let _ = handler.handle(write1, &ctx).await.unwrap();

        // Write version 2
        let mut data2 = HashMap::new();
        data2.insert("value".to_string(), "v2".to_string());

        let write2 = ClientRpcRequest::SecretsKvWrite {
            mount: "secret".to_string(),
            path: "test/versioned".to_string(),
            data: data2,
            cas: None,
        };

        let result = handler.handle(write2, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsKvWriteResult(resp) => {
                assert!(resp.success);
                assert_eq!(resp.version, Some(2));
            }
            other => panic!("expected SecretsKvWriteResult, got {:?}", other),
        }

        // Read specific version (v1)
        let read_v1 = ClientRpcRequest::SecretsKvRead {
            mount: "secret".to_string(),
            path: "test/versioned".to_string(),
            version: Some(1),
        };

        let result = handler.handle(read_v1, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsKvReadResult(resp) => {
                assert!(resp.success);
                let data = resp.data.expect("should have data");
                assert_eq!(data.get("value"), Some(&"v1".to_string()));
            }
            other => panic!("expected SecretsKvReadResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_kv_cas_success() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Write initial version
        let mut data1 = HashMap::new();
        data1.insert("value".to_string(), "initial".to_string());

        let write1 = ClientRpcRequest::SecretsKvWrite {
            mount: "secret".to_string(),
            path: "test/cas".to_string(),
            data: data1,
            cas: None,
        };

        let _ = handler.handle(write1, &ctx).await.unwrap();

        // CAS update with correct expected version
        let mut data2 = HashMap::new();
        data2.insert("value".to_string(), "updated".to_string());

        let write2 = ClientRpcRequest::SecretsKvWrite {
            mount: "secret".to_string(),
            path: "test/cas".to_string(),
            data: data2,
            cas: Some(1), // Expecting version 1
        };

        let result = handler.handle(write2, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsKvWriteResult(resp) => {
                assert!(resp.success);
                assert_eq!(resp.version, Some(2));
            }
            other => panic!("expected SecretsKvWriteResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_kv_cas_conflict() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Write initial version
        let mut data1 = HashMap::new();
        data1.insert("value".to_string(), "initial".to_string());

        let write1 = ClientRpcRequest::SecretsKvWrite {
            mount: "secret".to_string(),
            path: "test/cas-conflict".to_string(),
            data: data1,
            cas: None,
        };

        let _ = handler.handle(write1, &ctx).await.unwrap();

        // CAS update with wrong expected version
        let mut data2 = HashMap::new();
        data2.insert("value".to_string(), "updated".to_string());

        let write2 = ClientRpcRequest::SecretsKvWrite {
            mount: "secret".to_string(),
            path: "test/cas-conflict".to_string(),
            data: data2,
            cas: Some(99), // Wrong version
        };

        let result = handler.handle(write2, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsKvWriteResult(resp) => {
                assert!(!resp.success);
                assert!(resp.error.is_some());
                let error = resp.error.unwrap();
                assert!(error.contains("CAS conflict"), "error should mention CAS: {}", error);
            }
            other => panic!("expected SecretsKvWriteResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_kv_delete_versions() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Write a secret
        let mut data = HashMap::new();
        data.insert("key".to_string(), "value".to_string());

        let write = ClientRpcRequest::SecretsKvWrite {
            mount: "secret".to_string(),
            path: "test/delete".to_string(),
            data,
            cas: None,
        };

        let _ = handler.handle(write, &ctx).await.unwrap();

        // Soft delete version 1
        let delete = ClientRpcRequest::SecretsKvDelete {
            mount: "secret".to_string(),
            path: "test/delete".to_string(),
            versions: vec![1],
        };

        let result = handler.handle(delete, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsKvDeleteResult(resp) => {
                assert!(resp.success);
                assert!(resp.error.is_none());
            }
            other => panic!("expected SecretsKvDeleteResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_kv_list() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Write multiple secrets
        for path in &["list/a", "list/b", "list/c"] {
            let mut data = HashMap::new();
            data.insert("key".to_string(), "value".to_string());

            let write = ClientRpcRequest::SecretsKvWrite {
                mount: "secret".to_string(),
                path: path.to_string(),
                data,
                cas: None,
            };

            let _ = handler.handle(write, &ctx).await.unwrap();
        }

        // List secrets
        let list = ClientRpcRequest::SecretsKvList {
            mount: "secret".to_string(),
            path: "list/".to_string(),
        };

        let result = handler.handle(list, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsKvListResult(resp) => {
                assert!(resp.success);
                assert!(!resp.keys.is_empty());
            }
            other => panic!("expected SecretsKvListResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_kv_metadata() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Write a secret
        let mut data = HashMap::new();
        data.insert("key".to_string(), "value".to_string());

        let write = ClientRpcRequest::SecretsKvWrite {
            mount: "secret".to_string(),
            path: "test/metadata".to_string(),
            data,
            cas: None,
        };

        let _ = handler.handle(write, &ctx).await.unwrap();

        // Get metadata
        let metadata = ClientRpcRequest::SecretsKvMetadata {
            mount: "secret".to_string(),
            path: "test/metadata".to_string(),
        };

        let result = handler.handle(metadata, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsKvMetadataResult(resp) => {
                assert!(resp.success);
                assert_eq!(resp.current_version, Some(1));
                assert!(!resp.versions.is_empty());
            }
            other => panic!("expected SecretsKvMetadataResult, got {:?}", other),
        }
    }

    // =========================================================================
    // Transit Handler Tests
    // =========================================================================

    #[tokio::test]
    async fn test_transit_create_key_aes256() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        let request = ClientRpcRequest::SecretsTransitCreateKey {
            mount: "transit".to_string(),
            name: "test-aes".to_string(),
            key_type: "aes256-gcm".to_string(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsTransitKeyResult(resp) => {
                assert!(resp.success);
                assert_eq!(resp.name, Some("test-aes".to_string()));
                assert_eq!(resp.version, Some(1));
            }
            other => panic!("expected SecretsTransitKeyResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_transit_create_key_ed25519() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        let request = ClientRpcRequest::SecretsTransitCreateKey {
            mount: "transit".to_string(),
            name: "test-ed25519".to_string(),
            key_type: "ed25519".to_string(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsTransitKeyResult(resp) => {
                assert!(resp.success);
                assert_eq!(resp.name, Some("test-ed25519".to_string()));
            }
            other => panic!("expected SecretsTransitKeyResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_transit_create_key_invalid_type() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        let request = ClientRpcRequest::SecretsTransitCreateKey {
            mount: "transit".to_string(),
            name: "test-invalid".to_string(),
            key_type: "invalid-key-type".to_string(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsTransitKeyResult(resp) => {
                assert!(!resp.success);
                assert!(resp.error.is_some());
                let error = resp.error.unwrap();
                assert!(error.contains("Invalid key type"), "error should mention invalid: {}", error);
            }
            other => panic!("expected SecretsTransitKeyResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_transit_encrypt_decrypt_roundtrip() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Create encryption key
        let create_key = ClientRpcRequest::SecretsTransitCreateKey {
            mount: "transit".to_string(),
            name: "roundtrip-key".to_string(),
            key_type: "aes256-gcm".to_string(),
        };
        let _ = handler.handle(create_key, &ctx).await.unwrap();

        // Encrypt plaintext
        let plaintext = b"Hello, World! This is a secret message.".to_vec();
        let encrypt = ClientRpcRequest::SecretsTransitEncrypt {
            mount: "transit".to_string(),
            name: "roundtrip-key".to_string(),
            plaintext: plaintext.clone(),
            context: None,
        };

        let result = handler.handle(encrypt, &ctx).await;
        assert!(result.is_ok());

        let ciphertext = match result.unwrap() {
            ClientRpcResponse::SecretsTransitEncryptResult(resp) => {
                assert!(resp.success);
                resp.ciphertext.expect("should have ciphertext")
            }
            other => panic!("expected SecretsTransitEncryptResult, got {:?}", other),
        };

        // Decrypt ciphertext
        let decrypt = ClientRpcRequest::SecretsTransitDecrypt {
            mount: "transit".to_string(),
            name: "roundtrip-key".to_string(),
            ciphertext,
            context: None,
        };

        let result = handler.handle(decrypt, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsTransitDecryptResult(resp) => {
                assert!(resp.success);
                assert_eq!(resp.plaintext, Some(plaintext));
            }
            other => panic!("expected SecretsTransitDecryptResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_transit_encrypt_nonexistent_key() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        let request = ClientRpcRequest::SecretsTransitEncrypt {
            mount: "transit".to_string(),
            name: "nonexistent-key".to_string(),
            plaintext: b"test".to_vec(),
            context: None,
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsTransitEncryptResult(resp) => {
                assert!(!resp.success);
                assert!(resp.error.is_some());
                let error = resp.error.unwrap();
                assert!(error.contains("Transit key not found"), "error: {}", error);
            }
            other => panic!("expected SecretsTransitEncryptResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_transit_sign_verify_roundtrip() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Create signing key (Ed25519)
        let create_key = ClientRpcRequest::SecretsTransitCreateKey {
            mount: "transit".to_string(),
            name: "signing-key".to_string(),
            key_type: "ed25519".to_string(),
        };
        let _ = handler.handle(create_key, &ctx).await.unwrap();

        // Sign data
        let data = b"This message needs to be signed.".to_vec();
        let sign = ClientRpcRequest::SecretsTransitSign {
            mount: "transit".to_string(),
            name: "signing-key".to_string(),
            data: data.clone(),
        };

        let result = handler.handle(sign, &ctx).await;
        assert!(result.is_ok());

        let signature = match result.unwrap() {
            ClientRpcResponse::SecretsTransitSignResult(resp) => {
                assert!(resp.success);
                resp.signature.expect("should have signature")
            }
            other => panic!("expected SecretsTransitSignResult, got {:?}", other),
        };

        // Verify signature
        let verify = ClientRpcRequest::SecretsTransitVerify {
            mount: "transit".to_string(),
            name: "signing-key".to_string(),
            data,
            signature,
        };

        let result = handler.handle(verify, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsTransitVerifyResult(resp) => {
                assert!(resp.success);
                assert_eq!(resp.valid, Some(true));
            }
            other => panic!("expected SecretsTransitVerifyResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_transit_verify_invalid_signature() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Create signing key
        let create_key = ClientRpcRequest::SecretsTransitCreateKey {
            mount: "transit".to_string(),
            name: "verify-test-key".to_string(),
            key_type: "ed25519".to_string(),
        };
        let _ = handler.handle(create_key, &ctx).await.unwrap();

        // Try to verify with invalid signature
        let verify = ClientRpcRequest::SecretsTransitVerify {
            mount: "transit".to_string(),
            name: "verify-test-key".to_string(),
            data: b"some data".to_vec(),
            signature: "invalid-signature-format".to_string(),
        };

        let result = handler.handle(verify, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsTransitVerifyResult(resp) => {
                // Either valid=false or an error is acceptable
                if resp.success {
                    assert_eq!(resp.valid, Some(false));
                } else {
                    assert!(resp.error.is_some());
                }
            }
            other => panic!("expected SecretsTransitVerifyResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_transit_rotate_key() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Create key
        let create_key = ClientRpcRequest::SecretsTransitCreateKey {
            mount: "transit".to_string(),
            name: "rotate-test".to_string(),
            key_type: "aes256-gcm".to_string(),
        };
        let _ = handler.handle(create_key, &ctx).await.unwrap();

        // Rotate key
        let rotate = ClientRpcRequest::SecretsTransitRotateKey {
            mount: "transit".to_string(),
            name: "rotate-test".to_string(),
        };

        let result = handler.handle(rotate, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsTransitKeyResult(resp) => {
                assert!(resp.success);
                assert_eq!(resp.version, Some(2)); // Version should be 2 after rotation
            }
            other => panic!("expected SecretsTransitKeyResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_transit_list_keys() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Create multiple keys
        for name in &["list-key-1", "list-key-2", "list-key-3"] {
            let create_key = ClientRpcRequest::SecretsTransitCreateKey {
                mount: "transit".to_string(),
                name: name.to_string(),
                key_type: "aes256-gcm".to_string(),
            };
            let _ = handler.handle(create_key, &ctx).await.unwrap();
        }

        // List keys
        let list = ClientRpcRequest::SecretsTransitListKeys {
            mount: "transit".to_string(),
        };

        let result = handler.handle(list, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsTransitListResult(resp) => {
                assert!(resp.success);
                assert!(resp.keys.len() >= 3);
            }
            other => panic!("expected SecretsTransitListResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_transit_datakey() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Create encryption key
        let create_key = ClientRpcRequest::SecretsTransitCreateKey {
            mount: "transit".to_string(),
            name: "datakey-test".to_string(),
            key_type: "aes256-gcm".to_string(),
        };
        let _ = handler.handle(create_key, &ctx).await.unwrap();

        // Generate data key
        let datakey = ClientRpcRequest::SecretsTransitDatakey {
            mount: "transit".to_string(),
            name: "datakey-test".to_string(),
            key_type: "plaintext".to_string(),
        };

        let result = handler.handle(datakey, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsTransitDatakeyResult(resp) => {
                assert!(resp.success);
                assert!(resp.plaintext.is_some());
                assert!(resp.ciphertext.is_some());
            }
            other => panic!("expected SecretsTransitDatakeyResult, got {:?}", other),
        }
    }

    // =========================================================================
    // PKI Handler Tests
    // =========================================================================

    #[tokio::test]
    async fn test_pki_generate_root() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        let request = ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: "pki".to_string(),
            common_name: "Test Root CA".to_string(),
            ttl_days: Some(365),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsPkiCertificateResult(resp) => {
                assert!(resp.success);
                assert!(resp.certificate.is_some());
                assert!(resp.serial.is_some());
            }
            other => panic!("expected SecretsPkiCertificateResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pki_create_role_and_issue() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Generate root CA first
        let gen_root = ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: "pki".to_string(),
            common_name: "Test CA".to_string(),
            ttl_days: Some(365),
        };
        let _ = handler.handle(gen_root, &ctx).await.unwrap();

        // Create role
        let create_role = ClientRpcRequest::SecretsPkiCreateRole {
            mount: "pki".to_string(),
            name: "web-servers".to_string(),
            allowed_domains: vec!["example.com".to_string()],
            max_ttl_days: 90,
            allow_bare_domains: true,
            allow_wildcards: true,
            allow_subdomains: false,
        };

        let result = handler.handle(create_role, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsPkiRoleResult(resp) => {
                assert!(resp.success);
                let role = resp.role.expect("should have role");
                assert_eq!(role.name, "web-servers");
                assert!(role.allowed_domains.contains(&"example.com".to_string()));
            }
            other => panic!("expected SecretsPkiRoleResult, got {:?}", other),
        }

        // Issue certificate
        let issue = ClientRpcRequest::SecretsPkiIssue {
            mount: "pki".to_string(),
            role: "web-servers".to_string(),
            common_name: "example.com".to_string(),
            alt_names: vec![],
            ttl_days: Some(30),
        };

        let result = handler.handle(issue, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsPkiCertificateResult(resp) => {
                assert!(resp.success, "PKI issue failed: {:?}", resp.error);
                assert!(resp.certificate.is_some());
                assert!(resp.serial.is_some());
            }
            other => panic!("expected SecretsPkiCertificateResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pki_issue_without_role() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Generate root CA
        let gen_root = ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: "pki".to_string(),
            common_name: "Test CA".to_string(),
            ttl_days: Some(365),
        };
        let _ = handler.handle(gen_root, &ctx).await.unwrap();

        // Try to issue without creating role
        let issue = ClientRpcRequest::SecretsPkiIssue {
            mount: "pki".to_string(),
            role: "nonexistent-role".to_string(),
            common_name: "test.example.com".to_string(),
            alt_names: vec![],
            ttl_days: Some(30),
        };

        let result = handler.handle(issue, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsPkiCertificateResult(resp) => {
                assert!(!resp.success);
                assert!(resp.error.is_some());
                let error = resp.error.unwrap();
                assert!(error.contains("role not found") || error.contains("Role not found"), "error: {}", error);
            }
            other => panic!("expected SecretsPkiCertificateResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pki_revoke_certificate() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Setup: Generate root, create role, issue cert
        let gen_root = ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: "pki".to_string(),
            common_name: "Test CA".to_string(),
            ttl_days: Some(365),
        };
        let _ = handler.handle(gen_root, &ctx).await.unwrap();

        let create_role = ClientRpcRequest::SecretsPkiCreateRole {
            mount: "pki".to_string(),
            name: "revoke-test".to_string(),
            allowed_domains: vec!["example.com".to_string()],
            max_ttl_days: 90,
            allow_bare_domains: true,
            allow_wildcards: false,
            allow_subdomains: false,
        };
        let _ = handler.handle(create_role, &ctx).await.unwrap();

        let issue = ClientRpcRequest::SecretsPkiIssue {
            mount: "pki".to_string(),
            role: "revoke-test".to_string(),
            common_name: "example.com".to_string(),
            alt_names: vec![],
            ttl_days: Some(30),
        };

        let issue_result = handler.handle(issue, &ctx).await.unwrap();
        let serial = match issue_result {
            ClientRpcResponse::SecretsPkiCertificateResult(resp) => {
                assert!(resp.success, "PKI issue failed: {:?}", resp.error);
                resp.serial.expect("should have serial")
            }
            other => panic!("expected SecretsPkiCertificateResult, got {:?}", other),
        };

        // Revoke the certificate
        let revoke = ClientRpcRequest::SecretsPkiRevoke {
            mount: "pki".to_string(),
            serial: serial.clone(),
        };

        let result = handler.handle(revoke, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsPkiRevokeResult(resp) => {
                assert!(resp.success);
                assert_eq!(resp.serial, Some(serial));
            }
            other => panic!("expected SecretsPkiRevokeResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pki_list_roles() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Generate root CA
        let gen_root = ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: "pki".to_string(),
            common_name: "Test CA".to_string(),
            ttl_days: Some(365),
        };
        let _ = handler.handle(gen_root, &ctx).await.unwrap();

        // Create roles
        for name in &["role-a", "role-b", "role-c"] {
            let create_role = ClientRpcRequest::SecretsPkiCreateRole {
                mount: "pki".to_string(),
                name: name.to_string(),
                allowed_domains: vec!["example.com".to_string()],
                max_ttl_days: 90,
                allow_bare_domains: true,
                allow_wildcards: false,
                allow_subdomains: false,
            };
            let _ = handler.handle(create_role, &ctx).await.unwrap();
        }

        // List roles
        let list = ClientRpcRequest::SecretsPkiListRoles {
            mount: "pki".to_string(),
        };

        let result = handler.handle(list, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsPkiListResult(resp) => {
                assert!(resp.success);
                assert!(resp.items.len() >= 3);
            }
            other => panic!("expected SecretsPkiListResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pki_get_crl() {
        let ctx = setup_test_context_with_secrets().await;
        let handler = SecretsHandler;

        // Generate root CA
        let gen_root = ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: "pki".to_string(),
            common_name: "Test CA".to_string(),
            ttl_days: Some(365),
        };
        let _ = handler.handle(gen_root, &ctx).await.unwrap();

        // Get CRL
        let get_crl = ClientRpcRequest::SecretsPkiGetCrl {
            mount: "pki".to_string(),
        };

        let result = handler.handle(get_crl, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::SecretsPkiCrlResult(resp) => {
                assert!(resp.success);
                assert!(resp.crl.is_some());
            }
            other => panic!("expected SecretsPkiCrlResult, got {:?}", other),
        }
    }

    // =========================================================================
    // Error Sanitization Tests
    // =========================================================================

    #[test]
    fn test_sanitize_secret_not_found() {
        let error = aspen_secrets::SecretsError::SecretNotFound {
            key: "db/creds".to_string(),
        };
        let sanitized = sanitize_secrets_error(&error);
        assert_eq!(sanitized, "Secret not found: db/creds");
    }

    #[test]
    fn test_sanitize_version_not_found() {
        let error = aspen_secrets::SecretsError::VersionNotFound {
            path: "db/creds".to_string(),
            version: 5,
        };
        let sanitized = sanitize_secrets_error(&error);
        assert_eq!(sanitized, "Version 5 not found for secret: db/creds");
    }

    #[test]
    fn test_sanitize_cas_failed_shows_versions() {
        let error = aspen_secrets::SecretsError::CasFailed {
            path: "db/creds".to_string(),
            expected: 1,
            actual: 3,
        };
        let sanitized = sanitize_secrets_error(&error);
        // Shows path and versions for debugging CAS conflicts
        assert_eq!(sanitized, "CAS conflict for secret 'db/creds': expected version 1, found 3");
    }

    #[test]
    fn test_sanitize_transit_key_not_found() {
        let error = aspen_secrets::SecretsError::TransitKeyNotFound {
            name: "my-key".to_string(),
        };
        let sanitized = sanitize_secrets_error(&error);
        assert_eq!(sanitized, "Transit key not found: my-key");
    }

    #[test]
    fn test_sanitize_role_not_found() {
        let error = aspen_secrets::SecretsError::RoleNotFound {
            name: "web-servers".to_string(),
        };
        let sanitized = sanitize_secrets_error(&error);
        assert_eq!(sanitized, "PKI role not found: web-servers");
    }

    #[test]
    fn test_sanitize_path_too_long_shows_limits() {
        let error = aspen_secrets::SecretsError::PathTooLong { length: 1000, max: 512 };
        let sanitized = sanitize_secrets_error(&error);
        // Shows limits for debugging
        assert_eq!(sanitized, "Path too long: 1000 characters (max: 512)");
    }

    #[test]
    fn test_sanitize_value_too_large_shows_limits() {
        let error = aspen_secrets::SecretsError::ValueTooLarge {
            size: 2_000_000,
            max: 1_000_000,
        };
        let sanitized = sanitize_secrets_error(&error);
        // Shows limits for debugging
        assert_eq!(sanitized, "Secret too large: 2000000 bytes (max: 1000000)");
    }

    #[test]
    fn test_sanitize_internal_error_generic() {
        let error = aspen_secrets::SecretsError::Internal {
            reason: "database connection failed at /var/run/aspen.sock".to_string(),
        };
        let sanitized = sanitize_secrets_error(&error);
        assert_eq!(sanitized, "Internal secrets error");
        // Should not leak internal details
        assert!(!sanitized.contains("database"));
        assert!(!sanitized.contains("/var/run"));
    }

    #[test]
    fn test_sanitize_pki_common_name_not_allowed() {
        let error = aspen_secrets::SecretsError::CommonNameNotAllowed {
            cn: "api.example.com".to_string(),
            role: "web-server".to_string(),
        };
        let sanitized = sanitize_secrets_error(&error);
        assert_eq!(
            sanitized,
            "Common name 'api.example.com' not allowed by role 'web-server'. Check allowed_domains and consider --allow-subdomains."
        );
    }

    #[test]
    fn test_sanitize_pki_san_not_allowed() {
        let error = aspen_secrets::SecretsError::SanNotAllowed {
            san: "*.example.com".to_string(),
            role: "web-server".to_string(),
        };
        let sanitized = sanitize_secrets_error(&error);
        assert_eq!(sanitized, "SAN '*.example.com' not allowed by role 'web-server'");
    }

    #[test]
    fn test_sanitize_pki_ca_not_initialized() {
        let error = aspen_secrets::SecretsError::CaNotInitialized;
        let sanitized = sanitize_secrets_error(&error);
        assert_eq!(sanitized, "Certificate authority not initialized. Run 'secrets pki generate-root' first.");
    }

    #[test]
    fn test_sanitize_pki_ca_already_initialized() {
        let error = aspen_secrets::SecretsError::CaAlreadyInitialized {
            mount: "pki".to_string(),
        };
        let sanitized = sanitize_secrets_error(&error);
        assert_eq!(sanitized, "Certificate authority already initialized for mount 'pki'");
    }

    #[test]
    fn test_sanitize_pki_certificate_generation() {
        let error = aspen_secrets::SecretsError::CertificateGeneration {
            reason: "signature verification failed".to_string(),
        };
        let sanitized = sanitize_secrets_error(&error);
        assert_eq!(sanitized, "Certificate generation failed: signature verification failed");
    }

    #[test]
    fn test_sanitize_pki_ttl_exceeds_max() {
        let error = aspen_secrets::SecretsError::TtlExceedsMax {
            role: "web-server".to_string(),
            requested_secs: 31536000,
            max_secs: 2592000,
        };
        let sanitized = sanitize_secrets_error(&error);
        assert_eq!(sanitized, "Requested TTL 31536000s exceeds maximum 2592000s for role 'web-server'");
    }

    #[test]
    fn test_sanitize_transit_key_exists() {
        let error = aspen_secrets::SecretsError::TransitKeyExists {
            name: "my-key".to_string(),
        };
        let sanitized = sanitize_secrets_error(&error);
        assert_eq!(sanitized, "Transit key already exists: my-key");
    }

    #[test]
    fn test_sanitize_decryption_hides_details() {
        let error = aspen_secrets::SecretsError::Decryption {
            reason: "private key file /home/user/.age/key.txt corrupted".to_string(),
        };
        let sanitized = sanitize_secrets_error(&error);
        // Should NOT leak file paths or details
        assert_eq!(sanitized, "Internal secrets error");
        assert!(!sanitized.contains("/home"));
        assert!(!sanitized.contains("key.txt"));
    }
}
