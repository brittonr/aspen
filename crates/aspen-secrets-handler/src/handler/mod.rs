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
use aspen_secrets::SecretsMountProvider;

/// Secrets service with multi-mount support.
///
/// Uses a mount provider to dynamically resolve store instances per mount
/// point. Each mount has its own isolated storage while this service remains
/// decoupled from the concrete registry/cache implementation.
pub struct SecretsService {
    /// Mount provider for dynamic store management.
    mounts: Arc<dyn SecretsMountProvider>,
}

impl SecretsService {
    /// Create a new secrets service with the given mount provider.
    pub fn new(mounts: Arc<dyn SecretsMountProvider>) -> Self {
        Self { mounts }
    }

    /// Get or create a PKI store for the given mount point.
    ///
    /// # Errors
    ///
    /// Returns an error if the mount name is invalid or too many mounts exist.
    pub async fn get_pki_store(&self, mount: &str) -> anyhow::Result<Arc<dyn PkiStore>> {
        self.mounts
            .pki_store(mount)
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
        self.mounts
            .kv_store(mount)
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
        self.mounts
            .transit_store(mount)
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

    fn make_secrets_service(kv_store: Arc<dyn aspen_traits::KeyValueStore>) -> SecretsService {
        let mount_registry = Arc::new(aspen_secrets::MountRegistry::new(kv_store));
        SecretsService::new(mount_registry)
    }

    async fn setup_test_executor() -> crate::SecretsServiceExecutor {
        use aspen_testing::DeterministicKeyValueStore;

        let kv_store: Arc<dyn aspen_traits::KeyValueStore> = Arc::new(DeterministicKeyValueStore::new());
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

    // =========================================================================
    // PKI Revoke / CRL / List Tests
    // =========================================================================

    /// Helper: set up a PKI mount with root CA, role, and issued cert.
    /// Returns the serial of the issued certificate.
    async fn setup_pki_with_cert() -> (crate::SecretsServiceExecutor, String) {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;

        // Generate root CA
        let _ = executor
            .execute(ClientRpcRequest::SecretsPkiGenerateRoot {
                mount: "pki".to_string(),
                common_name: "Test CA".to_string(),
                ttl_days: Some(365),
            })
            .await
            .unwrap();

        // Create role
        let _ = executor
            .execute(ClientRpcRequest::SecretsPkiCreateRole {
                mount: "pki".to_string(),
                name: "servers".to_string(),
                allowed_domains: vec!["example.com".to_string()],
                max_ttl_days: 90,
                allow_bare_domains: true,
                allow_wildcards: false,
                allow_subdomains: false,
            })
            .await
            .unwrap();

        // Issue cert
        let resp = executor
            .execute(ClientRpcRequest::SecretsPkiIssue {
                mount: "pki".to_string(),
                role: "servers".to_string(),
                common_name: "example.com".to_string(),
                alt_names: vec![],
                ttl_days: Some(30),
            })
            .await
            .unwrap();

        let serial = match resp {
            ClientRpcResponse::SecretsPkiCertificateResult(r) => {
                assert!(r.is_success);
                r.serial.expect("issued cert must have serial")
            }
            other => panic!("expected SecretsPkiCertificateResult, got {:?}", other),
        };

        (executor, serial)
    }

    #[tokio::test]
    async fn test_pki_revoke_cert() {
        use aspen_rpc_core::ServiceExecutor;

        let (executor, serial) = setup_pki_with_cert().await;

        let result = executor
            .execute(ClientRpcRequest::SecretsPkiRevoke {
                mount: "pki".to_string(),
                serial: serial.clone(),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsPkiRevokeResult(resp) => {
                assert!(resp.is_success);
                assert_eq!(resp.serial, Some(serial));
            }
            other => panic!("expected SecretsPkiRevokeResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pki_revoke_nonexistent_cert() {
        use aspen_rpc_core::ServiceExecutor;

        let (executor, _serial) = setup_pki_with_cert().await;

        let result = executor
            .execute(ClientRpcRequest::SecretsPkiRevoke {
                mount: "pki".to_string(),
                serial: "nonexistent-serial".to_string(),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsPkiRevokeResult(resp) => {
                assert!(!resp.is_success);
                assert!(resp.error.is_some());
            }
            other => panic!("expected SecretsPkiRevokeResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pki_get_crl() {
        use aspen_rpc_core::ServiceExecutor;

        let (executor, _serial) = setup_pki_with_cert().await;

        let result = executor
            .execute(ClientRpcRequest::SecretsPkiGetCrl {
                mount: "pki".to_string(),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsPkiCrlResult(resp) => {
                assert!(resp.is_success);
                assert!(resp.crl.is_some());
            }
            other => panic!("expected SecretsPkiCrlResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pki_list_certs() {
        use aspen_rpc_core::ServiceExecutor;

        let (executor, serial) = setup_pki_with_cert().await;

        let result = executor
            .execute(ClientRpcRequest::SecretsPkiListCerts {
                mount: "pki".to_string(),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsPkiListResult(resp) => {
                assert!(resp.is_success);
                // Should contain at least the issued cert
                assert!(!resp.items.is_empty(), "expected at least 1 cert in list, got 0");
                assert!(resp.items.contains(&serial), "issued cert serial not in list");
            }
            other => panic!("expected SecretsPkiListResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pki_get_role() {
        use aspen_rpc_core::ServiceExecutor;

        let (executor, _serial) = setup_pki_with_cert().await;

        let result = executor
            .execute(ClientRpcRequest::SecretsPkiGetRole {
                mount: "pki".to_string(),
                name: "servers".to_string(),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsPkiRoleResult(resp) => {
                assert!(resp.is_success);
                let role = resp.role.expect("should have role");
                assert_eq!(role.name, "servers");
                assert_eq!(role.allowed_domains, vec!["example.com".to_string()]);
                assert!(role.allow_bare_domains);
            }
            other => panic!("expected SecretsPkiRoleResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pki_get_role_not_found() {
        use aspen_rpc_core::ServiceExecutor;

        let (executor, _serial) = setup_pki_with_cert().await;

        let result = executor
            .execute(ClientRpcRequest::SecretsPkiGetRole {
                mount: "pki".to_string(),
                name: "nonexistent-role".to_string(),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsPkiRoleResult(resp) => {
                assert!(!resp.is_success);
                assert!(resp.role.is_none());
                assert!(resp.error.is_some());
            }
            other => panic!("expected SecretsPkiRoleResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pki_list_roles() {
        use aspen_rpc_core::ServiceExecutor;

        let (executor, _serial) = setup_pki_with_cert().await;

        let result = executor
            .execute(ClientRpcRequest::SecretsPkiListRoles {
                mount: "pki".to_string(),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsPkiListResult(resp) => {
                assert!(resp.is_success);
                assert!(resp.items.contains(&"servers".to_string()));
            }
            other => panic!("expected SecretsPkiListResult, got {:?}", other),
        }
    }

    // =========================================================================
    // PKI Error Path Tests
    // =========================================================================

    #[tokio::test]
    async fn test_pki_issue_without_ca_init() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;

        // Create role without generating root CA first
        let _result = executor
            .execute(ClientRpcRequest::SecretsPkiCreateRole {
                mount: "pki".to_string(),
                name: "servers".to_string(),
                allowed_domains: vec!["example.com".to_string()],
                max_ttl_days: 90,
                allow_bare_domains: true,
                allow_wildcards: false,
                allow_subdomains: false,
            })
            .await
            .unwrap();

        // Role creation may succeed even without CA, but issue should fail
        // Try issuing without CA
        let issue_result = executor
            .execute(ClientRpcRequest::SecretsPkiIssue {
                mount: "pki".to_string(),
                role: "servers".to_string(),
                common_name: "example.com".to_string(),
                alt_names: vec![],
                ttl_days: Some(30),
            })
            .await
            .unwrap();

        match issue_result {
            ClientRpcResponse::SecretsPkiCertificateResult(resp) => {
                assert!(!resp.is_success, "issue should fail without CA");
                assert!(resp.error.is_some());
            }
            other => panic!("expected SecretsPkiCertificateResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pki_issue_with_disallowed_cn() {
        use aspen_rpc_core::ServiceExecutor;

        let (executor, _serial) = setup_pki_with_cert().await;

        // Try issuing cert for a domain not in allowed_domains
        let result = executor
            .execute(ClientRpcRequest::SecretsPkiIssue {
                mount: "pki".to_string(),
                role: "servers".to_string(),
                common_name: "evil.com".to_string(),
                alt_names: vec![],
                ttl_days: Some(30),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsPkiCertificateResult(resp) => {
                assert!(!resp.is_success, "issue should fail for disallowed CN");
                assert!(resp.error.is_some());
            }
            other => panic!("expected SecretsPkiCertificateResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pki_issue_for_nonexistent_role() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;

        // Generate root CA
        let _ = executor
            .execute(ClientRpcRequest::SecretsPkiGenerateRoot {
                mount: "pki".to_string(),
                common_name: "Test CA".to_string(),
                ttl_days: Some(365),
            })
            .await
            .unwrap();

        let result = executor
            .execute(ClientRpcRequest::SecretsPkiIssue {
                mount: "pki".to_string(),
                role: "does-not-exist".to_string(),
                common_name: "example.com".to_string(),
                alt_names: vec![],
                ttl_days: Some(30),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsPkiCertificateResult(resp) => {
                assert!(!resp.is_success);
                assert!(resp.error.is_some());
            }
            other => panic!("expected SecretsPkiCertificateResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pki_duplicate_root_ca() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;

        // First root CA generation
        let _ = executor
            .execute(ClientRpcRequest::SecretsPkiGenerateRoot {
                mount: "pki".to_string(),
                common_name: "Test CA".to_string(),
                ttl_days: Some(365),
            })
            .await
            .unwrap();

        // Second root CA generation on same mount should fail
        let result = executor
            .execute(ClientRpcRequest::SecretsPkiGenerateRoot {
                mount: "pki".to_string(),
                common_name: "Another CA".to_string(),
                ttl_days: Some(365),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsPkiCertificateResult(resp) => {
                assert!(!resp.is_success, "duplicate root CA should fail");
                assert!(resp.error.is_some());
            }
            other => panic!("expected SecretsPkiCertificateResult, got {:?}", other),
        }
    }

    // =========================================================================
    // Nix Cache Tests
    // =========================================================================

    #[tokio::test]
    async fn test_nix_cache_create_key() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;

        let result = executor
            .execute(ClientRpcRequest::SecretsNixCacheCreateKey {
                mount: "transit".to_string(),
                cache_name: "my-cache".to_string(),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsNixCacheKeyResult(resp) => {
                assert!(resp.is_success, "create key failed: {:?}", resp.error);
                assert!(resp.public_key.is_some());
                let pk = resp.public_key.unwrap();
                assert!(pk.starts_with("my-cache:"), "public key should be prefixed with cache name, got: {pk}");
            }
            other => panic!("expected SecretsNixCacheKeyResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nix_cache_get_public_key() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;

        // Create key first
        let _ = executor
            .execute(ClientRpcRequest::SecretsNixCacheCreateKey {
                mount: "transit".to_string(),
                cache_name: "test-cache".to_string(),
            })
            .await
            .unwrap();

        // Get public key
        let result = executor
            .execute(ClientRpcRequest::SecretsNixCacheGetPublicKey {
                mount: "transit".to_string(),
                cache_name: "test-cache".to_string(),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsNixCacheKeyResult(resp) => {
                assert!(resp.is_success, "get key failed: {:?}", resp.error);
                assert!(resp.public_key.is_some());
            }
            other => panic!("expected SecretsNixCacheKeyResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nix_cache_get_nonexistent_key() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;

        let result = executor
            .execute(ClientRpcRequest::SecretsNixCacheGetPublicKey {
                mount: "transit".to_string(),
                cache_name: "no-such-cache".to_string(),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsNixCacheKeyResult(resp) => {
                assert!(!resp.is_success);
                assert!(resp.error.is_some());
            }
            other => panic!("expected SecretsNixCacheKeyResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nix_cache_list_keys() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;

        // Create two keys
        let _ = executor
            .execute(ClientRpcRequest::SecretsNixCacheCreateKey {
                mount: "transit".to_string(),
                cache_name: "cache-a".to_string(),
            })
            .await
            .unwrap();
        let _ = executor
            .execute(ClientRpcRequest::SecretsNixCacheCreateKey {
                mount: "transit".to_string(),
                cache_name: "cache-b".to_string(),
            })
            .await
            .unwrap();

        let result = executor
            .execute(ClientRpcRequest::SecretsNixCacheListKeys {
                mount: "transit".to_string(),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsNixCacheListResult(resp) => {
                assert!(resp.is_success, "list keys failed: {:?}", resp.error);
                let names = resp.cache_names.expect("should have cache names");
                assert!(names.contains(&"cache-a".to_string()));
                assert!(names.contains(&"cache-b".to_string()));
            }
            other => panic!("expected SecretsNixCacheListResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nix_cache_delete_key() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;

        // Create key
        let _ = executor
            .execute(ClientRpcRequest::SecretsNixCacheCreateKey {
                mount: "transit".to_string(),
                cache_name: "to-delete".to_string(),
            })
            .await
            .unwrap();

        // Delete key
        let result = executor
            .execute(ClientRpcRequest::SecretsNixCacheDeleteKey {
                mount: "transit".to_string(),
                cache_name: "to-delete".to_string(),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsNixCacheDeleteResult(resp) => {
                assert!(resp.is_success, "delete key failed: {:?}", resp.error);
            }
            other => panic!("expected SecretsNixCacheDeleteResult, got {:?}", other),
        }

        // Verify it's gone
        let get_result = executor
            .execute(ClientRpcRequest::SecretsNixCacheGetPublicKey {
                mount: "transit".to_string(),
                cache_name: "to-delete".to_string(),
            })
            .await
            .unwrap();

        match get_result {
            ClientRpcResponse::SecretsNixCacheKeyResult(resp) => {
                assert!(!resp.is_success, "deleted key should not be retrievable");
            }
            other => panic!("expected SecretsNixCacheKeyResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nix_cache_rotate_key() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;

        // Create key
        let create_result = executor
            .execute(ClientRpcRequest::SecretsNixCacheCreateKey {
                mount: "transit".to_string(),
                cache_name: "rotatable".to_string(),
            })
            .await
            .unwrap();

        let original_pk = match &create_result {
            ClientRpcResponse::SecretsNixCacheKeyResult(r) => r.public_key.clone().unwrap(),
            other => panic!("expected SecretsNixCacheKeyResult, got {:?}", other),
        };

        // Rotate key
        let rotate_result = executor
            .execute(ClientRpcRequest::SecretsNixCacheRotateKey {
                mount: "transit".to_string(),
                cache_name: "rotatable".to_string(),
            })
            .await
            .unwrap();

        match rotate_result {
            ClientRpcResponse::SecretsNixCacheKeyResult(resp) => {
                assert!(resp.is_success, "rotate key failed: {:?}", resp.error);
                let rotated_pk = resp.public_key.unwrap();
                // After rotation, the public key should change
                assert_ne!(original_pk, rotated_pk, "rotated key should differ from original");
            }
            other => panic!("expected SecretsNixCacheKeyResult, got {:?}", other),
        }
    }

    // =========================================================================
    // Error Sanitization Tests
    // =========================================================================

    #[test]
    fn test_sanitize_ca_not_initialized() {
        let msg = sanitize_secrets_error(&aspen_secrets::SecretsError::CaNotInitialized);
        assert!(msg.contains("not initialized"), "got: {msg}");
        assert!(msg.contains("generate-root"), "should suggest fix, got: {msg}");
    }

    #[test]
    fn test_sanitize_role_not_found() {
        let msg = sanitize_secrets_error(&aspen_secrets::SecretsError::RoleNotFound {
            name: "web".to_string(),
        });
        assert!(msg.contains("web"), "should include role name, got: {msg}");
    }

    #[test]
    fn test_sanitize_common_name_not_allowed() {
        let msg = sanitize_secrets_error(&aspen_secrets::SecretsError::CommonNameNotAllowed {
            cn: "evil.com".to_string(),
            role: "servers".to_string(),
        });
        assert!(msg.contains("evil.com"), "got: {msg}");
        assert!(msg.contains("servers"), "got: {msg}");
    }

    #[test]
    fn test_sanitize_ttl_exceeds_max() {
        let msg = sanitize_secrets_error(&aspen_secrets::SecretsError::TtlExceedsMax {
            role: "servers".to_string(),
            requested_secs: 99999,
            max_secs: 7776000,
        });
        assert!(msg.contains("99999"), "got: {msg}");
        assert!(msg.contains("7776000"), "got: {msg}");
    }

    #[test]
    fn test_sanitize_mount_not_found() {
        let msg = sanitize_secrets_error(&aspen_secrets::SecretsError::MountNotFound {
            name: "missing".to_string(),
        });
        assert!(msg.contains("missing"), "got: {msg}");
    }

    #[test]
    fn test_sanitize_internal_error_is_generic() {
        let msg = sanitize_secrets_error(&aspen_secrets::SecretsError::Internal {
            reason: "super secret stack trace".to_string(),
        });
        // Internal errors should NOT leak details
        assert!(!msg.contains("super secret"), "internal details should be hidden, got: {msg}");
        assert_eq!(msg, "Internal secrets error");
    }

    // =========================================================================
    // Factory Tests
    // =========================================================================

    #[cfg(feature = "runtime-adapter")]
    #[test]
    fn test_factory_name_and_priority() {
        use aspen_rpc_core::HandlerFactory;

        let factory = super::super::SecretsHandlerFactory::new();
        assert_eq!(factory.name(), "SecretsHandler");
        assert_eq!(factory.priority(), 580);
        assert_eq!(factory.app_id(), Some("secrets"));
    }

    // =========================================================================
    // Mount Isolation Tests
    // =========================================================================

    #[tokio::test]
    async fn test_pki_separate_mounts_are_isolated() {
        use aspen_rpc_core::ServiceExecutor;

        let executor = setup_test_executor().await;

        // Generate root CA on mount "pki-a"
        let _ = executor
            .execute(ClientRpcRequest::SecretsPkiGenerateRoot {
                mount: "pki-a".to_string(),
                common_name: "CA-A".to_string(),
                ttl_days: Some(365),
            })
            .await
            .unwrap();

        // Generate root CA on mount "pki-b"
        let _ = executor
            .execute(ClientRpcRequest::SecretsPkiGenerateRoot {
                mount: "pki-b".to_string(),
                common_name: "CA-B".to_string(),
                ttl_days: Some(365),
            })
            .await
            .unwrap();

        // Roles on pki-a should not be visible on pki-b
        let _ = executor
            .execute(ClientRpcRequest::SecretsPkiCreateRole {
                mount: "pki-a".to_string(),
                name: "role-only-on-a".to_string(),
                allowed_domains: vec!["a.com".to_string()],
                max_ttl_days: 30,
                allow_bare_domains: true,
                allow_wildcards: false,
                allow_subdomains: false,
            })
            .await
            .unwrap();

        let result = executor
            .execute(ClientRpcRequest::SecretsPkiGetRole {
                mount: "pki-b".to_string(),
                name: "role-only-on-a".to_string(),
            })
            .await
            .unwrap();

        match result {
            ClientRpcResponse::SecretsPkiRoleResult(resp) => {
                assert!(!resp.is_success, "role from mount pki-a should not exist on pki-b");
            }
            other => panic!("expected SecretsPkiRoleResult, got {:?}", other),
        }
    }
}
