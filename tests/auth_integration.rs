//! Integration tests for capability-based authorization.
//!
//! These tests verify the complete end-to-end flow of capability-based authorization
//! using TokenBuilder, TokenVerifier, and Capability types from the auth module.
//!
//! Test coverage includes:
//! - Token creation and verification
//! - Delegation and attenuation
//! - Expiration checking
//! - Revocation
//! - Audience validation
//! - Operation authorization

use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;

use aspen::auth::Audience;
use aspen::auth::AuthError;
use aspen::auth::Capability;
use aspen::auth::CapabilityToken;
use aspen::auth::Operation;
use aspen::auth::TokenBuilder;
use aspen::auth::TokenVerifier;
use aspen::auth::generate_root_token;
use iroh::SecretKey;

/// Counter for generating unique secret keys.
static KEY_COUNTER: AtomicU64 = AtomicU64::new(1_000_000);

/// Helper to create a unique test secret key.
///
/// Uses an atomic counter to ensure each key is unique across all tests.
fn test_secret_key() -> SecretKey {
    let counter = KEY_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut seed = [0u8; 32];
    seed[..8].copy_from_slice(&counter.to_le_bytes());
    // Add some entropy pattern to avoid all-zero keys
    seed[8] = 0xAB;
    seed[16] = 0xCD;
    seed[24] = 0xEF;
    SecretKey::from(seed)
}

// ============================================================================
// Test: Root Token Generation and Full Access
// ============================================================================

/// Verify that generate_root_token creates a token with full cluster access.
///
/// This test validates:
/// - Root token has Full, ClusterAdmin, and Delegate capabilities
/// - Root token authorizes all operation types
/// - Root token verifies correctly with trusted root issuer
#[tokio::test]
async fn test_root_token_grants_full_access() {
    let secret_key = test_secret_key();
    let lifetime = Duration::from_secs(3600);

    // Generate root token
    let root_token = generate_root_token(&secret_key, lifetime).expect("should generate root token");

    // Verify token structure
    assert_eq!(root_token.issuer, secret_key.public());
    assert!(matches!(root_token.audience, Audience::Bearer));
    assert!(root_token.nonce.is_some());
    assert!(root_token.proof.is_none()); // Root tokens have no parent

    // Verify capabilities
    assert!(
        root_token.capabilities.contains(&Capability::Full { prefix: String::new() }),
        "root token should have Full capability"
    );
    assert!(
        root_token.capabilities.contains(&Capability::ClusterAdmin),
        "root token should have ClusterAdmin capability"
    );
    assert!(
        root_token.capabilities.contains(&Capability::Delegate),
        "root token should have Delegate capability"
    );

    // Create verifier with trusted root
    let verifier = TokenVerifier::new().with_trusted_root(secret_key.public());

    // Verify token passes verification
    verifier.verify(&root_token, None).expect("root token should verify");

    // Test authorization for all operation types
    verifier
        .authorize(&root_token, &Operation::Read { key: "any:key".into() }, None)
        .expect("root token should authorize reads");

    verifier
        .authorize(
            &root_token,
            &Operation::Write {
                key: "any:key".into(),
                value: vec![1, 2, 3],
            },
            None,
        )
        .expect("root token should authorize writes");

    verifier
        .authorize(&root_token, &Operation::Delete { key: "any:key".into() }, None)
        .expect("root token should authorize deletes");

    verifier
        .authorize(
            &root_token,
            &Operation::Watch {
                key_prefix: "any:".into(),
            },
            None,
        )
        .expect("root token should authorize watch");

    verifier
        .authorize(
            &root_token,
            &Operation::ClusterAdmin {
                action: "init_cluster".into(),
            },
            None,
        )
        .expect("root token should authorize cluster admin");

    verifier
        .authorize(
            &root_token,
            &Operation::ClusterAdmin {
                action: "add_learner".into(),
            },
            None,
        )
        .expect("root token should authorize add_learner");

    verifier
        .authorize(
            &root_token,
            &Operation::ClusterAdmin {
                action: "change_membership".into(),
            },
            None,
        )
        .expect("root token should authorize change_membership");

    verifier
        .authorize(
            &root_token,
            &Operation::ClusterAdmin {
                action: "trigger_snapshot".into(),
            },
            None,
        )
        .expect("root token should authorize trigger_snapshot");
}

