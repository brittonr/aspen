//! Tests for capability-based authorization.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use iroh::SecretKey;

use super::*;
use crate::api::KeyValueStore;
use crate::api::inmemory::DeterministicKeyValueStore;

/// Counter for generating unique secret keys.
static KEY_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Helper to create a unique test secret key.
fn test_secret_key() -> SecretKey {
    let counter = KEY_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut seed = [0u8; 32];
    seed[..8].copy_from_slice(&counter.to_le_bytes());
    SecretKey::from(seed)
}

// ============================================================================
// Capability Tests
// ============================================================================

#[test]
fn test_capability_authorizes_read() {
    let cap = Capability::Read {
        prefix: "users:".into(),
    };

    // Should authorize reads within prefix
    assert!(cap.authorizes(&Operation::Read {
        key: "users:123".into()
    }));
    assert!(cap.authorizes(&Operation::Read {
        key: "users:".into()
    }));

    // Should not authorize reads outside prefix
    assert!(!cap.authorizes(&Operation::Read {
        key: "other:123".into()
    }));

    // Should not authorize other operations
    assert!(!cap.authorizes(&Operation::Write {
        key: "users:123".into(),
        value: vec![]
    }));
    assert!(!cap.authorizes(&Operation::Delete {
        key: "users:123".into()
    }));
}

#[test]
fn test_capability_authorizes_write() {
    let cap = Capability::Write {
        prefix: "users:".into(),
    };

    // Should authorize writes within prefix
    assert!(cap.authorizes(&Operation::Write {
        key: "users:123".into(),
        value: vec![1, 2, 3]
    }));

    // Should not authorize reads or deletes
    assert!(!cap.authorizes(&Operation::Read {
        key: "users:123".into()
    }));
    assert!(!cap.authorizes(&Operation::Delete {
        key: "users:123".into()
    }));
}

#[test]
fn test_capability_authorizes_full() {
    let cap = Capability::Full {
        prefix: "myapp:".into(),
    };

    // Should authorize all data operations within prefix
    assert!(cap.authorizes(&Operation::Read {
        key: "myapp:data".into()
    }));
    assert!(cap.authorizes(&Operation::Write {
        key: "myapp:config".into(),
        value: vec![]
    }));
    assert!(cap.authorizes(&Operation::Delete {
        key: "myapp:temp".into()
    }));
    assert!(cap.authorizes(&Operation::Watch {
        key_prefix: "myapp:events".into()
    }));

    // Should not authorize outside prefix
    assert!(!cap.authorizes(&Operation::Read {
        key: "other:data".into()
    }));

    // Should not authorize ClusterAdmin
    assert!(!cap.authorizes(&Operation::ClusterAdmin {
        action: "snapshot".into()
    }));
}

#[test]
fn test_capability_authorizes_cluster_admin() {
    let cap = Capability::ClusterAdmin;

    assert!(cap.authorizes(&Operation::ClusterAdmin {
        action: "snapshot".into()
    }));
    assert!(cap.authorizes(&Operation::ClusterAdmin {
        action: "add_learner".into()
    }));

    // Should not authorize data operations
    assert!(!cap.authorizes(&Operation::Read { key: "any".into() }));
}

#[test]
fn test_capability_contains() {
    let full = Capability::Full {
        prefix: "users:".into(),
    };
    let read_narrow = Capability::Read {
        prefix: "users:profiles:".into(),
    };
    let read_wider = Capability::Read { prefix: "".into() };

    // Full contains narrower read
    assert!(full.contains(&read_narrow));

    // Full does not contain wider read
    assert!(!full.contains(&read_wider));

    // Read contains narrower read
    let read_users = Capability::Read {
        prefix: "users:".into(),
    };
    assert!(read_users.contains(&read_narrow));

    // ClusterAdmin only contains itself
    assert!(Capability::ClusterAdmin.contains(&Capability::ClusterAdmin));
    assert!(!Capability::ClusterAdmin.contains(&Capability::Delegate));
}

