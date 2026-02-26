//! Secrets service executor for typed RPC dispatch.
//!
//! Implements `ServiceExecutor` to handle PKI and Nix Cache secrets operations
//! when invoked via the RPC protocol.

use std::sync::Arc;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ServiceExecutor;
use async_trait::async_trait;

use crate::handler::SecretsService;
use crate::handler::nix_cache::NixCacheSecretsHandler;
use crate::handler::pki::PkiSecretsHandler;

/// Service executor for secrets operations (PKI + Nix Cache).
///
/// Handles the native-only operations that require crypto libraries:
/// - PKI: Certificate authority with role-based issuance
/// - Nix Cache: Signing key management for binary caches
///
/// KV and Transit operations are handled by the WASM secrets plugin.
pub struct SecretsServiceExecutor {
    secrets_service: Arc<SecretsService>,
    kv_store: Arc<dyn aspen_core::KeyValueStore>,
}

impl SecretsServiceExecutor {
    /// Create a new secrets service executor.
    pub fn new(secrets_service: Arc<SecretsService>, kv_store: Arc<dyn aspen_core::KeyValueStore>) -> Self {
        Self {
            secrets_service,
            kv_store,
        }
    }
}

#[async_trait]
impl ServiceExecutor for SecretsServiceExecutor {
    fn service_name(&self) -> &'static str {
        "secrets"
    }

    fn handles(&self) -> &'static [&'static str] {
        &[
            "SecretsPkiGenerateRoot",
            "SecretsPkiGenerateIntermediate",
            "SecretsPkiSetSignedIntermediate",
            "SecretsPkiCreateRole",
            "SecretsPkiIssue",
            "SecretsPkiRevoke",
            "SecretsPkiGetCrl",
            "SecretsPkiListCerts",
            "SecretsPkiGetRole",
            "SecretsPkiListRoles",
            "SecretsNixCacheCreateKey",
            "SecretsNixCacheGetPublicKey",
            "SecretsNixCacheRotateKey",
            "SecretsNixCacheDeleteKey",
            "SecretsNixCacheListKeys",
        ]
    }

    fn priority(&self) -> u32 {
        580
    }

    fn app_id(&self) -> Option<&'static str> {
        Some("secrets")
    }

    async fn execute(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        let pki = PkiSecretsHandler;
        let nix_cache = NixCacheSecretsHandler;

        if pki.can_handle(&request) {
            return pki.handle(request, &self.secrets_service).await;
        }
        if nix_cache.can_handle(&request) {
            return nix_cache.handle(request, &self.secrets_service, &self.kv_store).await;
        }

        unreachable!("SecretsServiceExecutor received unhandled request")
    }
}
