//! Nix cache signing handler functions.
//!
//! Handles Nix signing key management for binary caches.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::SecretsNixCacheDeleteResultResponse;
use aspen_client_api::SecretsNixCacheKeyResultResponse;
use aspen_client_api::SecretsNixCacheListResultResponse;
use aspen_core::ReadRequest;
use aspen_rpc_core::ClientProtocolContext;
use base64::Engine;
use tracing::debug;
use tracing::warn;

use super::sanitize_secrets_error;
use crate::handler::SecretsService;

pub(crate) async fn handle_nix_cache_create_key(
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

pub(crate) async fn handle_nix_cache_get_public_key(
    service: &SecretsService,
    mount: &str,
    cache_name: String,
    ctx: &ClientProtocolContext,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, cache_name = %cache_name, "Nix cache get public key request");

    // First try to read from Raft KV store (faster, distributed)
    let read_request = ReadRequest::new("_system:nix-cache:public-key");
    match ctx.kv_store.read(read_request).await {
        Ok(read_result) => {
            if let Some(kv) = read_result.kv {
                let public_key = kv.value;
                // Verify the cache name matches
                if public_key.starts_with(&format!("{}:", cache_name)) {
                    debug!(cache_name = %cache_name, "Public key retrieved from KV store");
                    Ok(ClientRpcResponse::SecretsNixCacheKeyResult(SecretsNixCacheKeyResultResponse {
                        success: true,
                        public_key: Some(public_key),
                        error: None,
                    }))
                } else {
                    warn!(
                        expected_cache = %cache_name,
                        stored_key = %public_key,
                        "Cache name mismatch in stored public key, falling back to Transit"
                    );
                    // Fall through to Transit fallback below
                    read_from_transit(service, mount, &cache_name).await
                }
            } else {
                debug!(cache_name = %cache_name, "Public key not found in KV store, trying Transit");
                // Fall through to Transit fallback below
                read_from_transit(service, mount, &cache_name).await
            }
        }
        Err(e) => {
            warn!(error = %e, "Failed to read from KV store, trying Transit");
            // Fall through to Transit fallback below
            read_from_transit(service, mount, &cache_name).await
        }
    }
}

/// Read public key directly from Transit store (fallback when KV store is not available).
async fn read_from_transit(
    service: &SecretsService,
    mount: &str,
    cache_name: &str,
) -> anyhow::Result<ClientRpcResponse> {
    let store = service.get_transit_store(mount).await?;

    match store.read_key(cache_name).await {
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

pub(crate) async fn handle_nix_cache_rotate_key(
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

pub(crate) async fn handle_nix_cache_delete_key(
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

pub(crate) async fn handle_nix_cache_list_keys(
    service: &SecretsService,
    mount: &str,
) -> anyhow::Result<ClientRpcResponse> {
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