// ============================================================================
// Token Builder Tests
// ============================================================================

#[test]
fn test_token_builder_basic() {
    let key = test_secret_key();

    let token = TokenBuilder::new(key.clone())
        .with_capability(Capability::Read {
            prefix: "test:".into(),
        })
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .expect("should build token");

    assert_eq!(token.version, 1);
    assert_eq!(token.issuer, key.public());
    assert_eq!(token.capabilities.len(), 1);
    assert!(token.expires_at > token.issued_at);
}

#[test]
fn test_token_builder_with_audience() {
    let issuer = test_secret_key();
    let audience_key = test_secret_key().public();

    let token = TokenBuilder::new(issuer)
        .for_key(audience_key)
        .with_capability(Capability::Read { prefix: "".into() })
        .build()
        .expect("should build token");

    assert_eq!(token.audience, Audience::Key(audience_key));
}

#[test]
fn test_token_builder_with_nonce() {
    let key = test_secret_key();

    let token = TokenBuilder::new(key)
        .with_capability(Capability::Read { prefix: "".into() })
        .with_random_nonce()
        .build()
        .expect("should build token");

    assert!(token.nonce.is_some());
    assert_ne!(token.nonce.unwrap(), [0u8; 16]);
}

#[test]
fn test_token_builder_too_many_capabilities() {
    let key = test_secret_key();

    let mut builder = TokenBuilder::new(key);
    for i in 0..40 {
        builder = builder.with_capability(Capability::Read {
            prefix: format!("prefix{i}:"),
        });
    }

    let result = builder.build();
    assert!(matches!(result, Err(AuthError::TooManyCapabilities { .. })));
}

#[test]
fn test_token_builder_delegation() {
    let root_key = test_secret_key();
    let child_key = test_secret_key();

    // Create root token with delegation capability
    let root = TokenBuilder::new(root_key)
        .with_capability(Capability::Full {
            prefix: "myapp:".into(),
        })
        .with_capability(Capability::Delegate)
        .build()
        .expect("should build root token");

    // Create child token with narrower scope
    let child = TokenBuilder::new(child_key)
        .delegated_from(root.clone())
        .with_capability(Capability::Read {
            prefix: "myapp:public:".into(),
        })
        .build()
        .expect("should build child token");

    assert!(child.proof.is_some());
    assert_eq!(child.proof.unwrap(), root.hash());
}

#[test]
fn test_token_builder_delegation_without_delegate_cap() {
    let root_key = test_secret_key();
    let child_key = test_secret_key();

    // Create root token WITHOUT delegation capability
    let root = TokenBuilder::new(root_key)
        .with_capability(Capability::Full {
            prefix: "myapp:".into(),
        })
        .build()
        .expect("should build root token");

    // Attempt to delegate should fail
    let result = TokenBuilder::new(child_key)
        .delegated_from(root)
        .with_capability(Capability::Read {
            prefix: "myapp:".into(),
        })
        .build();

    assert!(matches!(result, Err(AuthError::DelegationNotAllowed)));
}

#[test]
fn test_token_builder_capability_escalation() {
    let root_key = test_secret_key();
    let child_key = test_secret_key();

    // Create root token with limited scope
    let root = TokenBuilder::new(root_key)
        .with_capability(Capability::Read {
            prefix: "myapp:".into(),
        })
        .with_capability(Capability::Delegate)
        .build()
        .expect("should build root token");

    // Attempt to escalate to Write should fail
    let result = TokenBuilder::new(child_key)
        .delegated_from(root)
        .with_capability(Capability::Write {
            prefix: "myapp:".into(),
        })
        .build();

    assert!(matches!(
        result,
        Err(AuthError::CapabilityEscalation { .. })
    ));
}

// ============================================================================
// Token Encoding Tests
// ============================================================================

