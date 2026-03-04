//! Nix cache signing handler functions.
//!
//! Handles Nix signing key management for binary caches.
//! Delegates key operations to `aspen_secrets::nix_cache::NixCacheKeyManager`.

use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::SecretsNixCacheDeleteResultResponse;
use aspen_client_api::SecretsNixCacheKeyResultResponse;
use aspen_client_api::SecretsNixCacheListResultResponse;
use aspen_core::ReadRequest;
use aspen_secrets::nix_cache::NixCacheKeyManager;
use tracing::debug;
use tracing::warn;

use super::SecretsService;
use super::sanitize_secrets_error;

/// Sub-handler for Nix cache signing operations.
pub(crate) struct NixCacheSecretsHandler;

impl NixCacheSecretsHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::SecretsNixCacheCreateKey { .. }
                | ClientRpcRequest::SecretsNixCacheGetPublicKey { .. }
                | ClientRpcRequest::SecretsNixCacheRotateKey { .. }
                | ClientRpcRequest::SecretsNixCacheDeleteKey { .. }
                | ClientRpcRequest::SecretsNixCacheListKeys { .. }
        )
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        service: &SecretsService,
        kv_store: &Arc<dyn aspen_core::KeyValueStore>,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::SecretsNixCacheCreateKey { mount, cache_name } => {
                handle_nix_cache_create_key(service, &mount, cache_name).await
            }
            ClientRpcRequest::SecretsNixCacheGetPublicKey { mount, cache_name } => {
                handle_nix_cache_get_public_key(service, &mount, cache_name, kv_store).await
            }
            ClientRpcRequest::SecretsNixCacheRotateKey { mount, cache_name } => {
                handle_nix_cache_rotate_key(service, &mount, cache_name).await
            }
            ClientRpcRequest::SecretsNixCacheDeleteKey { mount, cache_name } => {
                handle_nix_cache_delete_key(service, &mount, cache_name).await
            }
            ClientRpcRequest::SecretsNixCacheListKeys { mount } => handle_nix_cache_list_keys(service, &mount).await,
            _ => Err(anyhow::anyhow!("request not handled by NixCacheSecretsHandler")),
        }
    }
}

/// Build a `NixCacheKeyManager` from the service's Transit store at the given mount.
async fn key_manager(service: &SecretsService, mount: &str) -> anyhow::Result<NixCacheKeyManager> {
    let store = service.get_transit_store(mount).await?;
    Ok(NixCacheKeyManager::new(store))
}

/// Convert a `NixCachePublicKeyInfo` into a success response.
fn key_info_response(info: &aspen_secrets::nix_cache::NixCachePublicKeyInfo) -> ClientRpcResponse {
    ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
        is_success: true,
        public_key: Some(info.public_key_string.clone()),
        error: None,
    })
}

/// Build a key-result error response.
fn key_error_response(error: String) -> ClientRpcResponse {
    ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
        is_success: false,
        public_key: None,
        error: Some(error),
    })
}

async fn handle_nix_cache_create_key(
    service: &SecretsService,
    mount: &str,
    cache_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, cache_name = %cache_name, "Nix cache create key request");

    let mgr = key_manager(service, mount).await?;
    match mgr.create_signing_key(&cache_name).await {
        Ok(info) => Ok(key_info_response(&info)),
        Err(e) => Ok(key_error_response(format!("Failed to create key: {}", sanitize_secrets_error(&e)))),
    }
}

async fn handle_nix_cache_get_public_key(
    service: &SecretsService,
    mount: &str,
    cache_name: String,
    kv_store: &Arc<dyn aspen_core::KeyValueStore>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, cache_name = %cache_name, "Nix cache get public key request");

    // First try to read from Raft KV store (faster, distributed)
    let read_request = ReadRequest::new("_system:nix-cache:public-key");
    match kv_store.read(read_request).await {
        Ok(read_result) => {
            if let Some(kv) = read_result.kv {
                let public_key = kv.value;
                // Verify the cache name matches
                if public_key.starts_with(&format!("{}:", cache_name)) {
                    debug!(cache_name = %cache_name, "Public key retrieved from KV store");
                    return Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                        is_success: true,
                        public_key: Some(public_key),
                        error: None,
                    }));
                }
                warn!(
                    expected_cache = %cache_name,
                    stored_key = %public_key,
                    "Cache name mismatch in stored public key, falling back to Transit"
                );
            } else {
                debug!(cache_name = %cache_name, "Public key not found in KV store, trying Transit");
            }
        }
        Err(e) => {
            warn!(error = %e, "Failed to read from KV store, trying Transit");
        }
    }

    // Fall through to Transit
    let mgr = key_manager(service, mount).await?;
    match mgr.get_public_key(&cache_name).await {
        Ok(Some(info)) => Ok(key_info_response(&info)),
        Ok(None) => Ok(key_error_response("Key not found".to_string())),
        Err(e) => Ok(key_error_response(format!("Key not found or read failed: {}", sanitize_secrets_error(&e)))),
    }
}

async fn handle_nix_cache_rotate_key(
    service: &SecretsService,
    mount: &str,
    cache_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, cache_name = %cache_name, "Nix cache rotate key request");

    let mgr = key_manager(service, mount).await?;
    match mgr.rotate_signing_key(&cache_name).await {
        Ok(info) => Ok(key_info_response(&info)),
        Err(e) => Ok(key_error_response(format!("Failed to rotate key: {}", sanitize_secrets_error(&e)))),
    }
}

async fn handle_nix_cache_delete_key(
    service: &SecretsService,
    mount: &str,
    cache_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, cache_name = %cache_name, "Nix cache delete key request");

    let mgr = key_manager(service, mount).await?;
    match mgr.delete_signing_key(&cache_name).await {
        Ok(_) => Ok(ClientRpcResponse::SecretsNixCacheDeleteResult(SecretsNixCacheDeleteResultResponse {
            is_success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SecretsNixCacheDeleteResult(SecretsNixCacheDeleteResultResponse {
            is_success: false,
            error: Some(format!("Failed to delete key: {}", sanitize_secrets_error(&e))),
        })),
    }
}

async fn handle_nix_cache_list_keys(service: &SecretsService, mount: &str) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, "Nix cache list keys request");

    let mgr = key_manager(service, mount).await?;
    match mgr.list_signing_keys().await {
        Ok(keys) => Ok(ClientRpcResponse::SecretsNixCacheListResult(SecretsNixCacheListResultResponse {
            is_success: true,
            cache_names: Some(keys),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SecretsNixCacheListResult(SecretsNixCacheListResultResponse {
            is_success: false,
            cache_names: None,
            error: Some(format!("Failed to list keys: {}", sanitize_secrets_error(&e))),
        })),
    }
}