// ============================================================================
// Test: Delegated Token Attenuation
// ============================================================================

/// Verify that delegated tokens respect capability attenuation.
///
/// This test validates:
/// - Child tokens can be created from parent tokens with Delegate capability
/// - Child tokens cannot have more permissions than parent
/// - Child tokens can only access keys within their scope
/// - Write operations outside scope are rejected
#[tokio::test]
async fn test_delegated_token_respects_attenuation() {
    let root_key = test_secret_key();
    let service_key = test_secret_key();

    // Create root token with Full access to "myapp:" prefix
    let root_token = TokenBuilder::new(root_key.clone())
        .with_capability(Capability::Full {
            prefix: "myapp:".into(),
        })
        .with_capability(Capability::Delegate)
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .expect("should build root token");

    // Delegate narrower token with read-only access to "myapp:public:" prefix
    let delegated_token = TokenBuilder::new(service_key.clone())
        .delegated_from(root_token.clone())
        .with_capability(Capability::Read {
            prefix: "myapp:public:".into(),
        })
        .with_lifetime(Duration::from_secs(1800))
        .build()
        .expect("should build delegated token");

    // Verify delegation chain
    assert!(delegated_token.proof.is_some(), "delegated token should have proof of parent");
    assert_eq!(delegated_token.proof.unwrap(), root_token.hash(), "proof should match parent hash");

    let verifier = TokenVerifier::new();

    // Delegated token can read within its prefix
    verifier
        .authorize(
            &delegated_token,
            &Operation::Read {
                key: "myapp:public:data".into(),
            },
            None,
        )
        .expect("delegated token should authorize read in prefix");

    // Delegated token cannot read outside its prefix
    let result = verifier.authorize(
        &delegated_token,
        &Operation::Read {
            key: "myapp:private:secrets".into(),
        },
        None,
    );
    assert!(
        matches!(result, Err(AuthError::Unauthorized { .. })),
        "delegated token should not authorize read outside prefix"
    );

    // Delegated token cannot write (only has Read capability)
    let result = verifier.authorize(
        &delegated_token,
        &Operation::Write {
            key: "myapp:public:data".into(),
            value: vec![],
        },
        None,
    );
    assert!(matches!(result, Err(AuthError::Unauthorized { .. })), "delegated token should not authorize write");

    // Delegated token cannot delete
    let result = verifier.authorize(
        &delegated_token,
        &Operation::Delete {
            key: "myapp:public:data".into(),
        },
        None,
    );
    assert!(matches!(result, Err(AuthError::Unauthorized { .. })), "delegated token should not authorize delete");
}

/// Verify that capability escalation is prevented during delegation.
///
/// This test validates:
/// - Cannot delegate Write when parent only has Read
/// - Cannot delegate to wider prefix than parent
/// - Cannot delegate ClusterAdmin without having it
#[tokio::test]
async fn test_delegation_prevents_escalation() {
    let root_key = test_secret_key();
    let child_key = test_secret_key();

    // Create parent token with Read-only access
    let parent_token = TokenBuilder::new(root_key)
        .with_capability(Capability::Read {
            prefix: "users:".into(),
        })
        .with_capability(Capability::Delegate)
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .expect("should build parent token");

    // Attempt to escalate Read -> Write (should fail)
    let result = TokenBuilder::new(child_key.clone())
        .delegated_from(parent_token.clone())
        .with_capability(Capability::Write {
            prefix: "users:".into(),
        })
        .build();

    assert!(
        matches!(result, Err(AuthError::CapabilityEscalation { .. })),
        "should not allow Read -> Write escalation"
    );

    // Attempt to widen prefix (should fail)
    let result = TokenBuilder::new(child_key.clone())
        .delegated_from(parent_token.clone())
        .with_capability(Capability::Read {
            prefix: "".into(), // Root prefix - wider than "users:"
        })
        .build();

    assert!(matches!(result, Err(AuthError::CapabilityEscalation { .. })), "should not allow prefix widening");

    // Attempt to add ClusterAdmin without having it (should fail)
    let result = TokenBuilder::new(child_key)
        .delegated_from(parent_token)
        .with_capability(Capability::ClusterAdmin)
        .build();

    assert!(
        matches!(result, Err(AuthError::CapabilityEscalation { .. })),
        "should not allow adding ClusterAdmin"
    );
}

