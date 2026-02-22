//! Secrets engine request handler (native subset).
//!
//! Handles: SecretsPki*, SecretsNixCache* operations only.
//!
//! KV and Transit secrets operations have been migrated to the
//! `aspen-secrets-plugin` WASM plugin. This native handler retains
//! PKI (X.509 certificate authority) and Nix cache signing because
//! they depend on native crypto libraries (rcgen, ed25519) that
//! cannot yet run inside the WASM sandbox.

mod nix_cache;
mod pki;

use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use aspen_secrets::PkiStore;
use nix_cache::NixCacheSecretsHandler;
use pki::PkiSecretsHandler;

/// Handler for native-only secrets engine operations (PKI + Nix Cache).
///
/// KV and Transit operations are now served by the WASM secrets plugin.
/// This handler retains only the operations that require native crypto.
pub struct SecretsHandler;

impl SecretsHandler {
    /// Obtain the secrets service from the context, or return an error response.
    #[allow(clippy::result_large_err)]
    fn get_secrets_service(ctx: &ClientProtocolContext) -> Result<Arc<SecretsService>, ClientRpcResponse> {
        let Some(ref secrets_any) = ctx.secrets_service else {
            return Err(ClientRpcResponse::Error(aspen_client_api::ErrorResponse {
                code: "SECRETS_NOT_ENABLED".to_string(),
                message: "Secrets engine is not enabled on this node".to_string(),
            }));
        };

        secrets_any.clone().downcast::<SecretsService>().map_err(|_| {
            ClientRpcResponse::Error(aspen_client_api::ErrorResponse {
                code: "SECRETS_SERVICE_ERROR".to_string(),
                message: "Secrets service has wrong type".to_string(),
            })
        })
    }
}

#[async_trait::async_trait]
impl RequestHandler for SecretsHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        let pki = PkiSecretsHandler;
        let nix_cache = NixCacheSecretsHandler;

        pki.can_handle(request) || nix_cache.can_handle(request)
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        let secrets_service = match Self::get_secrets_service(ctx) {
            Ok(service) => service,
            Err(error_response) => return Ok(error_response),
        };

        let pki = PkiSecretsHandler;
        let nix_cache = NixCacheSecretsHandler;

        if pki.can_handle(&request) {
            return pki.handle(request, &secrets_service, ctx).await;
        }
        if nix_cache.can_handle(&request) {
            return nix_cache.handle(request, &secrets_service, ctx).await;
        }

        Err(anyhow::anyhow!("request not handled by SecretsHandler"))
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

    /// Get or create a KV store for the given mount point.
    ///
    /// Retained for NixCache handler which stores signing keys via KV.
    ///
    /// # Errors
    ///
    /// Returns an error if the mount name is invalid or too many mounts exist.
    pub async fn get_kv_store(&self, mount: &str) -> anyhow::Result<Arc<dyn aspen_secrets::KvStore>> {
        self.mount_registry
            .get_or_create_kv_store(mount)
            .await
            .map_err(|e| anyhow::anyhow!("failed to get KV store for mount '{}': {}", mount, e))
    }

    /// Get or create a Transit store for the given mount point.
    ///
    /// Retained for NixCache handler which uses transit for signing operations.
    ///
    /// # Errors
    ///
    /// Returns an error if the mount name is invalid or too many mounts exist.
    pub async fn get_transit_store(&self, mount: &str) -> anyhow::Result<Arc<dyn aspen_secrets::TransitStore>> {
        self.mount_registry
            .get_or_create_transit_store(mount)
            .await
            .map_err(|e| anyhow::anyhow!("failed to get Transit store for mount '{}': {}", mount, e))
    }
}

// =============================================================================
// Error Sanitization
// =============================================================================

