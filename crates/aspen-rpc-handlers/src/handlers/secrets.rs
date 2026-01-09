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

use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_client_rpc::SecretsKvDeleteResultResponse;
use aspen_client_rpc::SecretsKvListResultResponse;
use aspen_client_rpc::SecretsKvMetadataResultResponse;
use aspen_client_rpc::SecretsKvReadResultResponse;
use aspen_client_rpc::SecretsKvVersionInfo;
use aspen_client_rpc::SecretsKvVersionMetadata;
use aspen_client_rpc::SecretsKvWriteResultResponse;
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
            // KV v2 operations
            ClientRpcRequest::SecretsKvRead { path, version, .. } => {
                handle_kv_read(secrets_service, path, version).await
            }
            ClientRpcRequest::SecretsKvWrite { path, data, cas, .. } => {
                handle_kv_write(secrets_service, path, data, cas).await
            }
            ClientRpcRequest::SecretsKvDelete { path, versions, .. } => {
                handle_kv_delete(secrets_service, path, versions).await
            }
            ClientRpcRequest::SecretsKvDestroy { path, versions, .. } => {
                handle_kv_destroy(secrets_service, path, versions).await
            }
            ClientRpcRequest::SecretsKvUndelete { path, versions, .. } => {
                handle_kv_undelete(secrets_service, path, versions).await
            }
            ClientRpcRequest::SecretsKvList { path, .. } => handle_kv_list(secrets_service, path).await,
            ClientRpcRequest::SecretsKvMetadata { path, .. } => handle_kv_metadata(secrets_service, path).await,
            ClientRpcRequest::SecretsKvUpdateMetadata {
                path,
                max_versions,
                cas_required,
                custom_metadata,
                ..
            } => handle_kv_update_metadata(secrets_service, path, max_versions, cas_required, custom_metadata).await,
            ClientRpcRequest::SecretsKvDeleteMetadata { path, .. } => {
                handle_kv_delete_metadata(secrets_service, path).await
            }
            // Transit operations
            ClientRpcRequest::SecretsTransitCreateKey { name, key_type, .. } => {
                handle_transit_create_key(secrets_service, name, key_type).await
            }
            ClientRpcRequest::SecretsTransitEncrypt {
                name,
                plaintext,
                context,
                ..
            } => handle_transit_encrypt(secrets_service, name, plaintext, context).await,
            ClientRpcRequest::SecretsTransitDecrypt {
                name,
                ciphertext,
                context,
                ..
            } => handle_transit_decrypt(secrets_service, name, ciphertext, context).await,
            ClientRpcRequest::SecretsTransitSign { name, data, .. } => {
                handle_transit_sign(secrets_service, name, data).await
            }
            ClientRpcRequest::SecretsTransitVerify {
                name, data, signature, ..
            } => handle_transit_verify(secrets_service, name, data, signature).await,
            ClientRpcRequest::SecretsTransitRotateKey { name, .. } => {
                handle_transit_rotate_key(secrets_service, name).await
            }
            ClientRpcRequest::SecretsTransitListKeys { .. } => handle_transit_list_keys(secrets_service).await,
            ClientRpcRequest::SecretsTransitRewrap {
                name,
                ciphertext,
                context,
                ..
            } => handle_transit_rewrap(secrets_service, name, ciphertext, context).await,
            ClientRpcRequest::SecretsTransitDatakey { name, key_type, .. } => {
                handle_transit_datakey(secrets_service, name, key_type).await
            }
            // PKI operations
            ClientRpcRequest::SecretsPkiGenerateRoot {
                common_name, ttl_days, ..
            } => handle_pki_generate_root(secrets_service, common_name, ttl_days).await,
            ClientRpcRequest::SecretsPkiGenerateIntermediate { common_name, .. } => {
                handle_pki_generate_intermediate(secrets_service, common_name).await
            }
            ClientRpcRequest::SecretsPkiSetSignedIntermediate { certificate, .. } => {
                handle_pki_set_signed_intermediate(secrets_service, certificate).await
            }
            ClientRpcRequest::SecretsPkiCreateRole {
                name,
                allowed_domains,
                max_ttl_days,
                allow_bare_domains,
                allow_wildcards,
                ..
            } => {
                handle_pki_create_role(
                    secrets_service,
                    name,
                    allowed_domains,
                    max_ttl_days,
                    allow_bare_domains,
                    allow_wildcards,
                )
                .await
            }
            ClientRpcRequest::SecretsPkiIssue {
                role,
                common_name,
                alt_names,
                ttl_days,
                ..
            } => handle_pki_issue(secrets_service, role, common_name, alt_names, ttl_days).await,
            ClientRpcRequest::SecretsPkiRevoke { serial, .. } => handle_pki_revoke(secrets_service, serial).await,
            ClientRpcRequest::SecretsPkiGetCrl { .. } => handle_pki_get_crl(secrets_service).await,
            ClientRpcRequest::SecretsPkiListCerts { .. } => handle_pki_list_certs(secrets_service).await,
            ClientRpcRequest::SecretsPkiGetRole { name, .. } => handle_pki_get_role(secrets_service, name).await,
            ClientRpcRequest::SecretsPkiListRoles { .. } => handle_pki_list_roles(secrets_service).await,
            _ => Err(anyhow::anyhow!("request not handled by SecretsHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "SecretsHandler"
    }
}

