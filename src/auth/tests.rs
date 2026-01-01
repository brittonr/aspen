//! Integration tests for capability-based authorization with the main Aspen crate.
//!
//! These tests verify the integration of aspen-auth with the distributed key-value store.

use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use iroh::SecretKey;

use super::*;
use crate::api::inmemory::DeterministicKeyValueStore;
use crate::api::KeyValueStore;
use crate::api::ReadRequest;

/// Counter for generating unique secret keys.
static KEY_COUNTER: AtomicU64 = AtomicU64::new(1000); // Start at 1000 to avoid collision with unit tests

/// Helper to create a unique test secret key.
fn test_secret_key() -> SecretKey {
    let counter = KEY_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut seed = [0u8; 32];
    seed[..8].copy_from_slice(&counter.to_le_bytes());
    SecretKey::from(seed)
}

// ============================================================================
// Revocation Store Integration Tests
// ============================================================================

#[tokio::test]
async fn test_revocation_store_roundtrip() {
    // DeterministicKeyValueStore::new() already returns Arc<Self>
    let kv = DeterministicKeyValueStore::new();
    let store = KeyValueRevocationStore::new(kv);

    let hash = [42u8; 32];

    // Initially not revoked
    assert!(!store.is_revoked(&hash).await.expect("should check"));

    // Revoke the token
    store.revoke(hash).await.expect("should revoke");

    // Now it's revoked
    assert!(store.is_revoked(&hash).await.expect("should check"));

    // Revoking again is idempotent
    store.revoke(hash).await.expect("should revoke again");
    assert!(store.is_revoked(&hash).await.expect("should check"));
}

#[tokio::test]
async fn test_load_revoked_at_startup() {
    // DeterministicKeyValueStore::new() already returns Arc<Self>
    let kv = DeterministicKeyValueStore::new();
    let store = KeyValueRevocationStore::new(kv);

    // Revoke several tokens via the store
    let hash1 = [1u8; 32];
    let hash2 = [2u8; 32];
    let hash3 = [3u8; 32];

    store.revoke(hash1).await.expect("should revoke");
    store.revoke(hash2).await.expect("should revoke");
    store.revoke(hash3).await.expect("should revoke");

    // Load all revocations
    let loaded = store.load_all().await.expect("should load");

    // Should contain all three hashes
    assert_eq!(loaded.len(), 3);
    assert!(loaded.contains(&hash1));
    assert!(loaded.contains(&hash2));
    assert!(loaded.contains(&hash3));

    // Create a new verifier and load the revocations
    let verifier = TokenVerifier::new();
    verifier.load_revoked(&loaded).unwrap();

    // Verify the verifier sees all as revoked
    assert!(verifier.is_revoked(&hash1).unwrap());
    assert!(verifier.is_revoked(&hash2).unwrap());
    assert!(verifier.is_revoked(&hash3).unwrap());
}

#[tokio::test]
async fn test_revocation_key_format() {
    // DeterministicKeyValueStore::new() already returns Arc<Self>
    let kv = DeterministicKeyValueStore::new();
    let store = KeyValueRevocationStore::new(kv.clone());

    // Create a hash with a distinctive pattern
    let hash = [0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89]
        .iter()
        .cycle()
        .take(32)
        .copied()
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    store.revoke(hash).await.expect("should revoke");

    // Verify the key is stored with correct format
    let expected_key = format!("_system:auth:revoked:{}", hex::encode(hash));
    let result = kv.read(ReadRequest::new(expected_key)).await.expect("should read");

    assert!(result.kv.is_some(), "revocation key should exist");
    assert_eq!(result.kv.unwrap().value, "", "value should be empty (existence is what matters)");
}

#[tokio::test]
async fn test_integration_persistent_revocation() {
    // Simulate a full revocation flow with persistence
    // DeterministicKeyValueStore::new() already returns Arc<Self>
    let kv = DeterministicKeyValueStore::new();
    let store = KeyValueRevocationStore::new(kv);
    let verifier = TokenVerifier::new();

    // Create a token
    let key = test_secret_key();
    let token = TokenBuilder::new(key)
        .with_capability(Capability::Read { prefix: "".into() })
        .with_random_nonce()
        .build()
        .expect("should build token");

    // Token should verify initially
    verifier.verify(&token, None).expect("should verify before revocation");

    // Revoke in both the verifier (in-memory) and the store (persistent)
    let token_hash = token.hash();
    verifier.revoke_token(&token).unwrap();
    store.revoke(token_hash).await.expect("should persist revocation");

    // Token should fail verification
    let result = verifier.verify(&token, None);
    assert!(matches!(result, Err(AuthError::TokenRevoked)));

    // Simulate restart: create new verifier and load from persistent store
    let verifier_after_restart = TokenVerifier::new();
    let loaded_revocations = store.load_all().await.expect("should load");
    verifier_after_restart.load_revoked(&loaded_revocations).unwrap();

    // Token should still be revoked after restart
    let result = verifier_after_restart.verify(&token, None);
    assert!(matches!(result, Err(AuthError::TokenRevoked)));
}