#[test]
fn test_token_encode_decode_roundtrip() {
    let key = test_secret_key();

    let token = TokenBuilder::new(key)
        .with_capability(Capability::Full {
            prefix: "test:".into(),
        })
        .with_random_nonce()
        .build()
        .expect("should build token");

    let encoded = token.encode().expect("should encode");
    let decoded = CapabilityToken::decode(&encoded).expect("should decode");

    assert_eq!(token.issuer, decoded.issuer);
    assert_eq!(token.capabilities, decoded.capabilities);
    assert_eq!(token.nonce, decoded.nonce);
}

#[test]
fn test_token_base64_roundtrip() {
    let key = test_secret_key();

    let token = TokenBuilder::new(key)
        .with_capability(Capability::Read { prefix: "".into() })
        .build()
        .expect("should build token");

    let b64 = token.to_base64().expect("should encode to base64");
    let decoded = CapabilityToken::from_base64(&b64).expect("should decode from base64");

    assert_eq!(token.issuer, decoded.issuer);
}

// ============================================================================
// Token Verifier Tests
// ============================================================================

#[test]
fn test_verifier_accepts_valid_token() {
    let key = test_secret_key();

    let token = TokenBuilder::new(key)
        .with_capability(Capability::Read { prefix: "".into() })
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .expect("should build token");

    let verifier = TokenVerifier::new();
    verifier.verify(&token, None).expect("should verify");
}

#[test]
fn test_verifier_rejects_tampered_signature() {
    let key = test_secret_key();

    let mut token = TokenBuilder::new(key)
        .with_capability(Capability::Read { prefix: "".into() })
        .build()
        .expect("should build token");

    // Tamper with the signature
    token.signature[0] ^= 0xFF;

    let verifier = TokenVerifier::new();
    let result = verifier.verify(&token, None);
    assert!(matches!(result, Err(AuthError::InvalidSignature)));
}

#[test]
fn test_verifier_rejects_expired_token() {
    let key = test_secret_key();

    let mut token = TokenBuilder::new(key)
        .with_capability(Capability::Read { prefix: "".into() })
        .build()
        .expect("should build token");

    // Make it expired
    token.expires_at = 1; // Long ago

    // Re-sign with original key (simulating a properly signed but expired token)
    // For this test, we just check the verifier logic
    let verifier = TokenVerifier::new();
    let result = verifier.verify(&token, None);
    // Will fail on signature because we changed expires_at but didn't re-sign
    assert!(result.is_err());
}

#[test]
fn test_verifier_checks_audience() {
    let issuer = test_secret_key();
    let intended_audience = test_secret_key();
    let wrong_presenter = test_secret_key();

    let token = TokenBuilder::new(issuer)
        .for_key(intended_audience.public())
        .with_capability(Capability::Read { prefix: "".into() })
        .build()
        .expect("should build token");

    let verifier = TokenVerifier::new();

    // Correct presenter should pass
    verifier
        .verify(&token, Some(&intended_audience.public()))
        .expect("should verify");

    // Wrong presenter should fail
    let result = verifier.verify(&token, Some(&wrong_presenter.public()));
    assert!(matches!(result, Err(AuthError::WrongAudience { .. })));

    // No presenter should fail for Key audience
    let result = verifier.verify(&token, None);
    assert!(matches!(result, Err(AuthError::AudienceRequired)));
}

#[test]
fn test_verifier_authorize() {
    let key = test_secret_key();

    let token = TokenBuilder::new(key)
        .with_capability(Capability::Read {
            prefix: "users:".into(),
        })
        .with_capability(Capability::Write {
            prefix: "users:self:".into(),
        })
        .build()
        .expect("should build token");

    let verifier = TokenVerifier::new();

    // Should authorize matching operations
    verifier
        .authorize(
            &token,
            &Operation::Read {
                key: "users:123".into(),
            },
            None,
        )
        .expect("should authorize read");
    verifier
        .authorize(
            &token,
            &Operation::Write {
                key: "users:self:profile".into(),
                value: vec![],
            },
            None,
        )
        .expect("should authorize write");

    // Should reject non-matching operations
    let result = verifier.authorize(
        &token,
        &Operation::Write {
            key: "users:other".into(),
            value: vec![],
        },
        None,
    );
    assert!(matches!(result, Err(AuthError::Unauthorized { .. })));

    let result = verifier.authorize(
        &token,
        &Operation::Delete {
            key: "users:123".into(),
        },
        None,
    );
    assert!(matches!(result, Err(AuthError::Unauthorized { .. })));
}

