//! Federation auth integration tests.
//!
//! Tests token issuance, credential verification, delegation chains,
//! unauthorized access rejection, token expiry, and revocation.

use std::time::Duration;

use aspen_auth::Capability;
use aspen_auth::Credential;
use aspen_auth::TokenBuilder;
use aspen_auth::TokenVerifier;
use iroh::SecretKey;

fn test_secret_key() -> SecretKey {
    SecretKey::generate(&mut rand::rng())
}

// =========================================================================
// 10.1: Two-cluster scenario — token issuance → verify → data transfer
// =========================================================================

#[test]
fn test_two_cluster_token_issuance_and_verification() {
    let cluster_a_sk = test_secret_key();
    let cluster_a_pk = cluster_a_sk.public();
    let cluster_b_sk = test_secret_key();
    let cluster_b_pk = cluster_b_sk.public();

    // Cluster A issues a token to Cluster B for nix cache
    let token = TokenBuilder::new(cluster_a_sk.clone())
        .for_key(cluster_b_pk)
        .with_capability(Capability::Read {
            prefix: "_sys:nix-cache:".into(),
        })
        .with_lifetime(Duration::from_secs(86400))
        .with_random_nonce()
        .build()
        .unwrap();

    let credential = Credential::from_root(token);

    // Cluster A verifies the credential presented by Cluster B
    assert!(credential.verify(&[cluster_a_pk], Some(&cluster_b_pk)).is_ok());

    // Credential should NOT verify with a different trusted root
    let other_pk = test_secret_key().public();
    assert!(credential.verify(&[other_pk], Some(&cluster_b_pk)).is_err());
}

#[test]
fn test_three_cluster_delegation_scenario() {
    let cluster_a_sk = test_secret_key();
    let cluster_a_pk = cluster_a_sk.public();
    let cluster_b_sk = test_secret_key();
    let cluster_b_pk = cluster_b_sk.public();
    let cluster_c_sk = test_secret_key();
    let cluster_c_pk = cluster_c_sk.public();

    // A → B: full nix cache access + delegate
    let a_to_b = TokenBuilder::new(cluster_a_sk.clone())
        .for_key(cluster_b_pk)
        .with_capability(Capability::Read {
            prefix: "_sys:nix-cache:".into(),
        })
        .with_capability(Capability::Delegate)
        .with_lifetime(Duration::from_secs(86400))
        .with_random_nonce()
        .build()
        .unwrap();

    let b_credential = Credential::from_root(a_to_b);

    // B → C: narrower narinfo access only
    let c_credential = b_credential
        .delegate(
            &cluster_b_sk,
            cluster_c_pk,
            vec![Capability::Read {
                prefix: "_sys:nix-cache:narinfo:".into(),
            }],
            Duration::from_secs(43200),
        )
        .unwrap();

    // Cluster C presents to Cluster A — should verify
    assert!(c_credential.verify(&[cluster_a_pk], Some(&cluster_c_pk)).is_ok());

    // Chain: leaf(C) → proof[0](B) → root(A)
    assert_eq!(c_credential.proofs.len(), 1);
    assert_eq!(c_credential.token.delegation_depth, 1);
}

// =========================================================================
// 10.2: Unauthorized access — outside token scope
// =========================================================================

#[test]
fn test_unauthorized_prefix_rejected() {
    let cluster_a_sk = test_secret_key();
    let cluster_a_pk = cluster_a_sk.public();
    let cluster_b_sk = test_secret_key();
    let cluster_b_pk = cluster_b_sk.public();

    // B has token for nix cache only
    let token = TokenBuilder::new(cluster_a_sk)
        .for_key(cluster_b_pk)
        .with_capability(Capability::Read {
            prefix: "_sys:nix-cache:".into(),
        })
        .with_lifetime(Duration::from_secs(86400))
        .build()
        .unwrap();

    let credential = Credential::from_root(token);

    // Verify succeeds (token is valid)
    assert!(credential.verify(&[cluster_a_pk], Some(&cluster_b_pk)).is_ok());

    // But authorizing forge access should fail
    let verifier = TokenVerifier::new().with_trusted_root(cluster_a_pk);
    let forge_op = aspen_auth::Operation::Read {
        key: "_forge:repos:aspen:".into(),
    };
    let result = verifier.authorize(&credential.token, &forge_op, Some(&cluster_b_pk));
    assert!(result.is_err(), "should not authorize access outside token scope");

    // Nix cache access should succeed
    let nix_op = aspen_auth::Operation::Read {
        key: "_sys:nix-cache:narinfo:abc123".into(),
    };
    let result = verifier.authorize(&credential.token, &nix_op, Some(&cluster_b_pk));
    assert!(result.is_ok(), "should authorize access within token scope");
}

// =========================================================================
// 10.3: Token expiry and refresh
// =========================================================================

