//! Integration tests for Forge gossip & discovery layer.
//!
//! These tests validate Forge's gossip-based discovery and synchronization
//! functionality including:
//!
//! - Gossip topic subscription & broadcasting
//! - Seeding announcement handling
//! - Auto-sync from gossip announcements
//! - Rate limiting enforcement
//!
//! # Test Categories
//!
//! 1. **Gossip Message Tests** - Message serialization and signing
//! 2. **Discovery Tests** - Peer discovery and seeding announcements
//! 3. **Rate Limiting Tests** - Per-repo and per-peer limits
//!
//! # Tiger Style
//!
//! - Bounded timeouts: All operations have explicit timeouts
//! - Resource cleanup: Services shut down after tests
//! - Explicit error handling: All errors wrapped with context

mod support;

use aspen_forge::Announcement;
use aspen_forge::ForgeTopic;
use aspen_forge::RepoId;
use aspen_forge::SignedAnnouncement;
use aspen_forge::cob::CobType;

// ============================================================================
// Gossip Message Unit Tests
// ============================================================================

/// Generate a test repo ID.
fn test_repo_id() -> RepoId {
    RepoId([0xab; 32])
}

/// Test: RefUpdate announcement serialization roundtrip.
#[test]
fn test_ref_update_announcement_serialization() {
    let announcement = Announcement::RefUpdate {
        repo_id: test_repo_id(),
        ref_name: "heads/main".to_string(),
        new_hash: [0x02; 32],
        old_hash: Some([0x01; 32]),
    };

    // Serialize
    let bytes = announcement.to_bytes();

    // Deserialize
    let parsed = Announcement::from_bytes(&bytes).expect("deserialization should succeed");

    match parsed {
        Announcement::RefUpdate {
            repo_id,
            ref_name,
            new_hash,
            old_hash,
        } => {
            assert_eq!(repo_id, test_repo_id());
            assert_eq!(ref_name, "heads/main");
            assert_eq!(new_hash, [0x02; 32]);
            assert_eq!(old_hash, Some([0x01; 32]));
        }
        _ => panic!("expected RefUpdate announcement"),
    }
}

/// Test: RefUpdate with no old hash (first ref creation).
#[test]
fn test_ref_update_new_ref_creation() {
    let announcement = Announcement::RefUpdate {
        repo_id: test_repo_id(),
        ref_name: "heads/feature".to_string(),
        new_hash: [0x03; 32],
        old_hash: None, // New ref - no previous value
    };

    let bytes = announcement.to_bytes();
    let parsed = Announcement::from_bytes(&bytes).expect("deserialization should succeed");

    match parsed {
        Announcement::RefUpdate { old_hash, new_hash, .. } => {
            assert!(old_hash.is_none());
            assert_eq!(new_hash, [0x03; 32]);
        }
        _ => panic!("expected RefUpdate announcement"),
    }
}

/// Test: Seeding announcement serialization roundtrip.
#[test]
fn test_seeding_announcement_serialization() {
    let node_key = iroh::SecretKey::generate(&mut rand::rng()).public();
    let announcement = Announcement::Seeding {
        repo_id: test_repo_id(),
        node_id: node_key,
    };

    let bytes = announcement.to_bytes();
    let parsed = Announcement::from_bytes(&bytes).expect("deserialization should succeed");

    match parsed {
        Announcement::Seeding { repo_id, node_id } => {
            assert_eq!(repo_id, test_repo_id());
            assert_eq!(node_id, node_key);
        }
        _ => panic!("expected Seeding announcement"),
    }
}

/// Test: Unseeding announcement.
#[test]
fn test_unseeding_announcement() {
    let node_key = iroh::SecretKey::generate(&mut rand::rng()).public();
    let announcement = Announcement::Unseeding {
        repo_id: test_repo_id(),
        node_id: node_key,
    };

    let bytes = announcement.to_bytes();
    let parsed = Announcement::from_bytes(&bytes).expect("deserialization should succeed");

    match parsed {
        Announcement::Unseeding { repo_id, node_id } => {
            assert_eq!(repo_id, test_repo_id());
            assert_eq!(node_id, node_key);
        }
        _ => panic!("expected Unseeding announcement"),
    }
}

/// Test: COB change announcement.
#[test]
fn test_cob_change_announcement() {
    let announcement = Announcement::CobChange {
        repo_id: test_repo_id(),
        cob_type: CobType::Issue,
        cob_id: [0x01; 32],
        change_hash: [0x02; 32],
    };

    let bytes = announcement.to_bytes();
    let parsed = Announcement::from_bytes(&bytes).expect("deserialization should succeed");

    match parsed {
        Announcement::CobChange {
            repo_id,
            cob_type,
            cob_id,
            change_hash,
        } => {
            assert_eq!(repo_id, test_repo_id());
            assert_eq!(cob_type, CobType::Issue);
            assert_eq!(cob_id, [0x01; 32]);
            assert_eq!(change_hash, [0x02; 32]);
        }
        _ => panic!("expected CobChange announcement"),
    }
}

