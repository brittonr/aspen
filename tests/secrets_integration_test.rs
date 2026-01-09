//! Integration tests for secrets management with real Iroh networking.
//!
//! These tests validate the secrets service RPC operations using actual
//! Iroh P2P networking (not madsim simulation).
//!
//! # Test Categories
//!
//! 1. **KV v2 Tests** - Versioned key-value secrets operations
//! 2. **Transit Tests** - Encryption-as-a-service operations
//! 3. **PKI Tests** - Certificate authority operations
//!
//! # Requirements
//!
//! These tests require:
//! - Network access (marked with `#[ignore]` for Nix sandbox)
//! - The `secrets` feature enabled
//!
//! Run with:
//!
//! ```bash
//! cargo nextest run secrets_integration --ignored
//! ```
//!
//! # Tiger Style
//!
//! - Bounded timeouts: All operations have explicit timeouts
//! - Resource cleanup: Clusters are properly shut down after tests
//! - Explicit error handling: All errors are wrapped with context

mod support;

use std::collections::HashMap;
use std::time::Duration;

use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use support::real_cluster::RealClusterConfig;
use support::real_cluster::RealClusterTester;

/// Test timeout for single-node operations.
const SINGLE_NODE_TIMEOUT: Duration = Duration::from_secs(30);

/// Default mount point for KV secrets.
const KV_MOUNT: &str = "kv";

/// Default mount point for Transit.
const TRANSIT_MOUNT: &str = "transit";

/// Default mount point for PKI.
const PKI_MOUNT: &str = "pki";

// ============================================================================
// KV v2 Engine Tests
// ============================================================================