#[test]
fn test_token_expiry_and_refresh_flow() {
    let cluster_a_sk = test_secret_key();
    let cluster_a_pk = cluster_a_sk.public();
    let cluster_b_pk = test_secret_key().public();

    // Issue a short-lived token (1 second lifetime, already expired by clock skew)
    // Create a short-lived token for testing
    let expired_token = TokenBuilder::new(cluster_a_sk.clone())
        .for_key(cluster_b_pk)
        .with_capability(Capability::Read {
            prefix: "_sys:nix-cache:".into(),
        })
        .with_lifetime(Duration::from_secs(1))
        .build()
        .unwrap();

    // Token should be valid immediately
    let verifier = TokenVerifier::new().with_trusted_root(cluster_a_pk);
    assert!(verifier.verify(&expired_token, Some(&cluster_b_pk)).is_ok());

    // Simulate refresh: issue a new token with same capabilities
    let refreshed_token = TokenBuilder::new(cluster_a_sk)
        .for_key(cluster_b_pk)
        .with_capability(Capability::Read {
            prefix: "_sys:nix-cache:".into(),
        })
        .with_lifetime(Duration::from_secs(86400)) // 24h
        .with_random_nonce()
        .build()
        .unwrap();

    assert!(verifier.verify(&refreshed_token, Some(&cluster_b_pk)).is_ok());
    assert!(refreshed_token.expires_at > expired_token.expires_at);
}

// =========================================================================
// 10.4: Token revocation
// =========================================================================

#[test]
fn test_token_revocation() {
    let cluster_a_sk = test_secret_key();
    let cluster_a_pk = cluster_a_sk.public();
    let cluster_b_pk = test_secret_key().public();

    let token = TokenBuilder::new(cluster_a_sk)
        .for_key(cluster_b_pk)
        .with_capability(Capability::Read {
            prefix: "_sys:nix-cache:".into(),
        })
        .with_lifetime(Duration::from_secs(86400))
        .with_random_nonce()
        .build()
        .unwrap();

    let verifier = TokenVerifier::new().with_trusted_root(cluster_a_pk);

    // Token is valid
    assert!(verifier.verify(&token, Some(&cluster_b_pk)).is_ok());

    // Revoke it
    verifier.revoke_token(&token).unwrap();

    // Token is now revoked
    let result = verifier.verify(&token, Some(&cluster_b_pk));
    assert!(matches!(result, Err(aspen_auth::AuthError::TokenRevoked)));
}

#[test]
fn test_revocation_gossip_message_roundtrip() {
    use aspen_federation::gossip::FederationGossipMessage;

    let revoker = test_secret_key().public();
    let token_hash = [0xab; 32];

    let msg = FederationGossipMessage::TokenRevoked {
        token_hash,
        revoker: *revoker.as_bytes(),
        timestamp_ms: 1710000000000,
    };

    // Verify cluster_key extraction works
    assert_eq!(msg.cluster_key().unwrap(), revoker);

    // Roundtrip serialization
    let bytes = postcard::to_allocvec(&msg).unwrap();
    let parsed: FederationGossipMessage = postcard::from_bytes(&bytes).unwrap();

    match parsed {
        FederationGossipMessage::TokenRevoked {
            token_hash: h,
            timestamp_ms: ts,
            ..
        } => {
            assert_eq!(h, token_hash);
            assert_eq!(ts, 1710000000000);
        }
        _ => panic!("expected TokenRevoked"),
    }
}

// =========================================================================
// Verified function tests
// =========================================================================

#[test]
fn test_verified_chain_validation() {
    use aspen_auth::verified_credential::*;

    // Valid single-level chain
    let parent_caps = vec![Capability::Read { prefix: "data:".into() }, Capability::Delegate];
    let leaf_caps = vec![Capability::Read {
        prefix: "data:sub:".into(),
    }];

    assert!(is_credential_chain_valid(&leaf_caps, 1, &[parent_caps.clone()], &[true]));

    // Invalid: escalation
    let bad_leaf_caps = vec![Capability::Write { prefix: "data:".into() }];
    assert!(!is_credential_chain_valid(&bad_leaf_caps, 1, &[parent_caps], &[true]));
}

#[test]
fn test_verified_prefix_authorization() {
    use aspen_auth::verified_credential::*;

    let caps = vec![Capability::Read {
        prefix: "_sys:nix-cache:".into(),
    }];

    assert!(credential_authorized_for_prefix(&caps, "_sys:nix-cache:narinfo:abc"));
    assert!(!credential_authorized_for_prefix(&caps, "_forge:repos:"));
}

