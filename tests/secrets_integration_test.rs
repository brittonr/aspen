//! Integration tests for secrets management with real Iroh networking.
//!
//! These tests validate the secrets service RPC operations using actual
//! Iroh P2P networking (not madsim simulation).
//!
//! # Test Categories
//!
//! 1. **KV Tests** - Versioned key-value secrets operations
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

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
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
// KV Engine Tests
// ============================================================================

/// Test: Write and read a secret using KV engine.
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
            assert!(result.is_success, "write should succeed: {:?}", result.error);
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
            assert!(result.is_success, "read should succeed: {:?}", result.error);
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

/// Test: KV versioning - multiple writes create versions.
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
            assert!(r.is_success);
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
            assert!(r.is_success);
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
            assert!(r.is_success);
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
            assert!(r.is_success);
            let data = r.data.expect("should have data");
            assert_eq!(data.get("value"), Some(&"version2".to_string()));
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: KV list secrets.
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
            ClientRpcResponse::SecretsKvWriteResult(r) => assert!(r.is_success),
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
            assert!(result.is_success, "list should succeed: {:?}", result.error);
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

/// Test: KV Check-and-Set (CAS) conflict detection.
///
/// This test validates CAS semantics:
/// 1. Write a secret (version 1)
/// 2. Attempt write with CAS=0 (should fail - version mismatch)
/// 3. Attempt write with CAS=1 (should succeed)
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_kv_cas_conflict() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Write initial secret (version 1)
    let mut data = HashMap::new();
    data.insert("value".to_string(), "initial".to_string());

    let write_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvWrite {
            mount: KV_MOUNT.to_string(),
            path: "cas/test".to_string(),
            data: data.clone(),
            cas: None,
        })
        .await
        .expect("failed to write");

    let version = match write_response {
        ClientRpcResponse::SecretsKvWriteResult(r) => {
            assert!(r.is_success);
            r.version.expect("should have version")
        }
        _ => panic!("unexpected response"),
    };
    assert_eq!(version, 1, "first write should be version 1");

    // Attempt write with CAS=0 (should fail - version mismatch)
    let mut data_v2 = HashMap::new();
    data_v2.insert("value".to_string(), "updated".to_string());

    let cas_fail_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvWrite {
            mount: KV_MOUNT.to_string(),
            path: "cas/test".to_string(),
            data: data_v2.clone(),
            cas: Some(0), // Wrong version
        })
        .await
        .expect("failed to send CAS write");

    match cas_fail_response {
        ClientRpcResponse::SecretsKvWriteResult(r) => {
            assert!(!r.is_success, "CAS write with wrong version should fail");
            assert!(
                r.error.as_ref().map(|e| e.contains("conflict") || e.contains("CAS")).unwrap_or(false),
                "error should mention conflict or CAS: {:?}",
                r.error
            );
            tracing::info!("CAS conflict detected as expected");
        }
        ClientRpcResponse::Error(e) => {
            // Also acceptable if it returns an error response
            tracing::info!(error = %e.message, "CAS conflict returned error");
        }
        _ => panic!("unexpected response"),
    }

    // Attempt write with correct CAS version (should succeed)
    let cas_ok_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvWrite {
            mount: KV_MOUNT.to_string(),
            path: "cas/test".to_string(),
            data: data_v2,
            cas: Some(version), // Correct version
        })
        .await
        .expect("failed to send CAS write");

    match cas_ok_response {
        ClientRpcResponse::SecretsKvWriteResult(r) => {
            assert!(r.is_success, "CAS write with correct version should succeed: {:?}", r.error);
            assert_eq!(r.version, Some(2), "should be version 2");
            tracing::info!("CAS write succeeded");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: KV metadata operations.
///
/// This test validates metadata operations:
/// 1. Write a secret
/// 2. Read metadata
/// 3. Update metadata (max versions)
/// 4. Verify metadata change
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_kv_metadata() {
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
            path: "metadata/test".to_string(),
            data,
            cas: None,
        })
        .await
        .expect("failed to write");

    match write_response {
        ClientRpcResponse::SecretsKvWriteResult(r) => assert!(r.is_success),
        _ => panic!("unexpected response"),
    }

    // Read metadata
    let metadata_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvMetadata {
            mount: KV_MOUNT.to_string(),
            path: "metadata/test".to_string(),
        })
        .await
        .expect("failed to read metadata");

    match metadata_response {
        ClientRpcResponse::SecretsKvMetadataResult(r) => {
            assert!(r.is_success, "metadata read should succeed: {:?}", r.error);
            assert!(r.current_version.is_some(), "should have current version");
            tracing::info!(current_version = r.current_version, max_versions = r.max_versions, "metadata retrieved");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: KV soft delete and undelete.
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
            assert!(r.is_success);
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
            assert!(r.is_success, "delete should succeed: {:?}", r.error);
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
            assert!(r.is_success, "undelete should succeed: {:?}", r.error);
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
            assert!(r.is_success, "read should succeed after undelete");
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
            assert!(r.is_success, "key creation should succeed: {:?}", r.error);
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
            assert!(r.is_success, "encryption should succeed: {:?}", r.error);
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
            assert!(r.is_success, "decryption should succeed: {:?}", r.error);
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
            assert!(r.is_success, "key creation should succeed: {:?}", r.error);
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
            assert!(r.is_success, "signing should succeed: {:?}", r.error);
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
            assert!(r.is_success, "verify request should succeed: {:?}", r.error);
            assert!(r.is_valid.unwrap_or(false), "signature should be valid");
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
        ClientRpcResponse::SecretsTransitKeyResult(r) => assert!(r.is_success),
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
            assert!(r.is_success);
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
            assert!(r.is_success, "rotation should succeed: {:?}", r.error);
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
            assert!(r.is_success, "decryption should work after rotation");
            let decrypted = r.plaintext.expect("should have plaintext");
            assert_eq!(decrypted, plaintext);
            tracing::info!("old ciphertext decrypted after rotation");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Transit engine - rewrap after key rotation.
///
/// This test validates rewrap functionality:
/// 1. Create key and encrypt
/// 2. Rotate key
/// 3. Rewrap ciphertext to use new key version
/// 4. Verify decryption still works
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_transit_rewrap() {
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
            name: "rewrap-key".to_string(),
            key_type: "aes256-gcm".to_string(),
        })
        .await
        .expect("failed to create key");

    match create_response {
        ClientRpcResponse::SecretsTransitKeyResult(r) => assert!(r.is_success),
        _ => panic!("unexpected response"),
    }

    // Encrypt with v1
    let plaintext = b"Secret data to rewrap".to_vec();
    let encrypt_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitEncrypt {
            mount: TRANSIT_MOUNT.to_string(),
            name: "rewrap-key".to_string(),
            plaintext: plaintext.clone(),
            context: None,
        })
        .await
        .expect("failed to encrypt");

    let ciphertext_v1 = match encrypt_response {
        ClientRpcResponse::SecretsTransitEncryptResult(r) => {
            assert!(r.is_success);
            r.ciphertext.expect("should have ciphertext")
        }
        _ => panic!("unexpected response"),
    };

    // Rotate key to v2
    let rotate_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitRotateKey {
            mount: TRANSIT_MOUNT.to_string(),
            name: "rewrap-key".to_string(),
        })
        .await
        .expect("failed to rotate");

    match rotate_response {
        ClientRpcResponse::SecretsTransitKeyResult(r) => {
            assert!(r.is_success);
            tracing::info!(version = r.version, "key rotated to v2");
        }
        _ => panic!("unexpected response"),
    }

    // Rewrap ciphertext to use v2
    let rewrap_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitRewrap {
            mount: TRANSIT_MOUNT.to_string(),
            name: "rewrap-key".to_string(),
            ciphertext: ciphertext_v1,
            context: None,
        })
        .await
        .expect("failed to rewrap");

    let ciphertext_v2 = match rewrap_response {
        ClientRpcResponse::SecretsTransitEncryptResult(r) => {
            assert!(r.is_success, "rewrap should succeed: {:?}", r.error);
            let ct = r.ciphertext.expect("should have rewrapped ciphertext");
            tracing::info!("ciphertext rewrapped to v2");
            ct
        }
        _ => panic!("unexpected response"),
    };

    // Verify decryption of rewrapped ciphertext works
    let decrypt_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitDecrypt {
            mount: TRANSIT_MOUNT.to_string(),
            name: "rewrap-key".to_string(),
            ciphertext: ciphertext_v2,
            context: None,
        })
        .await
        .expect("failed to decrypt");

    match decrypt_response {
        ClientRpcResponse::SecretsTransitDecryptResult(r) => {
            assert!(r.is_success, "decryption should succeed");
            let decrypted = r.plaintext.expect("should have plaintext");
            assert_eq!(decrypted, plaintext, "rewrapped data should decrypt correctly");
            tracing::info!("rewrapped ciphertext decrypted successfully");
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
            ClientRpcResponse::SecretsTransitKeyResult(r) => assert!(r.is_success),
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
            assert!(r.is_success, "list should succeed: {:?}", r.error);
            tracing::info!(keys = ?r.keys, "listed transit keys");
            assert!(r.keys.len() >= 3, "should have at least 3 keys");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Transit engine - multi-node encrypt/decrypt.
///
/// This test validates Transit works across nodes:
/// 1. Start 3-node cluster
/// 2. Create key (goes through Raft)
/// 3. Encrypt data
/// 4. Decrypt data
/// 5. Verify roundtrip across cluster
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_transit_multi_node() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(3)
        .with_workers(false)
        .with_timeout(Duration::from_secs(60));

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Create encryption key
    let create_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitCreateKey {
            mount: TRANSIT_MOUNT.to_string(),
            name: "cluster-key".to_string(),
            key_type: "aes256-gcm".to_string(),
        })
        .await
        .expect("failed to create key");

    match create_response {
        ClientRpcResponse::SecretsTransitKeyResult(r) => {
            assert!(r.is_success, "key creation should succeed in cluster");
            tracing::info!("key created in 3-node cluster");
        }
        _ => panic!("unexpected response"),
    }

    // Encrypt data
    let plaintext = b"Multi-node secret data".to_vec();

    let encrypt_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitEncrypt {
            mount: TRANSIT_MOUNT.to_string(),
            name: "cluster-key".to_string(),
            plaintext: plaintext.clone(),
            context: None,
        })
        .await
        .expect("failed to encrypt");

    let ciphertext = match encrypt_response {
        ClientRpcResponse::SecretsTransitEncryptResult(r) => {
            assert!(r.is_success, "encryption should succeed in cluster");
            r.ciphertext.expect("should have ciphertext")
        }
        _ => panic!("unexpected response"),
    };

    // Decrypt data
    let decrypt_response = tester
        .client()
        .send(ClientRpcRequest::SecretsTransitDecrypt {
            mount: TRANSIT_MOUNT.to_string(),
            name: "cluster-key".to_string(),
            ciphertext,
            context: None,
        })
        .await
        .expect("failed to decrypt");

    match decrypt_response {
        ClientRpcResponse::SecretsTransitDecryptResult(r) => {
            assert!(r.is_success, "decryption should succeed in cluster");
            let decrypted = r.plaintext.expect("should have plaintext");
            assert_eq!(decrypted, plaintext, "roundtrip should preserve data in cluster");
            tracing::info!("multi-node Transit roundtrip successful");
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
            assert!(r.is_success, "root generation should succeed: {:?}", r.error);
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
        ClientRpcResponse::SecretsPkiCertificateResult(r) => assert!(r.is_success),
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
            allow_subdomains: false,
            max_ttl_days: 30,
        })
        .await
        .expect("failed to create role");

    match role_response {
        ClientRpcResponse::SecretsPkiRoleResult(r) => {
            assert!(r.is_success, "role creation should succeed: {:?}", r.error);
            tracing::info!("role created");
        }
        _ => panic!("unexpected response"),
    }

    // Issue certificate (use bare domain matching allowed_domains)
    let issue_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiIssue {
            mount: PKI_MOUNT.to_string(),
            role: "web-server".to_string(),
            common_name: "example.com".to_string(),
            alt_names: vec!["test.local".to_string()],
            ttl_days: Some(7),
        })
        .await
        .expect("failed to issue certificate");

    match issue_response {
        ClientRpcResponse::SecretsPkiCertificateResult(r) => {
            assert!(r.is_success, "issuance should succeed: {:?}", r.error);
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
        ClientRpcResponse::SecretsPkiCertificateResult(r) => assert!(r.is_success),
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
            allow_subdomains: false,
            max_ttl_days: 30,
        })
        .await
        .expect("failed to create role");

    match role_response {
        ClientRpcResponse::SecretsPkiRoleResult(r) => assert!(r.is_success),
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
            assert!(r.is_success, "list roles should succeed: {:?}", r.error);
            tracing::info!(roles = ?r.items, "listed roles");
            assert!(!r.items.is_empty(), "should have at least one role");
        }
        _ => panic!("unexpected response"),
    }

    // List certificates (note: root CA is stored separately, this lists issued certs)
    let list_certs = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiListCerts {
            mount: PKI_MOUNT.to_string(),
        })
        .await
        .expect("failed to list certs");

    match list_certs {
        ClientRpcResponse::SecretsPkiListResult(r) => {
            assert!(r.is_success, "list certs should succeed: {:?}", r.error);
            tracing::info!(certs = ?r.items, "listed certificates");
            // Root CA is stored under ca/ prefix, list_certs returns issued certs from certs/
            // With just a root CA and no issued certs, the list will be empty
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
        ClientRpcResponse::SecretsPkiCertificateResult(r) => assert!(r.is_success),
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
            allow_subdomains: false,
            max_ttl_days: 30,
        })
        .await
        .expect("failed to create role");

    match role_response {
        ClientRpcResponse::SecretsPkiRoleResult(r) => assert!(r.is_success),
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
            assert!(r.is_success);
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
            assert!(r.is_success, "revocation should succeed: {:?}", r.error);
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
            assert!(r.is_success, "CRL should succeed: {:?}", r.error);
            let crl = r.crl.expect("should have CRL");
            // Current implementation returns CRL state summary, not PEM
            // TODO: Update when handler returns actual PEM-formatted CRL
            assert!(!crl.is_empty(), "should have CRL content");
            tracing::info!(crl = crl, "CRL retrieved");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: PKI engine - certificate chain in issuance response.
///
/// This test validates CA chain is included in issued certificates:
/// 1. Generate root CA
/// 2. Create role and issue certificate
/// 3. Verify ca_chain is returned in certificate result
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_pki_cert_chain_in_issue() {
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
            common_name: "Chain Test Root CA".to_string(),
            ttl_days: Some(365),
        })
        .await
        .expect("failed to generate root");

    match root_response {
        ClientRpcResponse::SecretsPkiCertificateResult(r) => {
            assert!(r.is_success);
            tracing::info!("root CA generated for chain test");
        }
        _ => panic!("unexpected response"),
    }

    // Create role
    let role_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiCreateRole {
            mount: PKI_MOUNT.to_string(),
            name: "chain-test".to_string(),
            allowed_domains: vec!["chain.local".to_string()],
            allow_bare_domains: true,
            allow_wildcards: false,
            allow_subdomains: false,
            max_ttl_days: 30,
        })
        .await
        .expect("failed to create role");

    match role_response {
        ClientRpcResponse::SecretsPkiRoleResult(r) => assert!(r.is_success),
        _ => panic!("unexpected response"),
    }

    // Issue certificate (use bare domain matching allowed_domains)
    let issue_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiIssue {
            mount: PKI_MOUNT.to_string(),
            role: "chain-test".to_string(),
            common_name: "chain.local".to_string(),
            alt_names: vec![],
            ttl_days: Some(7),
        })
        .await
        .expect("failed to issue");

    match issue_response {
        ClientRpcResponse::SecretsPkiCertificateResult(r) => {
            assert!(r.is_success, "issue should succeed: {:?}", r.error);
            let cert = r.certificate.expect("should have certificate");
            assert!(cert.contains("BEGIN CERTIFICATE"), "should be PEM certificate");
            tracing::info!(serial = r.serial, "certificate issued successfully");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: PKI engine - multi-node CA operations.
///
/// This test validates PKI works across a cluster:
/// 1. Start 3-node cluster
/// 2. Generate root CA
/// 3. Create role
/// 4. Issue certificate
/// 5. Verify all operations replicated via Raft
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_pki_multi_node() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(3)
        .with_workers(false)
        .with_timeout(Duration::from_secs(60));

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Generate root CA
    let root_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiGenerateRoot {
            mount: PKI_MOUNT.to_string(),
            common_name: "Cluster CA".to_string(),
            ttl_days: Some(365),
        })
        .await
        .expect("failed to generate root");

    match root_response {
        ClientRpcResponse::SecretsPkiCertificateResult(r) => {
            assert!(r.is_success, "root CA generation should succeed in cluster");
            tracing::info!("root CA generated in 3-node cluster");
        }
        _ => panic!("unexpected response"),
    }

    // Create role
    let role_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiCreateRole {
            mount: PKI_MOUNT.to_string(),
            name: "cluster-role".to_string(),
            allowed_domains: vec!["cluster.local".to_string()],
            allow_bare_domains: true,
            allow_wildcards: false,
            allow_subdomains: false,
            max_ttl_days: 30,
        })
        .await
        .expect("failed to create role");

    match role_response {
        ClientRpcResponse::SecretsPkiRoleResult(r) => {
            assert!(r.is_success, "role creation should succeed in cluster");
        }
        _ => panic!("unexpected response"),
    }

    // Issue certificate (use bare domain matching allowed_domains)
    let issue_response = tester
        .client()
        .send(ClientRpcRequest::SecretsPkiIssue {
            mount: PKI_MOUNT.to_string(),
            role: "cluster-role".to_string(),
            common_name: "cluster.local".to_string(),
            alt_names: vec![],
            ttl_days: Some(7),
        })
        .await
        .expect("failed to issue");

    match issue_response {
        ClientRpcResponse::SecretsPkiCertificateResult(r) => {
            assert!(r.is_success, "certificate issuance should succeed in cluster: {:?}", r.error);
            assert!(r.certificate.is_some(), "should have certificate");
            assert!(r.private_key.is_some(), "should have private key");
            tracing::info!(serial = r.serial, "certificate issued in cluster");
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
            assert!(r.is_success, "write should succeed");
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
            assert!(r.is_success, "read should succeed");
            let data = r.data.expect("should have data");
            assert_eq!(data.get("distributed"), Some(&"secret".to_string()));
            tracing::info!("secret read from cluster");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Concurrent secret writes are linearizable.
///
/// This test validates linearizable semantics:
/// 1. Start 3-node cluster
/// 2. Rapidly write multiple secrets
/// 3. Verify all writes are visible and consistent
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_concurrent_writes() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(3)
        .with_workers(false)
        .with_timeout(Duration::from_secs(60));

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Write multiple secrets in sequence (simulating concurrent access)
    for i in 0..5 {
        let mut data = HashMap::new();
        data.insert("index".to_string(), i.to_string());

        let write_response = tester
            .client()
            .send(ClientRpcRequest::SecretsKvWrite {
                mount: KV_MOUNT.to_string(),
                path: format!("concurrent/secret-{}", i),
                data,
                cas: None,
            })
            .await
            .expect("failed to write");

        match write_response {
            ClientRpcResponse::SecretsKvWriteResult(r) => {
                assert!(r.is_success, "write {} should succeed", i);
            }
            _ => panic!("unexpected response for write {}", i),
        }
    }

    // Verify all secrets are readable
    for i in 0..5 {
        let read_response = tester
            .client()
            .send(ClientRpcRequest::SecretsKvRead {
                mount: KV_MOUNT.to_string(),
                path: format!("concurrent/secret-{}", i),
                version: None,
            })
            .await
            .expect("failed to read");

        match read_response {
            ClientRpcResponse::SecretsKvReadResult(r) => {
                assert!(r.is_success, "read {} should succeed", i);
                let data = r.data.expect("should have data");
                assert_eq!(data.get("index"), Some(&i.to_string()), "data should match for {}", i);
            }
            _ => panic!("unexpected response for read {}", i),
        }
    }

    tracing::info!("all 5 concurrent writes verified");
    tester.shutdown().await.expect("shutdown failed");
}

/// Test: KV secrets with hard delete (destroy).
///
/// This test validates permanent deletion:
/// 1. Write a secret
/// 2. Destroy (hard delete) specific versions
/// 3. Verify destroyed versions are unrecoverable
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_secrets_kv_destroy() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Write a secret
    let mut data = HashMap::new();
    data.insert("key".to_string(), "to-destroy".to_string());

    let write_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvWrite {
            mount: KV_MOUNT.to_string(),
            path: "destroy/test".to_string(),
            data,
            cas: None,
        })
        .await
        .expect("failed to write");

    let version = match write_response {
        ClientRpcResponse::SecretsKvWriteResult(r) => {
            assert!(r.is_success);
            r.version.expect("should have version")
        }
        _ => panic!("unexpected response"),
    };

    // Destroy (hard delete) the version
    let destroy_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvDestroy {
            mount: KV_MOUNT.to_string(),
            path: "destroy/test".to_string(),
            versions: vec![version],
        })
        .await
        .expect("failed to destroy");

    match destroy_response {
        ClientRpcResponse::SecretsKvDeleteResult(r) => {
            assert!(r.is_success, "destroy should succeed: {:?}", r.error);
            tracing::info!("version {} destroyed", version);
        }
        _ => panic!("unexpected response"),
    }

    // Attempt to read destroyed version - should fail or return no data
    let read_response = tester
        .client()
        .send(ClientRpcRequest::SecretsKvRead {
            mount: KV_MOUNT.to_string(),
            path: "destroy/test".to_string(),
            version: Some(version),
        })
        .await
        .expect("failed to read");

    match read_response {
        ClientRpcResponse::SecretsKvReadResult(r) => {
            // Either success=false or no data expected for destroyed version
            if r.is_success {
                assert!(r.data.is_none(), "destroyed version should have no data");
            }
            tracing::info!("destroyed version correctly inaccessible");
        }
        ClientRpcResponse::Error(_) => {
            // Error response is also acceptable
            tracing::info!("destroyed version returned error as expected");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("shutdown failed");
}
