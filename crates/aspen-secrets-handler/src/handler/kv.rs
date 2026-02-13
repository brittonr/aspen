//! KV v2 handler functions.
//!
//! Handles versioned key-value secrets with soft/hard delete.

use std::collections::HashMap;

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::SecretsKvDeleteResultResponse;
use aspen_client_api::SecretsKvListResultResponse;
use aspen_client_api::SecretsKvMetadataResultResponse;
use aspen_client_api::SecretsKvReadResultResponse;
use aspen_client_api::SecretsKvVersionInfo;
use aspen_client_api::SecretsKvVersionMetadata;
use aspen_client_api::SecretsKvWriteResultResponse;
use aspen_secrets::kv::DeleteSecretRequest;
use aspen_secrets::kv::DestroySecretRequest;
use aspen_secrets::kv::ListSecretsRequest;
use aspen_secrets::kv::ReadMetadataRequest;
use aspen_secrets::kv::ReadSecretRequest;
use aspen_secrets::kv::SecretData;
use aspen_secrets::kv::UndeleteSecretRequest;
use aspen_secrets::kv::UpdateMetadataRequest;
use aspen_secrets::kv::WriteSecretRequest;
use tracing::debug;
use tracing::warn;

use super::sanitize_secrets_error;
use crate::handler::SecretsService;

pub(crate) async fn handle_kv_read(
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

pub(crate) async fn handle_kv_write(
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

pub(crate) async fn handle_kv_delete(
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

pub(crate) async fn handle_kv_destroy(
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

pub(crate) async fn handle_kv_undelete(
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

pub(crate) async fn handle_kv_list(
    service: &SecretsService,
    mount: &str,
    path: String,
) -> anyhow::Result<ClientRpcResponse> {
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

pub(crate) async fn handle_kv_metadata(
    service: &SecretsService,
    mount: &str,
    path: String,
) -> anyhow::Result<ClientRpcResponse> {
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

pub(crate) async fn handle_kv_update_metadata(
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

pub(crate) async fn handle_kv_delete_metadata(
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