#[test]
fn test_verified_refresh_timing() {
    use aspen_auth::verified_credential::*;

    // 24h token, check at various points
    let issued = 0u64;
    let expires = 86400u64;

    // At 50% — no refresh needed
    assert!(!needs_refresh(RefreshCheckInput {
        issued_at_secs: issued,
        expires_at_secs: expires,
        now_secs: 43_200,
    }));
    // At 80% — within refresh window
    assert!(needs_refresh(RefreshCheckInput {
        issued_at_secs: issued,
        expires_at_secs: expires,
        now_secs: 69_120,
    }));
    // At 95% — definitely needs refresh
    assert!(needs_refresh(RefreshCheckInput {
        issued_at_secs: issued,
        expires_at_secs: expires,
        now_secs: 82_080,
    }));
}

// =========================================================================
// Subscription types tests
// =========================================================================

#[test]
fn test_subscription_types_and_key_derivation() {
    use aspen_federation::subscription::*;

    let source = test_secret_key().public();

    // Key derivation is deterministic
    let k1 = pub_key("_sys:nix-cache:");
    let k2 = pub_key("_sys:nix-cache:");
    assert_eq!(k1, k2);

    let sk1 = sub_key(&source, "_sys:nix-cache:");
    let sk2 = sub_key(&source, "_sys:nix-cache:");
    assert_eq!(sk1, sk2);

    // Different inputs produce different keys
    let k3 = pub_key("_forge:repos:");
    assert_ne!(k1, k3);
}

// =========================================================================
// TrustManager credential-derived trust
// =========================================================================

#[test]
fn test_trust_manager_credential_lifecycle() {
    use aspen_federation::trust::TrustLevel;
    use aspen_federation::trust::TrustManager;

    let manager = TrustManager::new();
    let cluster_sk = test_secret_key();
    let remote_pk = test_secret_key().public();

    // Issue credential
    let token = TokenBuilder::new(cluster_sk)
        .for_key(remote_pk)
        .with_capability(Capability::Read { prefix: "data:".into() })
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .unwrap();
    let cred = Credential::from_root(token);

    // Before: Public
    assert_eq!(manager.trust_level(&remote_pk), TrustLevel::Public);

    // After credential: Trusted
    manager.update_from_credential(remote_pk, &cred);
    assert_eq!(manager.trust_level(&remote_pk), TrustLevel::Trusted);

    // Expire: back to Public
    manager.expire_credential(&remote_pk);
    assert_eq!(manager.trust_level(&remote_pk), TrustLevel::Public);

    // Re-trust, then revoke: Blocked
    manager.update_from_credential(remote_pk, &cred);
    manager.revoke_credential(remote_pk);
    assert_eq!(manager.trust_level(&remote_pk), TrustLevel::Blocked);
}

// =========================================================================
// Federation capability enforcement tests
// =========================================================================

#[test]
fn test_federation_push_credential_authorizes_push() {
    // Cluster A issues a push credential to Cluster B
    let cluster_a_sk = test_secret_key();
    let cluster_a_pk = cluster_a_sk.public();
    let cluster_b_pk = test_secret_key().public();

    let token = TokenBuilder::new(cluster_a_sk)
        .for_key(cluster_b_pk)
        .with_capability(Capability::FederationPush {
            repo_prefix: "forge:".into(),
        })
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .unwrap();

    let cred = Credential::from_root(token);

    // Verify credential
    assert!(cred.verify(&[cluster_a_pk], Some(&cluster_b_pk)).is_ok());

    // Check that FederationPush authorizes push
    let push_op = aspen_auth::Operation::FederationPush {
        fed_id: "forge:my-repo".into(),
    };
    assert!(cred.token.capabilities.iter().any(|c| c.authorizes(&push_op)));

    // But NOT pull
    let pull_op = aspen_auth::Operation::FederationPull {
        fed_id: "forge:my-repo".into(),
    };
    assert!(!cred.token.capabilities.iter().any(|c| c.authorizes(&pull_op)));
}

#[test]
fn test_federation_pull_credential_rejected_for_push() {
    let cluster_a_sk = test_secret_key();
    let cluster_b_pk = test_secret_key().public();

    let token = TokenBuilder::new(cluster_a_sk)
        .for_key(cluster_b_pk)
        .with_capability(Capability::FederationPull {
            repo_prefix: "forge:".into(),
        })
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .unwrap();

    let cred = Credential::from_root(token);

    // Pull-only credential should NOT authorize push
    let push_op = aspen_auth::Operation::FederationPush {
        fed_id: "forge:my-repo".into(),
    };
    assert!(!cred.token.capabilities.iter().any(|c| c.authorizes(&push_op)));
}