/// Sanitize secrets errors for client display.
///
/// Returns user-friendly error messages without leaking sensitive internal details.
pub(crate) fn sanitize_secrets_error(error: &aspen_secrets::SecretsError) -> String {
    use aspen_secrets::SecretsError;

    match error {
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

        // NixCache-relevant transit errors
        SecretsError::TransitKeyNotFound { name } => format!("Transit key not found: {name}"),
        SecretsError::TransitKeyExists { name } => format!("Transit key already exists: {name}"),

        // All other errors: return a generic message to avoid leaking internals.
        _ => "Internal secrets error".to_string(),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_client_api::ClientRpcRequest;
    use aspen_client_api::ClientRpcResponse;
    use aspen_rpc_core::test_support::TestContextBuilder;

    use super::*;

    // =========================================================================
    // Mock EndpointProvider for Testing
    // =========================================================================

    struct MockEndpointProvider {
        endpoint: iroh::Endpoint,
        node_addr: iroh::EndpointAddr,
        public_key: Vec<u8>,
        peer_id: String,
    }

    impl MockEndpointProvider {
        async fn new() -> Self {
            let mut key_bytes = [0u8; 32];
            key_bytes[0..8].copy_from_slice(&0u64.to_le_bytes());
            let secret_key = iroh::SecretKey::from_bytes(&key_bytes);

            let endpoint = iroh::Endpoint::builder()
                .secret_key(secret_key.clone())
                .bind_addr_v4("127.0.0.1:0".parse().unwrap())
                .bind()
                .await
                .expect("failed to create mock endpoint");

            let node_addr = endpoint.addr();
            let public_key = secret_key.public().as_bytes().to_vec();
            let peer_id = node_addr.id.fmt_short().to_string();

            Self {
                endpoint,
                node_addr,
                public_key,
                peer_id,
            }
        }
    }

    #[async_trait::async_trait]
    impl aspen_core::EndpointProvider for MockEndpointProvider {
        async fn public_key(&self) -> Vec<u8> {
            self.public_key.clone()
        }

        async fn peer_id(&self) -> String {
            self.peer_id.clone()
        }

        async fn addresses(&self) -> Vec<String> {
            vec!["127.0.0.1:0".to_string()]
        }

        fn node_addr(&self) -> &iroh::EndpointAddr {
            &self.node_addr
        }

        fn endpoint(&self) -> &iroh::Endpoint {
            &self.endpoint
        }
    }

    fn make_secrets_service(kv_store: Arc<dyn aspen_core::KeyValueStore>) -> SecretsService {
        let mount_registry = Arc::new(aspen_secrets::MountRegistry::new(kv_store));
        SecretsService::new(mount_registry)
    }

    async fn setup_test_context_with_secrets() -> ClientProtocolContext {
        use aspen_testing::DeterministicKeyValueStore;

        let mock_endpoint: Arc<dyn aspen_core::EndpointProvider> = Arc::new(MockEndpointProvider::new().await);
        let kv_store: Arc<dyn aspen_core::KeyValueStore> = Arc::new(DeterministicKeyValueStore::new());
        let secrets_service = Arc::new(make_secrets_service(Arc::clone(&kv_store)));

        let builder = TestContextBuilder::new()
            .with_node_id(1)
            .with_endpoint_manager(mock_endpoint)
            .with_cookie("test_cluster")
            .with_kv_store(kv_store);

        let mut ctx = builder.build();
        ctx.secrets_service = Some(secrets_service);
        ctx
    }

    async fn setup_test_context_without_secrets() -> ClientProtocolContext {
        let mock_endpoint: Arc<dyn aspen_core::EndpointProvider> = Arc::new(MockEndpointProvider::new().await);

        let builder = TestContextBuilder::new()
            .with_node_id(1)
            .with_endpoint_manager(mock_endpoint)
            .with_cookie("test_cluster");

        builder.build()
    }

    // =========================================================================
    // Dispatch Tests
    // =========================================================================

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
    fn test_rejects_kv_requests() {
        let handler = SecretsHandler;
        // KV requests are now handled by WASM plugin
        assert!(!handler.can_handle(&ClientRpcRequest::SecretsKvRead {
            mount: "secret".to_string(),
            path: "test/path".to_string(),
            version: None,
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::SecretsKvWrite {
            mount: "secret".to_string(),
            path: "test/path".to_string(),
            data: std::collections::HashMap::new(),
            cas: None,
        }));
    }

    #[test]
    fn test_rejects_transit_requests() {
        let handler = SecretsHandler;
        // Transit requests are now handled by WASM plugin
        assert!(!handler.can_handle(&ClientRpcRequest::SecretsTransitCreateKey {
            mount: "transit".to_string(),
            name: "my-key".to_string(),
            key_type: "aes256-gcm".to_string(),
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::SecretsTransitEncrypt {
            mount: "transit".to_string(),
            name: "my-key".to_string(),
            plaintext: b"test".to_vec(),
            context: None,
        }));
    }

    #[test]
    fn test_rejects_unrelated_requests() {
        let handler = SecretsHandler;
        assert!(!handler.can_handle(&ClientRpcRequest::Ping));
        assert!(!handler.can_handle(&ClientRpcRequest::GetHealth));
        assert!(!handler.can_handle(&ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        }));
    }

    #[test]
    fn test_handler_name() {
        let handler = SecretsHandler;
        assert_eq!(handler.name(), "SecretsHandler");
    }

    // =========================================================================
    // Secrets Service Availability
    // =========================================================================

    #[tokio::test]
    async fn test_secrets_not_enabled_error() {
        let ctx = setup_test_context_without_secrets().await;
        let handler = SecretsHandler;

        let request = ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: "pki".to_string(),
            common_name: "Test CA".to_string(),
            ttl_days: Some(365),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::Error(err) => {
                assert_eq!(err.code, "SECRETS_NOT_ENABLED");
            }
            other => panic!("expected Error response, got {:?}", other),
        }
    }

    // =========================================================================
    // PKI Tests
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
                assert!(resp.is_success);
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
                assert!(resp.is_success);
                let role = resp.role.expect("should have role");
                assert_eq!(role.name, "web-servers");
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
                assert!(resp.is_success, "PKI issue failed: {:?}", resp.error);
                assert!(resp.certificate.is_some());
            }
            other => panic!("expected SecretsPkiCertificateResult, got {:?}", other),
        }
    }
}
