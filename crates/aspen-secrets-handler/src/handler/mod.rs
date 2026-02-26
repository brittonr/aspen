//! Secrets engine request handler (native subset).
//!
//! Handles: SecretsPki*, SecretsNixCache* operations only.
//!
//! KV and Transit secrets operations have been migrated to the
//! `aspen-secrets-plugin` WASM plugin. This native handler retains
//! PKI (X.509 certificate authority) and Nix cache signing because
//! they depend on native crypto libraries (rcgen, ed25519) that
//! cannot yet run inside the WASM sandbox.

pub(crate) mod nix_cache;
pub(crate) mod pki;

use std::sync::Arc;

use aspen_secrets::PkiStore;

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

    use super::*;

    fn make_secrets_service(kv_store: Arc<dyn aspen_core::KeyValueStore>) -> SecretsService {
        let mount_registry = Arc::new(aspen_secrets::MountRegistry::new(kv_store));
        SecretsService::new(mount_registry)
    }

    async fn setup_test_executor() -> crate::SecretsServiceExecutor {
        use aspen_testing::DeterministicKeyValueStore;

        let kv_store: Arc<dyn aspen_core::KeyValueStore> = Arc::new(DeterministicKeyValueStore::new());
        let secrets_service = Arc::new(make_secrets_service(Arc::clone(&kv_store)));

        crate::SecretsServiceExecutor::new(secrets_service, kv_store)
    }

    // =========================================================================
    // Executor Tests
    // =========================================================================

    #[tokio::test]
    async fn test_executor_handles_pki_requests() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;

        // Check that the executor handles PKI requests
        assert!(executor.handles().contains(&"SecretsPkiGenerateRoot"));
        assert!(executor.handles().contains(&"SecretsPkiIssue"));
        assert!(executor.handles().contains(&"SecretsPkiCreateRole"));
    }

    #[tokio::test]
    async fn test_executor_handles_nix_cache_requests() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;

        // Check that the executor handles Nix Cache requests
        assert!(executor.handles().contains(&"SecretsNixCacheCreateKey"));
        assert!(executor.handles().contains(&"SecretsNixCacheGetPublicKey"));
        assert!(executor.handles().contains(&"SecretsNixCacheListKeys"));
    }

    #[tokio::test]
    async fn test_executor_service_name() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;
        assert_eq!(executor.service_name(), "secrets");
    }

    #[tokio::test]
    async fn test_executor_priority() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;
        assert_eq!(executor.priority(), 580);
    }

    #[tokio::test]
    async fn test_executor_app_id() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;
        assert_eq!(executor.app_id(), Some("secrets"));
    }

    // =========================================================================
    // PKI Tests
    // =========================================================================

    #[tokio::test]
    async fn test_pki_generate_root() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;

        let request = ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: "pki".to_string(),
            common_name: "Test Root CA".to_string(),
            ttl_days: Some(365),
        };

        let result = executor.execute(request).await;
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
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;

        // Generate root CA first
        let gen_root = ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: "pki".to_string(),
            common_name: "Test CA".to_string(),
            ttl_days: Some(365),
        };
        let _ = executor.execute(gen_root).await.unwrap();

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

        let result = executor.execute(create_role).await;
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

        let result = executor.execute(issue).await;
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