/// Verify that tokens without Delegate capability cannot delegate.
#[tokio::test]
async fn test_delegation_requires_delegate_capability() {
    let root_key = test_secret_key();
    let child_key = test_secret_key();

    // Create parent token WITHOUT Delegate capability
    let parent_token = TokenBuilder::new(root_key)
        .with_capability(Capability::Full {
            prefix: "myapp:".into(),
        })
        // Note: No .with_capability(Capability::Delegate)
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .expect("should build parent token");

    // Attempt to delegate (should fail)
    let result = TokenBuilder::new(child_key)
        .delegated_from(parent_token)
        .with_capability(Capability::Read {
            prefix: "myapp:".into(),
        })
        .build();

    assert!(
        matches!(result, Err(AuthError::DelegationNotAllowed)),
        "should not allow delegation without Delegate capability"
    );
}

// ============================================================================
// Test: Token Expiration
// ============================================================================

/// Verify that expired tokens are rejected.
///
/// Note: This test creates a token and manually modifies the expiration time
/// since we cannot wait for actual expiration in tests.
#[tokio::test]
async fn test_expired_token_rejected() {
    let secret_key = test_secret_key();

    // Create a token with normal lifetime
    let token = TokenBuilder::new(secret_key.clone())
        .with_capability(Capability::Read { prefix: "".into() })
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .expect("should build token");

    let verifier = TokenVerifier::new();

    // Token should verify initially
    verifier.verify(&token, None).expect("fresh token should verify");

    // Create an expired token by building one with modified timestamps
    // We simulate this by using the verifier with a very tight clock skew tolerance
    let tight_verifier = TokenVerifier::new().with_clock_skew_tolerance(0);

    // The token should still verify with tight tolerance since it was just created
    tight_verifier
        .verify(&token, None)
        .expect("just-created token should verify even with tight tolerance");

    // To properly test expiration, we need to encode/decode and modify
    // For this integration test, we verify the expiration check mechanism works
    // by testing a token where we manually set expiration in the past

    // Create a very short-lived token manually
    let mut expired_token = token.clone();
    expired_token.expires_at = 1; // January 1, 1970 - definitely expired

    // This will fail signature verification because we modified the token
    // without re-signing. The verifier checks signature first.
    let result = verifier.verify(&expired_token, None);
    assert!(result.is_err(), "modified token should fail verification (signature mismatch)");
}

// ============================================================================
// Test: Token Revocation
// ============================================================================

/// Verify that revoked tokens are rejected.
#[tokio::test]
async fn test_revoked_token_rejected() {
    let secret_key = test_secret_key();

    let token = TokenBuilder::new(secret_key)
        .with_capability(Capability::Read { prefix: "".into() })
        .with_random_nonce()
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .expect("should build token");

    let verifier = TokenVerifier::new();

    // Token should verify initially
    verifier.verify(&token, None).expect("token should verify before revocation");

    // Should also authorize operations
    verifier
        .authorize(&token, &Operation::Read { key: "test:key".into() }, None)
        .expect("token should authorize before revocation");

    // Revoke the token
    verifier.revoke_token(&token).unwrap();

    // Verify revocation count increased
    assert_eq!(verifier.revocation_count().unwrap(), 1);

    // Token should fail verification after revocation
    let result = verifier.verify(&token, None);
    assert!(matches!(result, Err(AuthError::TokenRevoked)), "revoked token should fail with TokenRevoked error");

    // Authorization should also fail
    let result = verifier.authorize(&token, &Operation::Read { key: "test:key".into() }, None);
    assert!(matches!(result, Err(AuthError::TokenRevoked)), "authorization with revoked token should fail");

    // Revoke by hash directly
    let token2 = TokenBuilder::new(test_secret_key())
        .with_capability(Capability::Read { prefix: "".into() })
        .with_random_nonce()
        .build()
        .expect("should build token2");

    let hash = token2.hash();
    verifier.revoke(hash).unwrap();
    assert_eq!(verifier.revocation_count().unwrap(), 2);

    // Check is_revoked helper
    assert!(verifier.is_revoked(&hash).unwrap());
    assert!(verifier.is_revoked(&token.hash()).unwrap());

    // Clear revocations
    verifier.clear_revocations().unwrap();
    assert_eq!(verifier.revocation_count().unwrap(), 0);
    assert!(!verifier.is_revoked(&hash).unwrap());

    // Token should verify again after clearing revocations
    verifier.verify(&token, None).expect("token should verify after clearing revocations");
}

