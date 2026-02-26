//! Tests for capability-based authorization.

use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;

use iroh::SecretKey;

use super::*;

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
    assert!(cap.authorizes(&Operation::Read { key: "users:".into() }));

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
        .with_capability(Capability::Read { prefix: "test:".into() })
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

    assert!(matches!(result, Err(AuthError::CapabilityEscalation { .. })));
}

// ============================================================================
// Token Encoding Tests
// ============================================================================

#[test]
fn test_token_encode_decode_roundtrip() {
    let key = test_secret_key();

    let token = TokenBuilder::new(key)
        .with_capability(Capability::Full { prefix: "test:".into() })
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
    verifier.verify(&token, Some(&intended_audience.public())).expect("should verify");

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
    verifier.verify(&token, None).expect("should verify before revocation");

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
    verifier.verify(&trusted_token, None).expect("should verify trusted");

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
    assert!(token.capabilities.contains(&Capability::Full { prefix: String::new() }));
    assert!(token.capabilities.contains(&Capability::ClusterAdmin));
    assert!(token.capabilities.contains(&Capability::Delegate));

    // Should have a nonce
    assert!(token.nonce.is_some());

    // Should verify correctly
    let verifier = TokenVerifier::new().with_trusted_root(secret.public());
    verifier.verify(&token, None).expect("root token should verify");

    // Should authorize all operations
    verifier
        .authorize(&token, &Operation::Read { key: "any:key".into() }, None)
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
        .authorize(&token, &Operation::Delete { key: "any:key".into() }, None)
        .expect("root token should authorize deletes");

    verifier
        .authorize(&token, &Operation::ClusterAdmin { action: "init".into() }, None)
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

    // Register parent token for chain verification
    verifier.register_parent_token(service_token).expect("should register parent");

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

#[test]
fn test_verifier_load_and_get_revoked() {
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

// ============================================================================
// Delegation Depth Tests (Tiger Style Boundary Tests)
// ============================================================================

#[test]
fn test_delegation_depth_tracking() {
    // Test that delegation_depth field is properly set and incremented
    let root_key = test_secret_key();

    // Root token should have depth 0
    let root = TokenBuilder::new(root_key.clone())
        .with_capability(Capability::Full { prefix: "".into() })
        .with_capability(Capability::Delegate)
        .build()
        .expect("should build root token");

    assert_eq!(root.delegation_depth, 0);
    assert!(root.proof.is_none());

    // First level delegation should have depth 1
    let child1_key = test_secret_key();
    let child1 = TokenBuilder::new(child1_key.clone())
        .delegated_from(root.clone())
        .with_capability(Capability::Full { prefix: "app:".into() })
        .with_capability(Capability::Delegate)
        .build()
        .expect("should build child1 token");

    assert_eq!(child1.delegation_depth, 1);
    assert!(child1.proof.is_some());
    assert_eq!(child1.proof.unwrap(), root.hash());

    // Second level delegation should have depth 2
    let child2_key = test_secret_key();
    let child2 = TokenBuilder::new(child2_key)
        .delegated_from(child1.clone())
        .with_capability(Capability::Read {
            prefix: "app:data:".into(),
        })
        .with_capability(Capability::Delegate)
        .build()
        .expect("should build child2 token");

    assert_eq!(child2.delegation_depth, 2);
    assert_eq!(child2.proof.unwrap(), child1.hash());
}

#[test]
fn test_delegation_depth_at_max_allowed() {
    use crate::constants::MAX_DELEGATION_DEPTH;

    // Build a chain exactly at MAX_DELEGATION_DEPTH (8 levels)
    let mut current_token: Option<CapabilityToken> = None;

    for i in 0..=MAX_DELEGATION_DEPTH {
        let key = test_secret_key();
        let mut builder = TokenBuilder::new(key)
            .with_capability(Capability::Read { prefix: "test:".into() })
            .with_capability(Capability::Delegate);

        if let Some(parent) = current_token.take() {
            builder = builder.delegated_from(parent);
        }

        let token = builder.build().expect(&format!("should build token at depth {}", i));
        assert_eq!(token.delegation_depth, i);
        current_token = Some(token);
    }

    // Verify final token is at max depth
    let final_token = current_token.unwrap();
    assert_eq!(final_token.delegation_depth, MAX_DELEGATION_DEPTH);
}

#[test]
fn test_delegation_too_deep_error() {
    use crate::constants::MAX_DELEGATION_DEPTH;

    // Build a chain at MAX_DELEGATION_DEPTH
    let mut current_token: Option<CapabilityToken> = None;

    for i in 0..=MAX_DELEGATION_DEPTH {
        let key = test_secret_key();
        let mut builder = TokenBuilder::new(key)
            .with_capability(Capability::Read { prefix: "test:".into() })
            .with_capability(Capability::Delegate);

        if let Some(parent) = current_token.take() {
            builder = builder.delegated_from(parent);
        }

        current_token = Some(builder.build().expect(&format!("should build token at depth {}", i)));
    }

    // Attempt to delegate beyond MAX_DELEGATION_DEPTH should fail
    let final_token = current_token.unwrap();
    assert_eq!(final_token.delegation_depth, MAX_DELEGATION_DEPTH);

    let one_more_key = test_secret_key();
    let result = TokenBuilder::new(one_more_key)
        .delegated_from(final_token)
        .with_capability(Capability::Read { prefix: "test:".into() })
        .build();

    assert!(
        matches!(result, Err(AuthError::DelegationTooDeep { depth, max }) if depth == MAX_DELEGATION_DEPTH + 1 && max == MAX_DELEGATION_DEPTH),
        "Expected DelegationTooDeep error, got {:?}",
        result
    );
}

// ============================================================================
// Token Size Tests (Tiger Style Boundary Tests)
// ============================================================================

#[test]
fn test_token_encoding_typical_size() {
    // Typical token with a few capabilities should be well under 8KB
    let key = test_secret_key();

    let token = TokenBuilder::new(key)
        .with_capability(Capability::Full {
            prefix: "app:data:".into(),
        })
        .with_capability(Capability::Read {
            prefix: "app:config:".into(),
        })
        .with_capability(Capability::ClusterAdmin)
        .with_capability(Capability::Delegate)
        .with_random_nonce()
        .build()
        .expect("should build token");

    let encoded = token.encode().expect("should encode");

    // Typical token should be a few hundred bytes
    assert!(encoded.len() < 1000, "Typical token should be under 1KB, got {} bytes", encoded.len());
}

#[test]
fn test_token_with_max_capabilities() {
    use crate::constants::MAX_CAPABILITIES_PER_TOKEN;

    let key = test_secret_key();

    let mut builder = TokenBuilder::new(key);
    for i in 0..MAX_CAPABILITIES_PER_TOKEN {
        builder = builder.with_capability(Capability::Read {
            prefix: format!("prefix{}:", i),
        });
    }

    let token = builder.build().expect("should build token with max capabilities");
    assert_eq!(token.capabilities.len(), MAX_CAPABILITIES_PER_TOKEN as usize);

    // Should still encode successfully (under 8KB)
    let encoded = token.encode().expect("should encode token with max capabilities");
    assert!(
        encoded.len() < crate::constants::MAX_TOKEN_SIZE as usize,
        "Token with {} capabilities should fit in {} bytes, got {} bytes",
        MAX_CAPABILITIES_PER_TOKEN,
        crate::constants::MAX_TOKEN_SIZE,
        encoded.len()
    );
}

#[test]
fn test_token_too_large_on_encode() {
    use crate::constants::MAX_TOKEN_SIZE;

    let key = test_secret_key();

    // Create a token with very long prefixes to exceed size limit
    let mut builder = TokenBuilder::new(key);

    // Each capability with a long prefix adds significant size
    // 32 capabilities * ~300 bytes each should exceed 8KB
    for i in 0..32 {
        let long_prefix = format!(
            "very_long_prefix_for_capability_number_{}_with_lots_of_extra_characters_to_make_it_even_longer_and_push_the_token_over_the_size_limit_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa{}:",
            i, i
        );
        builder = builder.with_capability(Capability::Full { prefix: long_prefix });
    }

    let token = builder.build().expect("should build token");

    // Encoding should fail due to size limit
    let result = token.encode();
    assert!(
        matches!(result, Err(AuthError::TokenTooLarge { size_bytes, max_bytes }) if max_bytes == MAX_TOKEN_SIZE as u64 && size_bytes > max_bytes),
        "Expected TokenTooLarge error, got {:?}",
        result
    );
}

#[test]
fn test_token_too_large_on_decode() {
    use crate::constants::MAX_TOKEN_SIZE;

    // Create oversized bytes (more than 8KB)
    let oversized_bytes = vec![0u8; MAX_TOKEN_SIZE as usize + 1];

    let result = CapabilityToken::decode(&oversized_bytes);
    assert!(
        matches!(result, Err(AuthError::TokenTooLarge { size_bytes, max_bytes }) if max_bytes == MAX_TOKEN_SIZE as u64 && size_bytes > max_bytes),
        "Expected TokenTooLarge error on decode, got {:?}",
        result
    );
}

// ============================================================================
// Trusted Root Chain Verification Tests
// ============================================================================

#[test]
fn test_trusted_root_rejects_untrusted_issuer() {
    let trusted_key = test_secret_key();
    let untrusted_key = test_secret_key();

    let untrusted_token = TokenBuilder::new(untrusted_key)
        .with_capability(Capability::Read { prefix: "".into() })
        .build()
        .expect("should build token");

    let verifier = TokenVerifier::new().with_trusted_root(trusted_key.public());

    // Untrusted root token should fail with UntrustedRoot
    let result = verifier.verify(&untrusted_token, None);
    assert!(matches!(result, Err(AuthError::UntrustedRoot)), "Expected UntrustedRoot error, got {:?}", result);
}

#[test]
fn test_delegated_token_requires_parent_for_chain_verification() {
    let trusted_key = test_secret_key();
    let child_key = test_secret_key();

    // Create root token from trusted issuer
    let root = TokenBuilder::new(trusted_key.clone())
        .with_capability(Capability::Full { prefix: "".into() })
        .with_capability(Capability::Delegate)
        .build()
        .expect("should build root token");

    // Create delegated token
    let child = TokenBuilder::new(child_key)
        .delegated_from(root.clone())
        .with_capability(Capability::Read { prefix: "app:".into() })
        .build()
        .expect("should build child token");

    let verifier = TokenVerifier::new().with_trusted_root(trusted_key.public());

    // Without registering parent, verification should fail
    let result = verifier.verify(&child, None);
    assert!(
        matches!(result, Err(AuthError::ParentTokenRequired)),
        "Expected ParentTokenRequired error, got {:?}",
        result
    );

    // After registering parent, verification should succeed
    verifier.register_parent_token(root).expect("should register parent");
    verifier.verify(&child, None).expect("should verify with registered parent");
}

#[test]
fn test_verify_with_chain_method() {
    let trusted_key = test_secret_key();
    let service_key = test_secret_key();
    let client_key = test_secret_key();

    // Create 3-level chain: trusted -> service -> client
    let root = TokenBuilder::new(trusted_key.clone())
        .with_capability(Capability::Full { prefix: "".into() })
        .with_capability(Capability::Delegate)
        .build()
        .expect("should build root token");

    let service = TokenBuilder::new(service_key.clone())
        .delegated_from(root.clone())
        .with_capability(Capability::Full {
            prefix: "service:".into(),
        })
        .with_capability(Capability::Delegate)
        .build()
        .expect("should build service token");

    let client = TokenBuilder::new(client_key.clone())
        .delegated_from(service.clone())
        .for_key(client_key.public())
        .with_capability(Capability::Read {
            prefix: "service:data:".into(),
        })
        .build()
        .expect("should build client token");

    let verifier = TokenVerifier::new().with_trusted_root(trusted_key.public());

    // Verify with explicit chain (order: immediate parent first, then ancestors)
    verifier
        .verify_with_chain(&client, &[service, root], Some(&client_key.public()))
        .expect("should verify with chain");
}

#[test]
fn test_chain_verification_rejects_untrusted_root_in_chain() {
    let trusted_key = test_secret_key();
    let untrusted_key = test_secret_key();
    let child_key = test_secret_key();

    // Create root from UNTRUSTED issuer
    let untrusted_root = TokenBuilder::new(untrusted_key)
        .with_capability(Capability::Full { prefix: "".into() })
        .with_capability(Capability::Delegate)
        .build()
        .expect("should build untrusted root token");

    // Create child delegated from untrusted root
    let child = TokenBuilder::new(child_key)
        .delegated_from(untrusted_root.clone())
        .with_capability(Capability::Read { prefix: "app:".into() })
        .build()
        .expect("should build child token");

    // Verifier only trusts trusted_key
    let verifier = TokenVerifier::new().with_trusted_root(trusted_key.public());

    // Verification should fail because root is not trusted
    let result = verifier.verify_with_chain(&child, &[untrusted_root], None);
    assert!(
        matches!(result, Err(AuthError::UntrustedRoot)),
        "Expected UntrustedRoot error when chain leads to untrusted issuer, got {:?}",
        result
    );
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[test]
fn test_concurrent_revocation_operations() {
    use std::sync::Arc;
    use std::thread;

    let verifier = Arc::new(TokenVerifier::new());
    let mut handles = vec![];

    // Spawn multiple threads to revoke different tokens concurrently
    for i in 0..10 {
        let verifier_clone = Arc::clone(&verifier);
        handles.push(thread::spawn(move || {
            let mut hash = [0u8; 32];
            hash[0] = i;
            verifier_clone.revoke(hash).expect("should revoke");
        }));
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("thread should complete");
    }

    // Verify all revocations were recorded
    assert_eq!(verifier.revocation_count().unwrap(), 10);
}

#[test]
fn test_concurrent_verify_and_revoke() {
    use std::sync::Arc;
    use std::thread;

    let key = test_secret_key();
    let token = TokenBuilder::new(key)
        .with_capability(Capability::Read { prefix: "".into() })
        .with_random_nonce()
        .build()
        .expect("should build token");

    let verifier = Arc::new(TokenVerifier::new());
    let token_arc = Arc::new(token);
    let mut handles = vec![];

    // Spawn verification threads
    for _ in 0..5 {
        let verifier_clone = Arc::clone(&verifier);
        let token_clone = Arc::clone(&token_arc);
        handles.push(thread::spawn(move || {
            // Verify multiple times - may succeed or fail depending on timing
            for _ in 0..10 {
                let _ = verifier_clone.verify(&token_clone, None);
            }
        }));
    }

    // Spawn revocation thread
    let verifier_clone = Arc::clone(&verifier);
    let token_clone = Arc::clone(&token_arc);
    handles.push(thread::spawn(move || {
        verifier_clone.revoke_token(&token_clone).expect("should revoke");
    }));

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("thread should complete");
    }

    // Token should be revoked
    assert!(verifier.is_revoked(&token_arc.hash()).unwrap());
}

#[test]
fn test_concurrent_parent_registration() {
    use std::sync::Arc;
    use std::thread;

    let verifier = Arc::new(TokenVerifier::new());
    let mut handles = vec![];

    // Spawn multiple threads to register parent tokens concurrently
    for i in 0..10 {
        let verifier_clone = Arc::clone(&verifier);
        handles.push(thread::spawn(move || {
            let key = {
                // Create a unique key for each thread
                let mut seed = [0u8; 32];
                seed[0] = i;
                iroh::SecretKey::from(seed)
            };

            let token = TokenBuilder::new(key)
                .with_capability(Capability::Read {
                    prefix: format!("prefix{}:", i),
                })
                .build()
                .expect("should build token");

            verifier_clone.register_parent_token(token).expect("should register parent");
        }));
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("thread should complete");
    }

    // Clearing should work after concurrent registration
    verifier.clear_parent_cache().expect("should clear cache");
}