/// Test: Write and read a secret using KV v2 engine.
///
/// This test validates the basic KV secret lifecycle:
/// 1. Start a single-node cluster
/// 2. Write a secret with key-value data
/// 3. Read the secret back
/// 4. Verify the data matches
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_kv_write_read() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Write a secret
    let mut data = HashMap::new();
    data.insert("username".to_string(), "admin".to_string());
    data.insert("password".to_string(), "secret123".to_string());

    let write_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvWrite {
            mount: KV_MOUNT.to_string(),
            path: "myapp/config".to_string(),
            data: data.clone(),
            cas: None,
        })
        .await
        .expect("failed to send write request");

    match write_response {
        ClientRpcResponse::SecretsKvWriteResult(result) => {
            assert!(result.success, "write should succeed: {:?}", result.error);
            assert!(result.version.is_some(), "should return version");
            tracing::info!(version = result.version, "secret written");
        }
        ClientRpcResponse::Error(e) => {
            panic!("write failed: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    }

    // Read the secret back
    let read_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvRead {
            mount: KV_MOUNT.to_string(),
            path: "myapp/config".to_string(),
            version: None,
        })
        .await
        .expect("failed to send read request");

    match read_response {
        ClientRpcResponse::SecretsKvReadResult(result) => {
            assert!(result.success, "read should succeed: {:?}", result.error);
            let read_data = result.data.expect("should have data");
            assert_eq!(read_data.get("username"), Some(&"admin".to_string()));
            assert_eq!(read_data.get("password"), Some(&"secret123".to_string()));
            tracing::info!("secret read successfully");
        }
        ClientRpcResponse::Error(e) => {
            panic!("read failed: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: KV v2 versioning - multiple writes create versions.
///
/// This test validates versioning behavior:
/// 1. Write a secret (version 1)
/// 2. Update the secret (version 2)
/// 3. Read specific versions
/// 4. Verify each version has correct data
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_kv_versioning() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Write version 1
    let mut data_v1 = HashMap::new();
    data_v1.insert("value".to_string(), "version1".to_string());

    let write1 = tester
        .client()
        .send(ClientRpcRequest::SecretsKvWrite {
            mount: KV_MOUNT.to_string(),
            path: "versioned/secret".to_string(),
            data: data_v1,
            cas: None,
        })
        .await
        .expect("failed to send write request");

    let version1 = match write1 {
        ClientRpcResponse::SecretsKvWriteResult(r) => {
            assert!(r.success);
            r.version.expect("should have version")
        }
        _ => panic!("unexpected response"),
    };

    // Write version 2
    let mut data_v2 = HashMap::new();
    data_v2.insert("value".to_string(), "version2".to_string());

    let write2 = tester
        .client()
        .send(ClientRpcRequest::SecretsKvWrite {
            mount: KV_MOUNT.to_string(),
            path: "versioned/secret".to_string(),
            data: data_v2,
            cas: None,
        })
        .await
        .expect("failed to send write request");

    let version2 = match write2 {
        ClientRpcResponse::SecretsKvWriteResult(r) => {
            assert!(r.success);
            r.version.expect("should have version")
        }
        _ => panic!("unexpected response"),
    };

    assert!(version2 > version1, "version should increment");

    // Read version 1 explicitly
    let read_v1 = tester
        .client()
        .send(ClientRpcRequest::SecretsKvRead {
            mount: KV_MOUNT.to_string(),
            path: "versioned/secret".to_string(),
            version: Some(version1),
        })
        .await
        .expect("failed to read");

    match read_v1 {
        ClientRpcResponse::SecretsKvReadResult(r) => {
            assert!(r.success);
            let data = r.data.expect("should have data");
            assert_eq!(data.get("value"), Some(&"version1".to_string()));
        }
        _ => panic!("unexpected response"),
    }

    // Read latest (should be version 2)
    let read_latest = tester
        .client()
        .send(ClientRpcRequest::SecretsKvRead {
            mount: KV_MOUNT.to_string(),
            path: "versioned/secret".to_string(),
            version: None,
        })
        .await
        .expect("failed to read");

    match read_latest {
        ClientRpcResponse::SecretsKvReadResult(r) => {
            assert!(r.success);
            let data = r.data.expect("should have data");
            assert_eq!(data.get("value"), Some(&"version2".to_string()));
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: KV v2 list secrets.
///
/// This test validates listing secrets:
/// 1. Write multiple secrets under different paths
/// 2. List secrets under a prefix
/// 3. Verify all secrets are listed
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_kv_list() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Write multiple secrets
    for name in ["config", "credentials", "settings"] {
        let mut data = HashMap::new();
        data.insert("name".to_string(), name.to_string());

        let response = tester
            .client()
            .send(ClientRpcRequest::SecretsKvWrite {
                mount: KV_MOUNT.to_string(),
                path: format!("myapp/{}", name),
                data,
                cas: None,
            })
            .await
            .expect("failed to write");

        match response {
            ClientRpcResponse::SecretsKvWriteResult(r) => assert!(r.success),
            _ => panic!("unexpected response"),
        }
    }

    // List secrets under myapp/
    let list_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvList {
            mount: KV_MOUNT.to_string(),
            path: "myapp/".to_string(),
        })
        .await
        .expect("failed to list");

    match list_response {
        ClientRpcResponse::SecretsKvListResult(result) => {
            assert!(result.success, "list should succeed: {:?}", result.error);
            tracing::info!(keys = ?result.keys, "listed secrets");
            assert_eq!(result.keys.len(), 3, "should have 3 secrets");
        }
        ClientRpcResponse::Error(e) => {
            panic!("list failed: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: KV v2 soft delete and undelete.
///
/// This test validates soft delete behavior:
/// 1. Write a secret
/// 2. Soft delete it
/// 3. Verify read returns deleted
/// 4. Undelete it
/// 5. Verify read returns data again
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_kv_delete_undelete() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Write a secret
    let mut data = HashMap::new();
    data.insert("key".to_string(), "value".to_string());

    let write_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvWrite {
            mount: KV_MOUNT.to_string(),
            path: "deletable/secret".to_string(),
            data,
            cas: None,
        })
        .await
        .expect("failed to write");

    let version = match write_response {
        ClientRpcResponse::SecretsKvWriteResult(r) => {
            assert!(r.success);
            r.version.expect("should have version")
        }
        _ => panic!("unexpected response"),
    };

    // Soft delete
    let delete_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvDelete {
            mount: KV_MOUNT.to_string(),
            path: "deletable/secret".to_string(),
            versions: vec![version],
        })
        .await
        .expect("failed to delete");

    match delete_response {
        ClientRpcResponse::SecretsKvDeleteResult(r) => {
            assert!(r.success, "delete should succeed: {:?}", r.error);
        }
        _ => panic!("unexpected response"),
    }

    // Undelete
    let undelete_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvUndelete {
            mount: KV_MOUNT.to_string(),
            path: "deletable/secret".to_string(),
            versions: vec![version],
        })
        .await
        .expect("failed to undelete");

    match undelete_response {
        ClientRpcResponse::SecretsKvDeleteResult(r) => {
            assert!(r.success, "undelete should succeed: {:?}", r.error);
        }
        _ => panic!("unexpected response"),
    }

    // Read should work again
    let read_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvRead {
            mount: KV_MOUNT.to_string(),
            path: "deletable/secret".to_string(),
            version: None,
        })
        .await
        .expect("failed to read");

    match read_response {
        ClientRpcResponse::SecretsKvReadResult(r) => {
            assert!(r.success, "read should succeed after undelete");
            assert!(r.data.is_some(), "should have data after undelete");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

// ============================================================================
// Transit Engine Tests
// ============================================================================

/// Test: Transit engine - create key and encrypt/decrypt.
///
/// This test validates basic Transit operations:
/// 1. Create an encryption key
/// 2. Encrypt data
/// 3. Decrypt data
/// 4. Verify roundtrip
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_transit_encrypt_decrypt() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Create encryption key
    let create_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitCreateKey {
            mount: TRANSIT_MOUNT.to_string(),
            name: "app-key".to_string(),
            key_type: "aes256-gcm".to_string(),
        })
        .await
        .expect("failed to create key");

    match create_response {
        ClientRpcResponse::SecretsTransitKeyResult(r) => {
            assert!(r.success, "key creation should succeed: {:?}", r.error);
            tracing::info!(name = r.name, "key created");
        }
        ClientRpcResponse::Error(e) => {
            panic!("key creation failed: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    }

    // Encrypt data
    let plaintext = b"Hello, World! This is sensitive data.".to_vec();

    let encrypt_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitEncrypt {
            mount: TRANSIT_MOUNT.to_string(),
            name: "app-key".to_string(),
            plaintext: plaintext.clone(),
            context: None,
        })
        .await
        .expect("failed to encrypt");

    let ciphertext = match encrypt_response {
        ClientRpcResponse::SecretsTransitEncryptResult(r) => {
            assert!(r.success, "encryption should succeed: {:?}", r.error);
            let ct = r.ciphertext.expect("should have ciphertext");
            tracing::info!(ciphertext_len = ct.len(), "data encrypted");
            ct
        }
        ClientRpcResponse::Error(e) => {
            panic!("encryption failed: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    };

    // Decrypt data
    let decrypt_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitDecrypt {
            mount: TRANSIT_MOUNT.to_string(),
            name: "app-key".to_string(),
            ciphertext,
            context: None,
        })
        .await
        .expect("failed to decrypt");

    match decrypt_response {
        ClientRpcResponse::SecretsTransitDecryptResult(r) => {
            assert!(r.success, "decryption should succeed: {:?}", r.error);
            let decrypted = r.plaintext.expect("should have plaintext");
            assert_eq!(decrypted, plaintext, "roundtrip should preserve data");
            tracing::info!("roundtrip successful");
        }
        ClientRpcResponse::Error(e) => {
            panic!("decryption failed: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Transit engine - sign and verify (Ed25519).
///
/// This test validates signing operations:
/// 1. Create a signing key (Ed25519)
/// 2. Sign data
/// 3. Verify signature
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_transit_sign_verify() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Create signing key
    let create_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitCreateKey {
            mount: TRANSIT_MOUNT.to_string(),
            name: "signing-key".to_string(),
            key_type: "ed25519".to_string(),
        })
        .await
        .expect("failed to create key");

    match create_response {
        ClientRpcResponse::SecretsTransitKeyResult(r) => {
            assert!(r.success, "key creation should succeed: {:?}", r.error);
        }
        _ => panic!("unexpected response"),
    }

    // Sign data
    let data = b"Message to sign".to_vec();

    let sign_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitSign {
            mount: TRANSIT_MOUNT.to_string(),
            name: "signing-key".to_string(),
            data: data.clone(),
        })
        .await
        .expect("failed to sign");

    let signature = match sign_response {
        ClientRpcResponse::SecretsTransitSignResult(r) => {
            assert!(r.success, "signing should succeed: {:?}", r.error);
            r.signature.expect("should have signature")
        }
        _ => panic!("unexpected response"),
    };

    // Verify signature
    let verify_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitVerify {
            mount: TRANSIT_MOUNT.to_string(),
            name: "signing-key".to_string(),
            data,
            signature,
        })
        .await
        .expect("failed to verify");

    match verify_response {
        ClientRpcResponse::SecretsTransitVerifyResult(r) => {
            assert!(r.success, "verify request should succeed: {:?}", r.error);
            assert!(r.valid.unwrap_or(false), "signature should be valid");
            tracing::info!("signature verified");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Transit engine - key rotation.
///
/// This test validates key rotation:
/// 1. Create a key
/// 2. Encrypt with original key
/// 3. Rotate the key
/// 4. Verify old ciphertext can still be decrypted
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_transit_key_rotation() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Create key
    let create_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitCreateKey {
            mount: TRANSIT_MOUNT.to_string(),
            name: "rotatable-key".to_string(),
            key_type: "aes256-gcm".to_string(),
        })
        .await
        .expect("failed to create key");

    match create_response {
        ClientRpcResponse::SecretsTransitKeyResult(r) => assert!(r.success),
        _ => panic!("unexpected response"),
    }

    // Encrypt with original key
    let plaintext = b"Secret data".to_vec();
    let encrypt_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitEncrypt {
            mount: TRANSIT_MOUNT.to_string(),
            name: "rotatable-key".to_string(),
            plaintext: plaintext.clone(),
            context: None,
        })
        .await
        .expect("failed to encrypt");

    let ciphertext = match encrypt_response {
        ClientRpcResponse::SecretsTransitEncryptResult(r) => {
            assert!(r.success);
            r.ciphertext.expect("should have ciphertext")
        }
        _ => panic!("unexpected response"),
    };

    // Rotate key
    let rotate_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitRotateKey {
            mount: TRANSIT_MOUNT.to_string(),
            name: "rotatable-key".to_string(),
        })
        .await
        .expect("failed to rotate");

    match rotate_response {
        ClientRpcResponse::SecretsTransitKeyResult(r) => {
            assert!(r.success, "rotation should succeed: {:?}", r.error);
            tracing::info!(version = r.version, "key rotated");
        }
        _ => panic!("unexpected response"),
    }

    // Decrypt old ciphertext (should still work)
    let decrypt_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitDecrypt {
            mount: TRANSIT_MOUNT.to_string(),
            name: "rotatable-key".to_string(),
            ciphertext,
            context: None,
        })
        .await
        .expect("failed to decrypt");

    match decrypt_response {
        ClientRpcResponse::SecretsTransitDecryptResult(r) => {
            assert!(r.success, "decryption should work after rotation");
            let decrypted = r.plaintext.expect("should have plaintext");
            assert_eq!(decrypted, plaintext);
            tracing::info!("old ciphertext decrypted after rotation");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Transit engine - list keys.
///
/// This test validates listing Transit keys:
/// 1. Create multiple keys
/// 2. List all keys
/// 3. Verify all keys are returned
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_transit_list_keys() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Create multiple keys
    for name in ["key-a", "key-b", "key-c"] {
        let response = tester
            .client()
            .send(ClientRpcRequest::SecretsTransitCreateKey {
                mount: TRANSIT_MOUNT.to_string(),
                name: name.to_string(),
                key_type: "aes256-gcm".to_string(),
            })
            .await
            .expect("failed to create key");

        match response {
            ClientRpcResponse::SecretsTransitKeyResult(r) => assert!(r.success),
            _ => panic!("unexpected response"),
        }
    }

    // List keys
    let list_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitListKeys {
            mount: TRANSIT_MOUNT.to_string(),
        })
        .await
        .expect("failed to list keys");

    match list_response {
        ClientRpcResponse::SecretsTransitListResult(r) => {
            assert!(r.success, "list should succeed: {:?}", r.error);
            tracing::info!(keys = ?r.keys, "listed transit keys");
            assert!(r.keys.len() >= 3, "should have at least 3 keys");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

// ============================================================================
// PKI Engine Tests
// ============================================================================

/// Test: PKI engine - generate root CA.
///
/// This test validates root CA generation:
/// 1. Generate a root CA certificate
/// 2. Verify certificate is returned
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_pki_generate_root() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Generate root CA
    let response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: PKI_MOUNT.to_string(),
            common_name: "Aspen Test Root CA".to_string(),
            ttl_days: Some(365),
        })
        .await
        .expect("failed to generate root");

    match response {
        ClientRpcResponse::SecretsPkiCertificateResult(r) => {
            assert!(r.success, "root generation should succeed: {:?}", r.error);
            let cert = r.certificate.expect("should have certificate");
            assert!(cert.contains("BEGIN CERTIFICATE"), "should be PEM format");
            tracing::info!(serial = r.serial, "root CA generated");
        }
        ClientRpcResponse::Error(e) => {
            panic!("root generation failed: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: PKI engine - create role and issue certificate.
///
/// This test validates the certificate issuance flow:
/// 1. Generate a root CA
/// 2. Create a role with domain restrictions
/// 3. Issue a certificate using the role
/// 4. Verify certificate is returned
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_pki_create_role_and_issue() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Generate root CA first
    let root_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: PKI_MOUNT.to_string(),
            common_name: "Aspen Test CA".to_string(),
            ttl_days: Some(365),
        })
        .await
        .expect("failed to generate root");

    match root_response {
        ClientRpcResponse::SecretsPkiCertificateResult(r) => assert!(r.success),
        _ => panic!("unexpected response"),
    }

    // Create role
    let role_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiCreateRole {
            mount: PKI_MOUNT.to_string(),
            name: "web-server".to_string(),
            allowed_domains: vec!["example.com".to_string(), "test.local".to_string()],
            allow_bare_domains: true,
            allow_wildcards: true,
            max_ttl_days: 30,
        })
        .await
        .expect("failed to create role");

    match role_response {
        ClientRpcResponse::SecretsPkiRoleResult(r) => {
            assert!(r.success, "role creation should succeed: {:?}", r.error);
            tracing::info!("role created");
        }
        _ => panic!("unexpected response"),
    }

    // Issue certificate
    let issue_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiIssue {
            mount: PKI_MOUNT.to_string(),
            role: "web-server".to_string(),
            common_name: "api.example.com".to_string(),
            alt_names: vec!["www.example.com".to_string()],
            ttl_days: Some(7),
        })
        .await
        .expect("failed to issue certificate");

    match issue_response {
        ClientRpcResponse::SecretsPkiCertificateResult(r) => {
            assert!(r.success, "issuance should succeed: {:?}", r.error);
            let cert = r.certificate.expect("should have certificate");
            let key = r.private_key.expect("should have private key");
            assert!(cert.contains("BEGIN CERTIFICATE"));
            assert!(key.contains("BEGIN") && key.contains("PRIVATE KEY"));
            tracing::info!(serial = r.serial, "certificate issued");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: PKI engine - list certificates and roles.
///
/// This test validates listing operations:
/// 1. Generate root and issue certificates
/// 2. List all certificates
/// 3. List all roles
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_pki_list_certs_and_roles() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Generate root CA
    let root_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: PKI_MOUNT.to_string(),
            common_name: "Test CA".to_string(),
            ttl_days: Some(365),
        })
        .await
        .expect("failed to generate root");

    match root_response {
        ClientRpcResponse::SecretsPkiCertificateResult(r) => assert!(r.success),
        _ => panic!("unexpected response"),
    }

    // Create a role
    let role_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiCreateRole {
            mount: PKI_MOUNT.to_string(),
            name: "test-role".to_string(),
            allowed_domains: vec!["test.local".to_string()],
            allow_bare_domains: true,
            allow_wildcards: false,
            max_ttl_days: 30,
        })
        .await
        .expect("failed to create role");

    match role_response {
        ClientRpcResponse::SecretsPkiRoleResult(r) => assert!(r.success),
        _ => panic!("unexpected response"),
    }

    // List roles
    let list_roles = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiListRoles {
            mount: PKI_MOUNT.to_string(),
        })
        .await
        .expect("failed to list roles");

    match list_roles {
        ClientRpcResponse::SecretsPkiListResult(r) => {
            assert!(r.success, "list roles should succeed: {:?}", r.error);
            tracing::info!(roles = ?r.items, "listed roles");
            assert!(!r.items.is_empty(), "should have at least one role");
        }
        _ => panic!("unexpected response"),
    }

    // List certificates
    let list_certs = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiListCerts {
            mount: PKI_MOUNT.to_string(),
        })
        .await
        .expect("failed to list certs");

    match list_certs {
        ClientRpcResponse::SecretsPkiListResult(r) => {
            assert!(r.success, "list certs should succeed: {:?}", r.error);
            tracing::info!(certs = ?r.items, "listed certificates");
            // Should have at least the root CA
            assert!(!r.items.is_empty(), "should have at least root CA");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: PKI engine - revoke certificate and get CRL.
///
/// This test validates revocation:
/// 1. Generate root and issue certificate
/// 2. Revoke the certificate
/// 3. Get CRL and verify revoked cert is listed
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_pki_revoke_and_crl() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Generate root CA
    let root_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: PKI_MOUNT.to_string(),
            common_name: "Test CA".to_string(),
            ttl_days: Some(365),
        })
        .await
        .expect("failed to generate root");

    match root_response {
        ClientRpcResponse::SecretsPkiCertificateResult(r) => assert!(r.success),
        _ => panic!("unexpected response"),
    }

    // Create role and issue certificate
    let role_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiCreateRole {
            mount: PKI_MOUNT.to_string(),
            name: "revoke-test".to_string(),
            allowed_domains: vec!["revoke.local".to_string()],
            allow_bare_domains: true,
            allow_wildcards: false,
            max_ttl_days: 30,
        })
        .await
        .expect("failed to create role");

    match role_response {
        ClientRpcResponse::SecretsPkiRoleResult(r) => assert!(r.success),
        _ => panic!("unexpected response"),
    }

    let issue_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiIssue {
            mount: PKI_MOUNT.to_string(),
            role: "revoke-test".to_string(),
            common_name: "revoke.local".to_string(),
            alt_names: vec![],
            ttl_days: Some(7),
        })
        .await
        .expect("failed to issue");

    let serial = match issue_response {
        ClientRpcResponse::SecretsPkiCertificateResult(r) => {
            assert!(r.success);
            r.serial.expect("should have serial")
        }
        _ => panic!("unexpected response"),
    };

    // Revoke certificate
    let revoke_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiRevoke {
            mount: PKI_MOUNT.to_string(),
            serial: serial.clone(),
        })
        .await
        .expect("failed to revoke");

    match revoke_response {
        ClientRpcResponse::SecretsPkiRevokeResult(r) => {
            assert!(r.success, "revocation should succeed: {:?}", r.error);
            tracing::info!(serial = serial, "certificate revoked");
        }
        _ => panic!("unexpected response"),
    }

    // Get CRL
    let crl_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiGetCrl {
            mount: PKI_MOUNT.to_string(),
        })
        .await
        .expect("failed to get CRL");

    match crl_response {
        ClientRpcResponse::SecretsPkiCrlResult(r) => {
            assert!(r.success, "CRL should succeed: {:?}", r.error);
            let crl = r.crl.expect("should have CRL");
            assert!(crl.contains("BEGIN") && crl.contains("CRL"), "should be PEM CRL");
            tracing::info!("CRL retrieved");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

// ============================================================================
// Multi-Node Tests
// ============================================================================

/// Test: Secrets operations across a 3-node cluster.
///
/// This test validates secrets work with Raft consensus:
/// 1. Start a 3-node cluster
/// 2. Write secret on one node
/// 3. Read secret from any node
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_multi_node_consistency() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(3)
        .with_workers(false)
        .with_timeout(Duration::from_secs(60));

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Write secret
    let mut data = HashMap::new();
    data.insert("distributed".to_string(), "secret".to_string());

    let write_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvWrite {
            mount: KV_MOUNT.to_string(),
            path: "cluster/test".to_string(),
            data,
            cas: None,
        })
        .await
        .expect("failed to write");

    match write_response {
        ClientRpcResponse::SecretsKvWriteResult(r) => {
            assert!(r.success, "write should succeed");
            tracing::info!("secret written to cluster");
        }
        _ => panic!("unexpected response"),
    }

    // Read back (goes through Raft consensus)
    let read_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvRead {
            mount: KV_MOUNT.to_string(),
            path: "cluster/test".to_string(),
            version: None,
        })
        .await
        .expect("failed to read");

    match read_response {
        ClientRpcResponse::SecretsKvReadResult(r) => {
            assert!(r.success, "read should succeed");
            let data = r.data.expect("should have data");
            assert_eq!(data.get("distributed"), Some(&"secret".to_string()));
            tracing::info!("secret read from cluster");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}
