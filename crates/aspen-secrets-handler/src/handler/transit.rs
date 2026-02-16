//! Transit handler functions.
//!
//! Handles encryption-as-a-service operations (encrypt, decrypt, sign, verify).

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::SecretsTransitDatakeyResultResponse;
use aspen_client_api::SecretsTransitDecryptResultResponse;
use aspen_client_api::SecretsTransitEncryptResultResponse;
use aspen_client_api::SecretsTransitKeyResultResponse;
use aspen_client_api::SecretsTransitListResultResponse;
use aspen_client_api::SecretsTransitSignResultResponse;
use aspen_client_api::SecretsTransitVerifyResultResponse;
use aspen_rpc_core::ClientProtocolContext;
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

use super::SecretsService;
use super::sanitize_secrets_error;

/// Sub-handler for Transit secrets operations.
pub(crate) struct TransitSecretsHandler;

impl TransitSecretsHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::SecretsTransitCreateKey { .. }
                | ClientRpcRequest::SecretsTransitEncrypt { .. }
                | ClientRpcRequest::SecretsTransitDecrypt { .. }
                | ClientRpcRequest::SecretsTransitSign { .. }
                | ClientRpcRequest::SecretsTransitVerify { .. }
                | ClientRpcRequest::SecretsTransitRotateKey { .. }
                | ClientRpcRequest::SecretsTransitListKeys { .. }
                | ClientRpcRequest::SecretsTransitRewrap { .. }
                | ClientRpcRequest::SecretsTransitDatakey { .. }
        )
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        service: &SecretsService,
        _ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::SecretsTransitCreateKey { mount, name, key_type } => {
                handle_transit_create_key(service, &mount, name, key_type).await
            }
            ClientRpcRequest::SecretsTransitEncrypt {
                mount,
                name,
                plaintext,
                context,
            } => handle_transit_encrypt(service, &mount, name, plaintext, context).await,
            ClientRpcRequest::SecretsTransitDecrypt {
                mount,
                name,
                ciphertext,
                context,
            } => handle_transit_decrypt(service, &mount, name, ciphertext, context).await,
            ClientRpcRequest::SecretsTransitSign { mount, name, data } => {
                handle_transit_sign(service, &mount, name, data).await
            }
            ClientRpcRequest::SecretsTransitVerify {
                mount,
                name,
                data,
                signature,
            } => handle_transit_verify(service, &mount, name, data, signature).await,
            ClientRpcRequest::SecretsTransitRotateKey { mount, name } => {
                handle_transit_rotate_key(service, &mount, name).await
            }
            ClientRpcRequest::SecretsTransitListKeys { mount } => handle_transit_list_keys(service, &mount).await,
            ClientRpcRequest::SecretsTransitRewrap {
                mount,
                name,
                ciphertext,
                context,
            } => handle_transit_rewrap(service, &mount, name, ciphertext, context).await,
            ClientRpcRequest::SecretsTransitDatakey { mount, name, key_type } => {
                handle_transit_datakey(service, &mount, name, key_type).await
            }
            _ => Err(anyhow::anyhow!("request not handled by TransitSecretsHandler")),
        }
    }
}

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
                is_success: false,
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
            is_success: true,
            name: Some(key.name),
            version: Some(key.current_version as u64),
            key_type: Some(format!("{:?}", key.key_type)),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit create key failed");
            Ok(ClientRpcResponse::SecretsTransitKeyResult(SecretsTransitKeyResultResponse {
                is_success: false,
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
            is_success: true,
            ciphertext: Some(response.ciphertext),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit encrypt failed");
            Ok(ClientRpcResponse::SecretsTransitEncryptResult(SecretsTransitEncryptResultResponse {
                is_success: false,
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
            is_success: true,
            plaintext: Some(response.plaintext),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit decrypt failed");
            Ok(ClientRpcResponse::SecretsTransitDecryptResult(SecretsTransitDecryptResultResponse {
                is_success: false,
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
            is_success: true,
            signature: Some(response.signature),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit sign failed");
            Ok(ClientRpcResponse::SecretsTransitSignResult(SecretsTransitSignResultResponse {
                is_success: false,
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
            is_success: true,
            is_valid: Some(response.is_valid),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit verify failed");
            Ok(ClientRpcResponse::SecretsTransitVerifyResult(SecretsTransitVerifyResultResponse {
                is_success: false,
                is_valid: None,
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
            is_success: true,
            name: Some(key.name),
            version: Some(key.current_version as u64),
            key_type: Some(format!("{:?}", key.key_type)),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit rotate key failed");
            Ok(ClientRpcResponse::SecretsTransitKeyResult(SecretsTransitKeyResultResponse {
                is_success: false,
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
            is_success: true,
            keys,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit list keys failed");
            Ok(ClientRpcResponse::SecretsTransitListResult(SecretsTransitListResultResponse {
                is_success: false,
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
            is_success: true,
            ciphertext: Some(response.ciphertext),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit rewrap failed");
            Ok(ClientRpcResponse::SecretsTransitEncryptResult(SecretsTransitEncryptResultResponse {
                is_success: false,
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
            is_success: true,
            plaintext: Some(response.plaintext),
            ciphertext: Some(response.ciphertext),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "Transit datakey failed");
            Ok(ClientRpcResponse::SecretsTransitDatakeyResult(SecretsTransitDatakeyResultResponse {
                is_success: false,
                plaintext: None,
                ciphertext: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}