/// Secrets service aggregating all secrets engines.
pub struct SecretsService {
    /// KV v2 secrets engine.
    pub kv_store: Arc<dyn KvStore>,
    /// Transit secrets engine.
    pub transit_store: Arc<dyn TransitStore>,
    /// PKI secrets engine.
    pub pki_store: Arc<dyn PkiStore>,
}

impl SecretsService {
    /// Create a new secrets service.
    pub fn new(kv_store: Arc<dyn KvStore>, transit_store: Arc<dyn TransitStore>, pki_store: Arc<dyn PkiStore>) -> Self {
        Self {
            kv_store,
            transit_store,
            pki_store,
        }
    }
}

// =============================================================================
// KV v2 Handler Functions
// =============================================================================

async fn handle_kv_read(
    service: &SecretsService,
    path: String,
    version: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(path = %path, version = ?version, "KV read request");

    let request = ReadSecretRequest { path, version };
    match service.kv_store.read(request).await {
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
    path: String,
    data: HashMap<String, String>,
    cas: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(path = %path, cas = ?cas, "KV write request");

    let request = WriteSecretRequest {
        path,
        data: SecretData { data },
        cas,
    };
    match service.kv_store.write(request).await {
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
    path: String,
    versions: Vec<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(path = %path, versions = ?versions, "KV delete request");

    let request = DeleteSecretRequest { path, versions };
    match service.kv_store.delete(request).await {
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
    path: String,
    versions: Vec<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(path = %path, versions = ?versions, "KV destroy request");

    let request = DestroySecretRequest { path, versions };
    match service.kv_store.destroy(request).await {
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
    path: String,
    versions: Vec<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(path = %path, versions = ?versions, "KV undelete request");

    let request = UndeleteSecretRequest { path, versions };
    match service.kv_store.undelete(request).await {
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

async fn handle_kv_list(service: &SecretsService, path: String) -> anyhow::Result<ClientRpcResponse> {
    debug!(path = %path, "KV list request");

    let request = ListSecretsRequest { path };
    match service.kv_store.list(request).await {
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

async fn handle_kv_metadata(service: &SecretsService, path: String) -> anyhow::Result<ClientRpcResponse> {
    debug!(path = %path, "KV metadata request");

    let request = ReadMetadataRequest { path };
    match service.kv_store.read_metadata(request).await {
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
    path: String,
    max_versions: Option<u32>,
    cas_required: Option<bool>,
    custom_metadata: Option<HashMap<String, String>>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(path = %path, "KV update metadata request");

    let request = UpdateMetadataRequest {
        path,
        max_versions,
        cas_required,
        custom_metadata,
        delete_version_after_secs: None,
    };
    match service.kv_store.update_metadata(request).await {
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

async fn handle_kv_delete_metadata(service: &SecretsService, path: String) -> anyhow::Result<ClientRpcResponse> {
    debug!(path = %path, "KV delete metadata request");

    match service.kv_store.delete_metadata(&path).await {
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
    name: String,
    key_type: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(name = %name, key_type = %key_type, "Transit create key request");

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

    let request = CreateKeyRequest {
        name: name.clone(),
        key_type: key_type_enum,
        exportable: false,
        deletion_allowed: false,
        convergent_encryption: false,
    };

    match service.transit_store.create_key(request).await {
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
    name: String,
    plaintext: Vec<u8>,
    context: Option<Vec<u8>>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(name = %name, plaintext_len = plaintext.len(), "Transit encrypt request");

    let request = EncryptRequest {
        key_name: name,
        plaintext,
        context,
        key_version: None,
    };

    match service.transit_store.encrypt(request).await {
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
    name: String,
    ciphertext: String,
    context: Option<Vec<u8>>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(name = %name, "Transit decrypt request");

    let request = DecryptRequest {
        key_name: name,
        ciphertext,
        context,
    };

    match service.transit_store.decrypt(request).await {
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
    name: String,
    data: Vec<u8>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(name = %name, data_len = data.len(), "Transit sign request");

    let request = SignRequest {
        key_name: name,
        input: data,
        hash_algorithm: None,
        prehashed: false,
        key_version: None,
    };

    match service.transit_store.sign(request).await {
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
    name: String,
    data: Vec<u8>,
    signature: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(name = %name, data_len = data.len(), "Transit verify request");

    let request = VerifyRequest {
        key_name: name,
        input: data,
        signature,
        hash_algorithm: None,
        prehashed: false,
    };

    match service.transit_store.verify(request).await {
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

async fn handle_transit_rotate_key(service: &SecretsService, name: String) -> anyhow::Result<ClientRpcResponse> {
    debug!(name = %name, "Transit rotate key request");

    match service.transit_store.rotate_key(&name).await {
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

async fn handle_transit_list_keys(service: &SecretsService) -> anyhow::Result<ClientRpcResponse> {
    debug!("Transit list keys request");

    match service.transit_store.list_keys().await {
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
    name: String,
    ciphertext: String,
    context: Option<Vec<u8>>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(name = %name, "Transit rewrap request");

    let request = RewrapRequest {
        key_name: name,
        ciphertext,
        context,
    };

    match service.transit_store.rewrap(request).await {
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
    name: String,
    key_type: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(name = %name, key_type = %key_type, "Transit datakey request");

    // Note: include_plaintext is handled by the response type, not the request
    // The client specifies "plaintext" or "wrapped" to indicate whether they want
    // the plaintext included in the response
    let _include_plaintext = key_type == "plaintext";

    let request = DataKeyRequest {
        key_name: name,
        bits: 256,
        context: None,
    };

    match service.transit_store.generate_data_key(request).await {
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
    common_name: String,
    ttl_days: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(common_name = %common_name, ttl_days = ?ttl_days, "PKI generate root request");

    let ttl_secs = ttl_days.map(|d| d as u64 * 24 * 3600).unwrap_or(10 * 365 * 24 * 3600);
    let request = GenerateRootRequest::new(common_name).with_ttl_secs(ttl_secs);

    match service.pki_store.generate_root(request).await {
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
    common_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(common_name = %common_name, "PKI generate intermediate request");

    let request = GenerateIntermediateRequest::new(common_name);

    match service.pki_store.generate_intermediate(request).await {
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
    certificate: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("PKI set signed intermediate request");

    let request = SetSignedIntermediateRequest {
        certificate: certificate.clone(),
    };

    match service.pki_store.set_signed_intermediate(request).await {
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
    name: String,
    allowed_domains: Vec<String>,
    max_ttl_days: u32,
    allow_bare_domains: bool,
    allow_wildcards: bool,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(name = %name, "PKI create role request");

    let mut role = PkiRole::new(name.clone());
    role.allowed_domains = allowed_domains.clone();
    role.max_ttl_secs = max_ttl_days as u64 * 24 * 3600;
    role.allow_bare_domains = allow_bare_domains;
    role.allow_wildcard_certificates = allow_wildcards;

    let request = CreateRoleRequest {
        name: name.clone(),
        config: role,
    };

    match service.pki_store.create_role(request).await {
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
    role: String,
    common_name: String,
    alt_names: Vec<String>,
    ttl_days: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(role = %role, common_name = %common_name, "PKI issue request");

    let request = IssueCertificateRequest {
        role,
        common_name,
        alt_names,
        ip_sans: vec![],
        uri_sans: vec![],
        ttl_secs: ttl_days.map(|d| d as u64 * 24 * 3600),
        exclude_cn_from_sans: false,
    };

    match service.pki_store.issue(request).await {
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

async fn handle_pki_revoke(service: &SecretsService, serial: String) -> anyhow::Result<ClientRpcResponse> {
    debug!(serial = %serial, "PKI revoke request");

    let request = RevokeCertificateRequest { serial: serial.clone() };

    match service.pki_store.revoke(request).await {
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

async fn handle_pki_get_crl(service: &SecretsService) -> anyhow::Result<ClientRpcResponse> {
    debug!("PKI get CRL request");

    match service.pki_store.get_crl().await {
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

async fn handle_pki_list_certs(service: &SecretsService) -> anyhow::Result<ClientRpcResponse> {
    debug!("PKI list certs request");

    match service.pki_store.list_certificates().await {
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

async fn handle_pki_get_role(service: &SecretsService, name: String) -> anyhow::Result<ClientRpcResponse> {
    debug!(name = %name, "PKI get role request");

    match service.pki_store.read_role(&name).await {
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

async fn handle_pki_list_roles(service: &SecretsService) -> anyhow::Result<ClientRpcResponse> {
    debug!("PKI list roles request");

    match service.pki_store.list_roles().await {
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
/// Removes internal details that could leak sensitive information.
fn sanitize_secrets_error(error: &aspen_secrets::SecretsError) -> String {
    match error {
        aspen_secrets::SecretsError::SecretNotFound { key } => {
            format!("Secret not found: {}", key)
        }
        aspen_secrets::SecretsError::VersionNotFound { path, version } => {
            format!("Version {} not found for secret: {}", version, path)
        }
        aspen_secrets::SecretsError::CasFailed { path, .. } => {
            format!("CAS conflict for secret: {}", path)
        }
        aspen_secrets::SecretsError::TransitKeyNotFound { name } => {
            format!("Transit key not found: {}", name)
        }
        aspen_secrets::SecretsError::RoleNotFound { name } => {
            format!("PKI role not found: {}", name)
        }
        aspen_secrets::SecretsError::PathTooLong { .. } => "Path too long".to_string(),
        aspen_secrets::SecretsError::ValueTooLarge { .. } => "Secret too large".to_string(),
        aspen_secrets::SecretsError::TooManyVersions { .. } => "Too many versions".to_string(),
        _ => "Internal secrets error".to_string(),
    }
}