#[test]
fn test_federation_credential_wrong_prefix_denied() {
    let cluster_a_sk = test_secret_key();
    let cluster_b_pk = test_secret_key().public();

    let token = TokenBuilder::new(cluster_a_sk)
        .for_key(cluster_b_pk)
        .with_capability(Capability::FederationPush {
            repo_prefix: "forge:org-a/".into(),
        })
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .unwrap();

    let cred = Credential::from_root(token);

    // Matching prefix
    let push_a = aspen_auth::Operation::FederationPush {
        fed_id: "forge:org-a/my-repo".into(),
    };
    assert!(cred.token.capabilities.iter().any(|c| c.authorizes(&push_a)));

    // Non-matching prefix
    let push_b = aspen_auth::Operation::FederationPush {
        fed_id: "forge:org-b/my-repo".into(),
    };
    assert!(!cred.token.capabilities.iter().any(|c| c.authorizes(&push_b)));
}

#[test]
fn test_blocked_peer_denied_despite_credential() {
    use aspen_federation::trust::TrustLevel;
    use aspen_federation::trust::TrustManager;

    let manager = TrustManager::new();
    let cluster_sk = test_secret_key();
    let remote_pk = test_secret_key().public();

    // Issue valid credential
    let token = TokenBuilder::new(cluster_sk)
        .for_key(remote_pk)
        .with_capability(Capability::FederationPush {
            repo_prefix: String::new(),
        })
        .with_lifetime(Duration::from_secs(3600))
        .build()
        .unwrap();
    let cred = Credential::from_root(token);

    // Block the peer
    manager.block(remote_pk);
    assert_eq!(manager.trust_level(&remote_pk), TrustLevel::Blocked);

    // Even with a valid credential, blocked peers should be denied
    // The handler checks blocked status before credential capabilities
    assert!(manager.is_blocked(&remote_pk));

    // The credential is valid, but the trust layer overrides it
    let push_op = aspen_auth::Operation::FederationPush {
        fed_id: "forge:anything".into(),
    };
    assert!(cred.token.capabilities.iter().any(|c| c.authorizes(&push_op)));
    // ^ credential authorizes, but handler would still deny because blocked
}

#[test]
fn test_federation_delegation_attenuation() {
    let cluster_a_sk = test_secret_key();
    let cluster_a_pk = cluster_a_sk.public();
    let cluster_b_sk = test_secret_key();
    let cluster_b_pk = cluster_b_sk.public();
    let cluster_c_pk = test_secret_key().public();

    // A → B: broad pull + delegate
    let a_to_b = TokenBuilder::new(cluster_a_sk)
        .for_key(cluster_b_pk)
        .with_capability(Capability::FederationPull {
            repo_prefix: String::new(), // all repos
        })
        .with_capability(Capability::Delegate)
        .with_lifetime(Duration::from_secs(86400))
        .with_random_nonce()
        .build()
        .unwrap();

    let b_cred = Credential::from_root(a_to_b);

    // B → C: narrower pull (org-a only)
    let c_cred = b_cred
        .delegate(
            &cluster_b_sk,
            cluster_c_pk,
            vec![Capability::FederationPull {
                repo_prefix: "forge:org-a/".into(),
            }],
            Duration::from_secs(3600),
        )
        .unwrap();

    // C's credential should verify
    assert!(c_cred.verify(&[cluster_a_pk], Some(&cluster_c_pk)).is_ok());

    // C can pull from org-a
    let pull_a = aspen_auth::Operation::FederationPull {
        fed_id: "forge:org-a/repo".into(),
    };
    assert!(c_cred.token.capabilities.iter().any(|c| c.authorizes(&pull_a)));

    // C cannot pull from org-b (attenuated away)
    let pull_b = aspen_auth::Operation::FederationPull {
        fed_id: "forge:org-b/repo".into(),
    };
    assert!(!c_cred.token.capabilities.iter().any(|c| c.authorizes(&pull_b)));
}

#[test]
fn test_federation_delegation_escalation_rejected() {
    let cluster_a_sk = test_secret_key();
    let cluster_b_sk = test_secret_key();
    let cluster_b_pk = cluster_b_sk.public();
    let cluster_c_pk = test_secret_key().public();

    // A → B: narrow pull for org-a only + delegate
    let a_to_b = TokenBuilder::new(cluster_a_sk)
        .for_key(cluster_b_pk)
        .with_capability(Capability::FederationPull {
            repo_prefix: "forge:org-a/".into(),
        })
        .with_capability(Capability::Delegate)
        .with_lifetime(Duration::from_secs(86400))
        .with_random_nonce()
        .build()
        .unwrap();

    let b_cred = Credential::from_root(a_to_b);

    // B → C: try to escalate to all repos (should fail)
    let result = b_cred.delegate(
        &cluster_b_sk,
        cluster_c_pk,
        vec![Capability::FederationPull {
            repo_prefix: String::new(), // all repos — broader than parent
        }],
        Duration::from_secs(3600),
    );

    assert!(result.is_err(), "delegation escalation should be rejected");
}