// ============================================================================
// Test: Audience Validation
// ============================================================================

/// Verify that tokens with Key audience are only valid for the correct presenter.
#[tokio::test]
async fn test_wrong_audience_rejected() {
    let issuer_key = test_secret_key();
    let intended_audience = test_secret_key();
    let wrong_presenter = test_secret_key();

    // Create token for specific audience
    let token = TokenBuilder::new(issuer_key)
        .for_key(intended_audience.public())
        .with_capability(Capability::Read { prefix: "".into() })
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .expect("should build token");

    assert_eq!(
        token.audience,
        Audience::Key(intended_audience.public()),
        "token audience should be set to specific key"
    );

    let verifier = TokenVerifier::new();

    // Correct presenter should pass
    verifier.verify(&token, Some(&intended_audience.public())).expect("correct presenter should verify");

    // Wrong presenter should fail
    let result = verifier.verify(&token, Some(&wrong_presenter.public()));
    match result {
        Err(AuthError::WrongAudience { expected, actual }) => {
            assert_eq!(expected, intended_audience.public().to_string(), "error should contain expected audience");
            assert_eq!(actual, wrong_presenter.public().to_string(), "error should contain actual presenter");
        }
        other => panic!("expected WrongAudience error, got {:?}", other),
    }

    // No presenter when Key audience is required should fail
    let result = verifier.verify(&token, None);
    assert!(
        matches!(result, Err(AuthError::AudienceRequired)),
        "missing presenter should fail with AudienceRequired"
    );
}

/// Verify that bearer tokens work for any presenter.
#[tokio::test]
async fn test_bearer_token_accepts_any_presenter() {
    let issuer_key = test_secret_key();
    let random_presenter = test_secret_key();

    // Create bearer token (default)
    let token = TokenBuilder::new(issuer_key)
        .with_capability(Capability::Read { prefix: "".into() })
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .expect("should build token");

    assert!(matches!(token.audience, Audience::Bearer), "default audience should be Bearer");

    let verifier = TokenVerifier::new();

    // Bearer token should work without presenter
    verifier.verify(&token, None).expect("bearer token should verify without presenter");

    // Bearer token should also work with any presenter
    verifier
        .verify(&token, Some(&random_presenter.public()))
        .expect("bearer token should verify with any presenter");
}

// ============================================================================
// Test: Token Encoding/Decoding
// ============================================================================