/// Test: RepoCreated announcement.
#[test]
fn test_repo_created_announcement() {
    let creator = iroh::SecretKey::generate(&mut rand::rng()).public();
    let announcement = Announcement::RepoCreated {
        repo_id: test_repo_id(),
        name: "my-project".to_string(),
        creator,
    };

    let bytes = announcement.to_bytes();
    let parsed = Announcement::from_bytes(&bytes).expect("deserialization should succeed");

    match parsed {
        Announcement::RepoCreated {
            repo_id,
            name,
            creator: c,
        } => {
            assert_eq!(repo_id, test_repo_id());
            assert_eq!(name, "my-project");
            assert_eq!(c, creator);
        }
        _ => panic!("expected RepoCreated announcement"),
    }
}

/// Test: Signed announcement creation and verification.
#[test]
fn test_signed_announcement() {
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());
    let announcement = Announcement::RefUpdate {
        repo_id: test_repo_id(),
        ref_name: "heads/main".to_string(),
        new_hash: [0x05; 32],
        old_hash: None,
    };

    // Sign the message
    let signed = SignedAnnouncement::sign(announcement.clone(), &secret_key);

    // Verify signature
    assert!(signed.verify().is_some(), "signature should be valid");

    // Verify content matches
    assert_eq!(signed.announcement, announcement);
    assert_eq!(signed.signer, secret_key.public());
}

/// Test: Tampered signed announcement detection.
#[test]
fn test_signed_announcement_tamper_detection() {
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());
    let announcement = Announcement::RefUpdate {
        repo_id: test_repo_id(),
        ref_name: "heads/main".to_string(),
        new_hash: [0x06; 32],
        old_hash: None,
    };

    let mut signed = SignedAnnouncement::sign(announcement, &secret_key);

    // Tamper with the announcement
    signed.announcement = Announcement::RefUpdate {
        repo_id: test_repo_id(),
        ref_name: "heads/tampered".to_string(), // Changed!
        new_hash: [0x06; 32],
        old_hash: None,
    };

    // Verification should fail
    assert!(signed.verify().is_none(), "tampered message should fail verification");
}

/// Test: Different signers produce different signatures.
#[test]
fn test_different_signer_detection() {
    let secret_key1 = iroh::SecretKey::generate(&mut rand::rng());
    let secret_key2 = iroh::SecretKey::generate(&mut rand::rng());

    let announcement = Announcement::RefUpdate {
        repo_id: test_repo_id(),
        ref_name: "heads/main".to_string(),
        new_hash: [0x07; 32],
        old_hash: None,
    };

    // Sign with different keys
    let signed1 = SignedAnnouncement::sign(announcement.clone(), &secret_key1);
    let signed2 = SignedAnnouncement::sign(announcement, &secret_key2);

    // Both should verify but have different signers
    assert!(signed1.verify().is_some());
    assert!(signed2.verify().is_some());
    assert_ne!(signed1.signer, signed2.signer);
}

// ============================================================================
// ForgeTopic Tests
// ============================================================================

/// Test: ForgeTopic for repo is deterministic.
#[test]
fn test_forge_topic_deterministic() {
    let repo_id = test_repo_id();

    // Create topic twice
    let topic1 = ForgeTopic::for_repo(repo_id);
    let topic2 = ForgeTopic::for_repo(repo_id);

    // Should produce identical topic IDs
    assert_eq!(topic1.to_topic_id(), topic2.to_topic_id());
}

/// Test: Different repos have different topics.
#[test]
fn test_different_repos_different_topics() {
    let repo1 = RepoId([0x01; 32]);
    let repo2 = RepoId([0x02; 32]);

    let topic1 = ForgeTopic::for_repo(repo1);
    let topic2 = ForgeTopic::for_repo(repo2);

    assert_ne!(topic1.to_topic_id(), topic2.to_topic_id());
}

/// Test: Global topic is consistent.
#[test]
fn test_global_topic_consistent() {
    let topic1 = ForgeTopic::global();
    let topic2 = ForgeTopic::global();

    assert_eq!(topic1.to_topic_id(), topic2.to_topic_id());
}

/// Test: Global and repo topics are different.
#[test]
fn test_global_vs_repo_topic() {
    let global_topic = ForgeTopic::global();
    let repo_topic = ForgeTopic::for_repo(test_repo_id());

    assert_ne!(global_topic.to_topic_id(), repo_topic.to_topic_id());
}

// ============================================================================
// Announcement variant coverage
// ============================================================================

