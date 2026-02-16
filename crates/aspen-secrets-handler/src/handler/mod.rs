//! Secrets engine request handler.
//!
//! Handles: SecretsKv*, SecretsTransit*, SecretsPki*, SecretsNixCache* operations.
//!
//! Provides Vault-compatible secrets management through:
//! - KV v2: Versioned key-value secrets with soft/hard delete
//! - Transit: Encryption-as-a-service (encrypt, decrypt, sign, verify)
//! - PKI: Certificate authority with role-based issuance
//! - Nix Cache: Signing key management for Nix binary caches
//!
//! Each domain is handled by a dedicated sub-handler struct that owns
//! its routing logic, eliminating duplicate dispatch in a single match.

mod kv;
mod nix_cache;
mod pki;
mod transit;

use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use aspen_secrets::KvStore;
use aspen_secrets::PkiStore;
use aspen_secrets::TransitStore;
use kv::KvSecretsHandler;
use nix_cache::NixCacheSecretsHandler;
use pki::PkiSecretsHandler;
use transit::TransitSecretsHandler;

/// Handler for secrets engine operations.
///
/// Dispatches requests to domain-specific sub-handlers:
/// - `KvSecretsHandler` for KV v2 operations
/// - `TransitSecretsHandler` for encryption-as-a-service
/// - `PkiSecretsHandler` for certificate authority
/// - `NixCacheSecretsHandler` for Nix binary cache signing
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
        // Stateless sub-handler instances for routing checks.
        let kv = KvSecretsHandler;
        let transit = TransitSecretsHandler;
        let pki = PkiSecretsHandler;
        let nix_cache = NixCacheSecretsHandler;

        kv.can_handle(request)
            || transit.can_handle(request)
            || pki.can_handle(request)
            || nix_cache.can_handle(request)
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

        // Dispatch to the appropriate sub-handler.
        let kv = KvSecretsHandler;
        let transit = TransitSecretsHandler;
        let pki = PkiSecretsHandler;
        let nix_cache = NixCacheSecretsHandler;

        if kv.can_handle(&request) {
            return kv.handle(request, &secrets_service, ctx).await;
        }
        if transit.can_handle(&request) {
            return transit.handle(request, &secrets_service, ctx).await;
        }
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
// Error Sanitization
// =============================================================================

/// Sanitize secrets errors for client display.
///
/// Returns user-friendly error messages without leaking sensitive internal details.
/// All error variants are explicitly handled to prevent catch-all masking of
/// actionable errors.
pub(crate) fn sanitize_secrets_error(error: &aspen_secrets::SecretsError) -> String {
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
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use aspen_client_api::ClientRpcRequest;
    use aspen_client_api::ClientRpcResponse;
    use aspen_rpc_core::test_support::TestContextBuilder;

    use super::*;

    // =========================================================================
    // Mock EndpointProvider for Testing
    // =========================================================================

    /// Mock implementation of `EndpointProvider` for testing.
    ///
    /// Creates a real Iroh endpoint for compatibility with handler code.
    struct MockEndpointProvider {
        /// Iroh endpoint for mock network operations.
        endpoint: iroh::Endpoint,
        /// Node address for peer discovery.
        node_addr: iroh::EndpointAddr,
        /// Public key bytes.
        public_key: Vec<u8>,
        /// Peer ID string.
        peer_id: String,
    }

    impl MockEndpointProvider {
        /// Create a new mock endpoint provider.
        async fn new() -> Self {
            // Generate deterministic secret key from seed
            let mut key_bytes = [0u8; 32];
            key_bytes[0..8].copy_from_slice(&0u64.to_le_bytes());
            let secret_key = iroh::SecretKey::from_bytes(&key_bytes);

            // Build endpoint without discovery (isolated)
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

    /// Create a SecretsService with in-memory backends for testing.
    fn make_secrets_service(kv_store: Arc<dyn aspen_core::KeyValueStore>) -> SecretsService {
        let mount_registry = Arc::new(aspen_secrets::MountRegistry::new(kv_store));
        SecretsService::new(mount_registry)
    }

    /// Create a test context with secrets service enabled.
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

    /// Create a test context without secrets service.
    async fn setup_test_context_without_secrets() -> ClientProtocolContext {
        let mock_endpoint: Arc<dyn aspen_core::EndpointProvider> = Arc::new(MockEndpointProvider::new().await);

        let builder = TestContextBuilder::new()
            .with_node_id(1)
            .with_endpoint_manager(mock_endpoint)
            .with_cookie("test_cluster");

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
                assert!(resp.is_success);
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
                assert!(resp.is_success);
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
                assert!(!resp.is_success);
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
                assert!(resp.is_success);
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
                assert!(resp.is_success);
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
                assert!(resp.is_success);
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
                assert!(!resp.is_success);
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
                assert!(resp.is_success);
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
                assert!(resp.is_success);
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
                assert!(resp.is_success);
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
                assert!(resp.is_success);
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
                assert!(resp.is_success);
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
                assert!(!resp.is_success);
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
                assert!(resp.is_success);
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
                assert!(resp.is_success);
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
                assert!(!resp.is_success);
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
                assert!(resp.is_success);
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
                assert!(resp.is_success);
                assert_eq!(resp.is_valid, Some(true));
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
                if resp.is_success {
                    assert_eq!(resp.is_valid, Some(false));
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
                assert!(resp.is_success);
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
                assert!(resp.is_success);
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
                assert!(resp.is_success);
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
                assert!(resp.is_success, "PKI issue failed: {:?}", resp.error);
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
                assert!(!resp.is_success);
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
                assert!(resp.is_success, "PKI issue failed: {:?}", resp.error);
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
                assert!(resp.is_success);
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
                assert!(resp.is_success);
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
                assert!(resp.is_success);
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