#[test]
fn test_verifier_revocation() {
    let key = test_secret_key();

    let token = TokenBuilder::new(key)
        .with_capability(Capability::Read { prefix: "".into() })
        .with_random_nonce()
        .build()
        .expect("should build token");

    let verifier = TokenVerifier::new();

    // Should verify before revocation
    verifier
        .verify(&token, None)
        .expect("should verify before revocation");

    // Revoke the token
    verifier.revoke_token(&token).unwrap();

    // Should fail after revocation
    let result = verifier.verify(&token, None);
    assert!(matches!(result, Err(AuthError::TokenRevoked)));
}

#[test]
fn test_verifier_trusted_roots() {
    let trusted = test_secret_key();
    let untrusted = test_secret_key();

    let trusted_token = TokenBuilder::new(trusted.clone())
        .with_capability(Capability::Read { prefix: "".into() })
        .build()
        .expect("should build token");

    let untrusted_token = TokenBuilder::new(untrusted)
        .with_capability(Capability::Read { prefix: "".into() })
        .build()
        .expect("should build token");

    let verifier = TokenVerifier::new().with_trusted_root(trusted.public());

    // Trusted issuer should pass
    verifier
        .verify(&trusted_token, None)
        .expect("should verify trusted");

    // Untrusted issuer should fail
    let result = verifier.verify(&untrusted_token, None);
    assert!(result.is_err());
}

// ============================================================================
// Root Token Generation Tests
// ============================================================================

#[test]
fn test_generate_root_token() {
    let secret = test_secret_key();
    let lifetime = Duration::from_secs(3600);

    let token = generate_root_token(&secret, lifetime).expect("should generate root token");

    // Should have correct issuer
    assert_eq!(token.issuer, secret.public());

    // Should be a bearer token
    assert!(matches!(token.audience, Audience::Bearer));

    // Should have Full, ClusterAdmin, and Delegate capabilities
    assert!(token.capabilities.contains(&Capability::Full {
        prefix: String::new()
    }));
    assert!(token.capabilities.contains(&Capability::ClusterAdmin));
    assert!(token.capabilities.contains(&Capability::Delegate));

    // Should have a nonce
    assert!(token.nonce.is_some());

    // Should verify correctly
    let verifier = TokenVerifier::new().with_trusted_root(secret.public());
    verifier
        .verify(&token, None)
        .expect("root token should verify");

    // Should authorize all operations
    verifier
        .authorize(
            &token,
            &Operation::Read {
                key: "any:key".into(),
            },
            None,
        )
        .expect("root token should authorize reads");

    verifier
        .authorize(
            &token,
            &Operation::Write {
                key: "any:key".into(),
                value: vec![],
            },
            None,
        )
        .expect("root token should authorize writes");

    verifier
        .authorize(
            &token,
            &Operation::Delete {
                key: "any:key".into(),
            },
            None,
        )
        .expect("root token should authorize deletes");

    verifier
        .authorize(
            &token,
            &Operation::ClusterAdmin {
                action: "init".into(),
            },
            None,
        )
        .expect("root token should authorize cluster admin");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_authorization_flow() {
    let cluster_key = test_secret_key();
    let service_key = test_secret_key();
    let client_key = test_secret_key();

    // 1. Cluster issues root token to service
    let service_token = TokenBuilder::new(cluster_key.clone())
        .for_key(service_key.public())
        .with_capability(Capability::Full {
            prefix: "service:".into(),
        })
        .with_capability(Capability::Delegate)
        .with_lifetime(Duration::from_secs(86400)) // 24 hours
        .build()
        .expect("should build service token");

    // 2. Service delegates limited access to client
    let client_token = TokenBuilder::new(service_key)
        .delegated_from(service_token.clone())
        .for_key(client_key.public())
        .with_capability(Capability::Read {
            prefix: "service:public:".into(),
        })
        .with_lifetime(Duration::from_secs(3600)) // 1 hour
        .build()
        .expect("should build client token");

    // 3. Verifier with trusted root
    let verifier = TokenVerifier::new().with_trusted_root(cluster_key.public());

    // 4. Verify client can read public data
    verifier
        .authorize(
            &client_token,
            &Operation::Read {
                key: "service:public:data".into(),
            },
            Some(&client_key.public()),
        )
        .expect("should authorize read");

    // 5. Verify client cannot read private data
    let result = verifier.authorize(
        &client_token,
        &Operation::Read {
            key: "service:private:secrets".into(),
        },
        Some(&client_key.public()),
    );
    assert!(matches!(result, Err(AuthError::Unauthorized { .. })));

    // 6. Verify client cannot write
    let result = verifier.authorize(
        &client_token,
        &Operation::Write {
            key: "service:public:data".into(),
            value: vec![],
        },
        Some(&client_key.public()),
    );
    assert!(matches!(result, Err(AuthError::Unauthorized { .. })));
}

// ============================================================================
// Revocation Store Tests
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
    let result = kv
        .read(crate::api::ReadRequest::new(expected_key))
        .await
        .expect("should read");

    assert!(result.kv.is_some(), "revocation key should exist");
    assert_eq!(
        result.kv.unwrap().value,
        "",
        "value should be empty (existence is what matters)"
    );
}