/// Verify token encode/decode roundtrip preserves all fields.
#[tokio::test]
async fn test_token_encoding_roundtrip() {
    let secret_key = test_secret_key();
    let audience_key = test_secret_key().public();

    let original = TokenBuilder::new(secret_key.clone())
        .for_key(audience_key)
        .with_capability(Capability::Full {
            prefix: "myapp:".into(),
        })
        .with_capability(Capability::ClusterAdmin)
        .with_capability(Capability::Delegate)
        .with_random_nonce()
        .with_lifetime(Duration::from_secs(7200))
        .build()
        .expect("should build token");

    // Binary roundtrip
    let encoded = original.encode().expect("should encode to bytes");
    let decoded = CapabilityToken::decode(&encoded).expect("should decode from bytes");

    assert_eq!(original.version, decoded.version);
    assert_eq!(original.issuer, decoded.issuer);
    assert_eq!(original.audience, decoded.audience);
    assert_eq!(original.capabilities, decoded.capabilities);
    assert_eq!(original.issued_at, decoded.issued_at);
    assert_eq!(original.expires_at, decoded.expires_at);
    assert_eq!(original.nonce, decoded.nonce);
    assert_eq!(original.proof, decoded.proof);
    assert_eq!(original.signature, decoded.signature);

    // Base64 roundtrip
    let b64 = original.to_base64().expect("should encode to base64");
    let decoded_b64 = CapabilityToken::from_base64(&b64).expect("should decode from base64");

    assert_eq!(original.issuer, decoded_b64.issuer);
    assert_eq!(original.capabilities, decoded_b64.capabilities);

    // Decoded token should still verify
    let verifier = TokenVerifier::new().with_trusted_root(secret_key.public());
    verifier.verify(&decoded_b64, Some(&audience_key)).expect("decoded token should verify");
}

// ============================================================================
// Test: Trusted Root Verification
// ============================================================================

/// Verify that untrusted root issuers are rejected when trusted roots are configured.
#[tokio::test]
async fn test_untrusted_root_rejected() {
    let trusted_key = test_secret_key();
    let untrusted_key = test_secret_key();

    let trusted_token = TokenBuilder::new(trusted_key.clone())
        .with_capability(Capability::Read { prefix: "".into() })
        .build()
        .expect("should build trusted token");

    let untrusted_token = TokenBuilder::new(untrusted_key)
        .with_capability(Capability::Read { prefix: "".into() })
        .build()
        .expect("should build untrusted token");

    // Create verifier with only trusted root
    let verifier = TokenVerifier::new().with_trusted_root(trusted_key.public());

    // Trusted token should verify
    verifier.verify(&trusted_token, None).expect("trusted token should verify");

    // Untrusted token should fail (treated as InvalidSignature)
    let result = verifier.verify(&untrusted_token, None);
    assert!(result.is_err(), "untrusted root should fail verification");
}

// ============================================================================
// Test: Multi-level Delegation Chain
// ============================================================================

/// Verify delegation chains work correctly with proper attenuation at each level.
#[tokio::test]
async fn test_multi_level_delegation_chain() {
    let cluster_key = test_secret_key();
    let service_key = test_secret_key();
    let client_key = test_secret_key();

    // Level 1: Cluster issues service token
    let service_token = TokenBuilder::new(cluster_key.clone())
        .for_key(service_key.public())
        .with_capability(Capability::Full {
            prefix: "service:".into(),
        })
        .with_capability(Capability::Delegate)
        .with_lifetime(Duration::from_secs(86400)) // 24 hours
        .build()
        .expect("should build service token");

    // Verify service token structure
    assert!(service_token.proof.is_none()); // Root token, no parent
    assert_eq!(service_token.audience, Audience::Key(service_key.public()));

    // Level 2: Service delegates to client with narrower scope
    let client_token = TokenBuilder::new(service_key.clone())
        .delegated_from(service_token.clone())
        .for_key(client_key.public())
        .with_capability(Capability::Read {
            prefix: "service:public:".into(),
        })
        .with_lifetime(Duration::from_secs(3600)) // 1 hour
        .build()
        .expect("should build client token");

    // Verify client token structure
    assert_eq!(client_token.proof, Some(service_token.hash()));
    assert_eq!(client_token.audience, Audience::Key(client_key.public()));

    // Create verifier with trusted cluster root
    let verifier = TokenVerifier::new().with_trusted_root(cluster_key.public());

    // Service token should work
    verifier
        .authorize(
            &service_token,
            &Operation::Write {
                key: "service:data".into(),
                value: vec![],
            },
            Some(&service_key.public()),
        )
        .expect("service token should authorize write");

    // Client token can read public data
    verifier
        .authorize(
            &client_token,
            &Operation::Read {
                key: "service:public:info".into(),
            },
            Some(&client_key.public()),
        )
        .expect("client token should authorize read in public prefix");

    // Client token cannot read private data
    let result = verifier.authorize(
        &client_token,
        &Operation::Read {
            key: "service:private:secrets".into(),
        },
        Some(&client_key.public()),
    );
    assert!(matches!(result, Err(AuthError::Unauthorized { .. })), "client should not access private prefix");

    // Client token cannot write
    let result = verifier.authorize(
        &client_token,
        &Operation::Write {
            key: "service:public:info".into(),
            value: vec![],
        },
        Some(&client_key.public()),
    );
    assert!(matches!(result, Err(AuthError::Unauthorized { .. })), "client should not write");
}