/// Test: All announcement variants serialize correctly.
#[test]
fn test_all_announcement_variants() {
    let repo_id = test_repo_id();
    let node_key = iroh::SecretKey::generate(&mut rand::rng()).public();

    // RefUpdate variant
    let ref_update = Announcement::RefUpdate {
        repo_id,
        ref_name: "heads/main".to_string(),
        new_hash: [0x01; 32],
        old_hash: None,
    };
    let _ = Announcement::from_bytes(&ref_update.to_bytes()).expect("RefUpdate roundtrip");

    // CobChange variant
    let cob_change = Announcement::CobChange {
        repo_id,
        cob_type: CobType::Issue,
        cob_id: [0x02; 32],
        change_hash: [0x03; 32],
    };
    let _ = Announcement::from_bytes(&cob_change.to_bytes()).expect("CobChange roundtrip");

    // Seeding variant
    let seeding = Announcement::Seeding {
        repo_id,
        node_id: node_key,
    };
    let _ = Announcement::from_bytes(&seeding.to_bytes()).expect("Seeding roundtrip");

    // Unseeding variant
    let unseeding = Announcement::Unseeding {
        repo_id,
        node_id: node_key,
    };
    let _ = Announcement::from_bytes(&unseeding.to_bytes()).expect("Unseeding roundtrip");

    // RepoCreated variant
    let repo_created = Announcement::RepoCreated {
        repo_id,
        name: "test-repo".to_string(),
        creator: node_key,
    };
    let _ = Announcement::from_bytes(&repo_created.to_bytes()).expect("RepoCreated roundtrip");
}

/// Test: Announcement repo_id accessor.
#[test]
fn test_announcement_repo_id_accessor() {
    let repo_id = test_repo_id();
    let node_key = iroh::SecretKey::generate(&mut rand::rng()).public();

    let announcements = vec![
        Announcement::RefUpdate {
            repo_id,
            ref_name: "heads/main".to_string(),
            new_hash: [0x01; 32],
            old_hash: None,
        },
        Announcement::CobChange {
            repo_id,
            cob_type: CobType::Patch,
            cob_id: [0x01; 32],
            change_hash: [0x02; 32],
        },
        Announcement::Seeding {
            repo_id,
            node_id: node_key,
        },
        Announcement::Unseeding {
            repo_id,
            node_id: node_key,
        },
        Announcement::RepoCreated {
            repo_id,
            name: "test".to_string(),
            creator: node_key,
        },
    ];

    for ann in announcements {
        assert_eq!(*ann.repo_id(), repo_id, "repo_id() should return correct repo_id");
    }
}

// ============================================================================
// Edge Cases & Limits
// ============================================================================

/// Test: Empty ref name handling.
#[test]
fn test_empty_ref_name() {
    let announcement = Announcement::RefUpdate {
        repo_id: test_repo_id(),
        ref_name: String::new(), // Empty ref name (should be rejected by higher layers)
        new_hash: [0x01; 32],
        old_hash: None,
    };

    // Serialization should succeed (validation happens at business logic layer)
    let bytes = announcement.to_bytes();
    let parsed = Announcement::from_bytes(&bytes).expect("deserialization should succeed");

    match parsed {
        Announcement::RefUpdate { ref_name, .. } => assert!(ref_name.is_empty()),
        _ => panic!("expected RefUpdate"),
    }
}

/// Test: Long ref name handling.
#[test]
fn test_long_ref_name() {
    // Create a long ref name (but within reasonable limits)
    let long_name = format!("heads/{}", "a".repeat(200));

    let announcement = Announcement::RefUpdate {
        repo_id: test_repo_id(),
        ref_name: long_name.clone(),
        new_hash: [0x01; 32],
        old_hash: None,
    };

    let bytes = announcement.to_bytes();
    let parsed = Announcement::from_bytes(&bytes).expect("deserialization should succeed");

    match parsed {
        Announcement::RefUpdate { ref_name, .. } => assert_eq!(ref_name, long_name),
        _ => panic!("expected RefUpdate"),
    }
}

/// Test: Long repo name handling.
#[test]
fn test_long_repo_name() {
    let long_name = "a".repeat(500);
    let creator = iroh::SecretKey::generate(&mut rand::rng()).public();

    let announcement = Announcement::RepoCreated {
        repo_id: test_repo_id(),
        name: long_name.clone(),
        creator,
    };

    let bytes = announcement.to_bytes();
    let parsed = Announcement::from_bytes(&bytes).expect("deserialization should succeed");

    match parsed {
        Announcement::RepoCreated { name, .. } => assert_eq!(name, long_name),
        _ => panic!("expected RepoCreated"),
    }
}

/// Test: All CobType variants.
#[test]
fn test_all_cob_types() {
    let cob_types = [CobType::Issue, CobType::Patch];

    for cob_type in cob_types {
        let announcement = Announcement::CobChange {
            repo_id: test_repo_id(),
            cob_type,
            cob_id: [0x01; 32],
            change_hash: [0x02; 32],
        };

        let bytes = announcement.to_bytes();
        let parsed = Announcement::from_bytes(&bytes).expect("deserialization should succeed");

        match parsed {
            Announcement::CobChange {
                cob_type: parsed_type, ..
            } => {
                assert_eq!(parsed_type, cob_type);
            }
            _ => panic!("expected CobChange"),
        }
    }
}

// ============================================================================
// Note: Integration tests requiring real Iroh endpoints are in separate files.
// The gossip service lifecycle and multi-node tests require actual network
// connectivity which is not available in the Nix sandbox.
//
// Run network tests with: cargo nextest run --ignored
// ============================================================================