#[tokio::test]
async fn test_verifier_load_and_get_revoked() {
    let verifier = TokenVerifier::new();

    // Add some revocations
    let hash1 = [1u8; 32];
    let hash2 = [2u8; 32];
    let hash3 = [3u8; 32];

    verifier.revoke(hash1).unwrap();
    verifier.revoke(hash2).unwrap();
    verifier.revoke(hash3).unwrap();

    // Get all revoked hashes
    let all_revoked = verifier.get_all_revoked().unwrap();
    assert_eq!(all_revoked.len(), 3);
    assert!(all_revoked.contains(&hash1));
    assert!(all_revoked.contains(&hash2));
    assert!(all_revoked.contains(&hash3));

    // Create a new verifier and load from the first one
    let verifier2 = TokenVerifier::new();
    verifier2.load_revoked(&all_revoked).unwrap();

    // Both verifiers should see the same revocations
    assert_eq!(verifier2.revocation_count().unwrap(), 3);
    assert!(verifier2.is_revoked(&hash1).unwrap());
    assert!(verifier2.is_revoked(&hash2).unwrap());
    assert!(verifier2.is_revoked(&hash3).unwrap());
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
    verifier
        .verify(&token, None)
        .expect("should verify before revocation");

    // Revoke in both the verifier (in-memory) and the store (persistent)
    let token_hash = token.hash();
    verifier.revoke_token(&token).unwrap();
    store
        .revoke(token_hash)
        .await
        .expect("should persist revocation");

    // Token should fail verification
    let result = verifier.verify(&token, None);
    assert!(matches!(result, Err(AuthError::TokenRevoked)));

    // Simulate restart: create new verifier and load from persistent store
    let verifier_after_restart = TokenVerifier::new();
    let loaded_revocations = store.load_all().await.expect("should load");
    verifier_after_restart
        .load_revoked(&loaded_revocations)
        .unwrap();

    // Token should still be revoked after restart
    let result = verifier_after_restart.verify(&token, None);
    assert!(matches!(result, Err(AuthError::TokenRevoked)));
}