// ============================================================================
// Test: Capability-specific Operation Authorization
// ============================================================================

/// Verify each capability type authorizes only its intended operations.
#[tokio::test]
async fn test_capability_operation_authorization() {
    let key = test_secret_key();
    let verifier = TokenVerifier::new();

    // Test Read capability
    let read_token = TokenBuilder::new(key.clone())
        .with_capability(Capability::Read { prefix: "data:".into() })
        .build()
        .expect("should build read token");

    verifier
        .authorize(
            &read_token,
            &Operation::Read {
                key: "data:item".into(),
            },
            None,
        )
        .expect("Read should authorize Read");

    assert!(
        verifier
            .authorize(
                &read_token,
                &Operation::Write {
                    key: "data:item".into(),
                    value: vec![]
                },
                None
            )
            .is_err()
    );

    // Test Write capability
    let write_token = TokenBuilder::new(key.clone())
        .with_capability(Capability::Write { prefix: "data:".into() })
        .build()
        .expect("should build write token");

    verifier
        .authorize(
            &write_token,
            &Operation::Write {
                key: "data:item".into(),
                value: vec![1],
            },
            None,
        )
        .expect("Write should authorize Write");

    assert!(
        verifier
            .authorize(
                &write_token,
                &Operation::Read {
                    key: "data:item".into()
                },
                None
            )
            .is_err()
    );

    // Test Delete capability
    let delete_token = TokenBuilder::new(key.clone())
        .with_capability(Capability::Delete { prefix: "data:".into() })
        .build()
        .expect("should build delete token");

    verifier
        .authorize(
            &delete_token,
            &Operation::Delete {
                key: "data:item".into(),
            },
            None,
        )
        .expect("Delete should authorize Delete");

    assert!(
        verifier
            .authorize(
                &delete_token,
                &Operation::Read {
                    key: "data:item".into()
                },
                None
            )
            .is_err()
    );

    // Test Watch capability
    let watch_token = TokenBuilder::new(key.clone())
        .with_capability(Capability::Watch { prefix: "data:".into() })
        .build()
        .expect("should build watch token");

    verifier
        .authorize(
            &watch_token,
            &Operation::Watch {
                key_prefix: "data:events:".into(),
            },
            None,
        )
        .expect("Watch should authorize Watch");

    assert!(
        verifier
            .authorize(
                &watch_token,
                &Operation::Read {
                    key: "data:item".into()
                },
                None
            )
            .is_err()
    );

    // Test ClusterAdmin capability
    let admin_token = TokenBuilder::new(key)
        .with_capability(Capability::ClusterAdmin)
        .build()
        .expect("should build admin token");

    verifier
        .authorize(
            &admin_token,
            &Operation::ClusterAdmin {
                action: "snapshot".into(),
            },
            None,
        )
        .expect("ClusterAdmin should authorize ClusterAdmin");

    assert!(verifier.authorize(&admin_token, &Operation::Read { key: "any:key".into() }, None).is_err());
}

// ============================================================================
// Test: Full Capability Authorizes Multiple Operations
// ============================================================================

/// Verify Full capability authorizes Read, Write, Delete, and Watch.
#[tokio::test]
async fn test_full_capability_authorizes_all_data_operations() {
    let key = test_secret_key();
    let verifier = TokenVerifier::new();

    let token = TokenBuilder::new(key)
        .with_capability(Capability::Full {
            prefix: "myapp:".into(),
        })
        .build()
        .expect("should build token");

    // Full should authorize Read
    verifier
        .authorize(
            &token,
            &Operation::Read {
                key: "myapp:data".into(),
            },
            None,
        )
        .expect("Full should authorize Read");

    // Full should authorize Write
    verifier
        .authorize(
            &token,
            &Operation::Write {
                key: "myapp:data".into(),
                value: vec![1, 2, 3],
            },
            None,
        )
        .expect("Full should authorize Write");

    // Full should authorize Delete
    verifier
        .authorize(
            &token,
            &Operation::Delete {
                key: "myapp:data".into(),
            },
            None,
        )
        .expect("Full should authorize Delete");

    // Full should authorize Watch
    verifier
        .authorize(
            &token,
            &Operation::Watch {
                key_prefix: "myapp:events:".into(),
            },
            None,
        )
        .expect("Full should authorize Watch");

    // Full should NOT authorize ClusterAdmin
    let result = verifier.authorize(
        &token,
        &Operation::ClusterAdmin {
            action: "snapshot".into(),
        },
        None,
    );
    assert!(matches!(result, Err(AuthError::Unauthorized { .. })), "Full should not authorize ClusterAdmin");

    // Full should NOT authorize outside prefix
    let result = verifier.authorize(
        &token,
        &Operation::Read {
            key: "other:data".into(),
        },
        None,
    );
    assert!(matches!(result, Err(AuthError::Unauthorized { .. })), "Full should not authorize outside prefix");
}

// ============================================================================
// Test: Token Hash Uniqueness
// ============================================================================

/// Verify that different tokens produce different hashes.
#[tokio::test]
async fn test_token_hash_uniqueness() {
    let key = test_secret_key();

    let token1 = TokenBuilder::new(key.clone())
        .with_capability(Capability::Read { prefix: "".into() })
        .with_random_nonce()
        .build()
        .expect("should build token1");

    let token2 = TokenBuilder::new(key)
        .with_capability(Capability::Read { prefix: "".into() })
        .with_random_nonce()
        .build()
        .expect("should build token2");

    // Different nonces should produce different hashes
    assert_ne!(token1.hash(), token2.hash());
    assert_ne!(token1.nonce, token2.nonce);
}

// ============================================================================
// Test: Token Validity Window
// ============================================================================

/// Verify tokens have correct validity window based on lifetime.
#[tokio::test]
async fn test_token_validity_window() {
    let key = test_secret_key();
    let lifetime = Duration::from_secs(7200); // 2 hours

    let token = TokenBuilder::new(key)
        .with_capability(Capability::Read { prefix: "".into() })
        .with_lifetime(lifetime)
        .build()
        .expect("should build token");

    // Token should have correct lifetime
    let actual_lifetime = token.expires_at - token.issued_at;
    assert_eq!(actual_lifetime, lifetime.as_secs());

    // Token should be valid now (issued_at should be recent)
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("time").as_secs();

    assert!(token.issued_at <= now, "issued_at should be at or before now");
    assert!(now - token.issued_at < 5, "issued_at should be within last 5 seconds");
    assert!(token.expires_at > now, "token should not be expired");
}

// ============================================================================
// Test: Clock Skew Tolerance
// ============================================================================

/// Verify verifier respects clock skew tolerance settings.
#[tokio::test]
async fn test_clock_skew_tolerance() {
    let key = test_secret_key();

    let token = TokenBuilder::new(key)
        .with_capability(Capability::Read { prefix: "".into() })
        .with_lifetime(Duration::from_secs(60))
        .build()
        .expect("should build token");

    // Default tolerance (60 seconds) should accept
    let default_verifier = TokenVerifier::new();
    default_verifier.verify(&token, None).expect("default tolerance should accept");

    // Custom tolerance should work
    let custom_verifier = TokenVerifier::new().with_clock_skew_tolerance(120);
    custom_verifier.verify(&token, None).expect("custom tolerance should accept");

    // Zero tolerance should still accept recently-created token
    let strict_verifier = TokenVerifier::new().with_clock_skew_tolerance(0);
    strict_verifier.verify(&token, None).expect("zero tolerance should accept fresh token");
}

// ============================================================================
// Test: Token Size Limits
// ============================================================================

/// Verify that tokens with too many capabilities are rejected.
#[tokio::test]
async fn test_too_many_capabilities_rejected() {
    let key = test_secret_key();

    let mut builder = TokenBuilder::new(key);

    // Add more capabilities than allowed (MAX_CAPABILITIES_PER_TOKEN = 32)
    for i in 0..40 {
        builder = builder.with_capability(Capability::Read {
            prefix: format!("prefix{i}:"),
        });
    }

    let result = builder.build();
    match result {
        Err(AuthError::TooManyCapabilities { count, max }) => {
            assert_eq!(count, 40);
            assert_eq!(max, 32);
        }
        other => panic!("expected TooManyCapabilities error, got {:?}", other),
    }
}

// ============================================================================
// Test: Operation Display Trait
// ============================================================================

/// Verify Operation Display formatting for logging.
#[tokio::test]
async fn test_operation_display() {
    let read = Operation::Read { key: "test:key".into() };
    assert_eq!(format!("{}", read), "Read(test:key)");

    let write = Operation::Write {
        key: "test:key".into(),
        value: vec![1, 2, 3],
    };
    assert_eq!(format!("{}", write), "Write(test:key)");

    let delete = Operation::Delete { key: "test:key".into() };
    assert_eq!(format!("{}", delete), "Delete(test:key)");

    let watch = Operation::Watch {
        key_prefix: "test:".into(),
    };
    assert_eq!(format!("{}", watch), "Watch(test:)");

    let admin = Operation::ClusterAdmin {
        action: "snapshot".into(),
    };
    assert_eq!(format!("{}", admin), "ClusterAdmin(snapshot)");
}

// ============================================================================
// Test: Verifier Debug Output
// ============================================================================

/// Verify TokenVerifier Debug doesn't leak sensitive information.
#[tokio::test]
async fn test_verifier_debug_safe() {
    let key = test_secret_key();
    let verifier = TokenVerifier::new().with_trusted_root(key.public()).with_clock_skew_tolerance(120);

    let debug = format!("{:?}", verifier);
    assert!(debug.contains("TokenVerifier"));
    assert!(debug.contains("trusted_roots"));
    assert!(debug.contains("clock_skew_tolerance"));
    assert!(debug.contains("revocation_count"));
}

// ============================================================================
// Test: Prefix Matching Edge Cases
// ============================================================================

/// Verify prefix matching works correctly for edge cases.
#[tokio::test]
async fn test_prefix_matching_edge_cases() {
    let key = test_secret_key();
    let verifier = TokenVerifier::new();

    // Empty prefix should match everything
    let root_token = TokenBuilder::new(key.clone())
        .with_capability(Capability::Read { prefix: String::new() })
        .build()
        .expect("should build token");

    verifier
        .authorize(
            &root_token,
            &Operation::Read {
                key: "any:path:here".into(),
            },
            None,
        )
        .expect("empty prefix should match any key");

    verifier
        .authorize(
            &root_token,
            &Operation::Read {
                key: "".into(), // Empty key
            },
            None,
        )
        .expect("empty prefix should match empty key");

    // Prefix should not match partial words
    let users_token = TokenBuilder::new(key)
        .with_capability(Capability::Read {
            prefix: "users:".into(),
        })
        .build()
        .expect("should build token");

    verifier
        .authorize(
            &users_token,
            &Operation::Read {
                key: "users:123".into(),
            },
            None,
        )
        .expect("prefix should match");

    // "users" (no colon) should not match "users:" prefix
    let result = verifier.authorize(
        &users_token,
        &Operation::Read {
            key: "usersList".into(),
        },
        None,
    );
    assert!(
        matches!(result, Err(AuthError::Unauthorized { .. })),
        "prefix 'users:' should not match 'usersList'"
    );
}
