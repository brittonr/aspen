//! Integration test: simulate a 3-node cluster trust initialization.
//!
//! Verifies the full flow:
//! 1. Generate a cluster secret
//! 2. Split into 3 shares with threshold 2
//! 3. Store each node's share and all digests
//! 4. Verify shares via digests
//! 5. Reconstruct from any 2 of 3 nodes
//! 6. Derive keys and verify determinism

use std::collections::BTreeMap;

use aspen_trust::kdf;
use aspen_trust::secret::ClusterSecret;
use aspen_trust::secret::Threshold;
use aspen_trust::shamir::ShamirError;
use aspen_trust::shamir::ShareDigest;
use aspen_trust::shamir::{self};

fn total_shares_u8(total_shares: u32) -> u8 {
    (total_shares.min(u8::MAX as u32)) as u8
}

#[test]
fn test_three_node_trust_init_and_reconstruct() -> Result<(), ShamirError> {
    let n: u32 = 3;
    let threshold = Threshold::default_for_cluster_size(n);
    assert_eq!(threshold.value(), 2, "3-node cluster should default to threshold 2");

    // 1. Generate cluster secret
    let secret = ClusterSecret::generate();

    // 2. Split into shares
    let mut rng = rand::rng();
    let shares = shamir::split_secret(secret.as_bytes(), threshold.value(), total_shares_u8(n), &mut rng)?;
    assert_eq!(shares.len(), 3);

    // 3. Compute digests (simulating what the leader stores)
    let node_ids: Vec<u64> = vec![1, 2, 3];
    let digests: BTreeMap<u64, ShareDigest> =
        node_ids.iter().copied().zip(shares.iter().map(shamir::share_digest)).collect();

    // 4. Verify each share against its stored digest
    for (i, share) in shares.iter().enumerate() {
        let computed = shamir::share_digest(share);
        let stored = digests.get(&node_ids[i]).copied().unwrap_or_default();
        assert_eq!(computed, stored, "digest mismatch for node {}", node_ids[i]);
    }

    // 5. Reconstruct from any 2-of-3
    // Pair (0, 1)
    let reconstructed_01 = shamir::reconstruct_secret(&[shares[0].clone(), shares[1].clone()])?;
    assert_eq!(&reconstructed_01, secret.as_bytes());

    // Pair (0, 2)
    let reconstructed_02 = shamir::reconstruct_secret(&[shares[0].clone(), shares[2].clone()])?;
    assert_eq!(&reconstructed_02, secret.as_bytes());

    // Pair (1, 2)
    let reconstructed_12 = shamir::reconstruct_secret(&[shares[1].clone(), shares[2].clone()])?;
    assert_eq!(&reconstructed_12, secret.as_bytes());

    // All three
    let reconstructed_all = shamir::reconstruct_secret(&shares)?;
    assert_eq!(&reconstructed_all, secret.as_bytes());

    // 6. Single share cannot reconstruct
    let wrong = shamir::reconstruct_secret(&[shares[0].clone()])?;
    assert_ne!(&wrong, secret.as_bytes());

    // 7. Derive keys from reconstructed secret — deterministic
    let cluster_id = b"test-cluster-001";
    let epoch = 1u64;

    let key1 = kdf::derive_key(&reconstructed_01, kdf::CONTEXT_SECRETS_AT_REST, cluster_id, epoch);
    let key2 = kdf::derive_key(&reconstructed_12, kdf::CONTEXT_SECRETS_AT_REST, cluster_id, epoch);
    assert_eq!(key1, key2, "keys derived from different share subsets should match");

    // Different context produces different key
    let transit_key = kdf::derive_key(&reconstructed_01, kdf::CONTEXT_TRANSIT_KEYS, cluster_id, epoch);
    assert_ne!(key1, transit_key);
    Ok(())
}

#[test]
fn test_five_node_trust_any_three_reconstruct() -> Result<(), ShamirError> {
    let n: u32 = 5;
    let threshold = Threshold::default_for_cluster_size(n);
    assert_eq!(threshold.value(), 3);

    let secret = ClusterSecret::generate();
    let mut rng = rand::rng();
    let shares = shamir::split_secret(secret.as_bytes(), threshold.value(), total_shares_u8(n), &mut rng)?;

    // Try all C(5,3) = 10 subsets
    let mut success_count = 0u32;
    for i in 0..5usize {
        for j in (i).saturating_add(1)..5 {
            for k in (j).saturating_add(1)..5 {
                let subset = vec![shares[i].clone(), shares[j].clone(), shares[k].clone()];
                let reconstructed = shamir::reconstruct_secret(&subset)?;
                assert_eq!(&reconstructed, secret.as_bytes());
                success_count = success_count.saturating_add(1);
            }
        }
    }
    assert_eq!(success_count, 10);

    // Only 2 shares should fail
    let wrong = shamir::reconstruct_secret(&[shares[0].clone(), shares[3].clone()])?;
    assert_ne!(&wrong, secret.as_bytes());
    Ok(())
}

#[test]
fn test_single_node_trust() -> Result<(), ShamirError> {
    // K=1, N=1: single node holds the full secret
    let secret = ClusterSecret::generate();
    let mut rng = rand::rng();
    let shares = shamir::split_secret(secret.as_bytes(), 1, 1, &mut rng)?;
    assert_eq!(shares.len(), 1);

    let reconstructed = shamir::reconstruct_secret(&shares)?;
    assert_eq!(&reconstructed, secret.as_bytes());
    Ok(())
}

#[test]
fn test_custom_threshold() -> Result<(), ShamirError> {
    // 5-node cluster with threshold 4 (higher than default 3)
    let secret = ClusterSecret::generate();
    let mut rng = rand::rng();
    let shares = shamir::split_secret(secret.as_bytes(), 4, 5, &mut rng)?;

    // 3 shares should NOT reconstruct
    let wrong = shamir::reconstruct_secret(&[shares[0].clone(), shares[1].clone(), shares[2].clone()])?;
    assert_ne!(&wrong, secret.as_bytes());

    // 4 shares should reconstruct
    let correct = shamir::reconstruct_secret(&[
        shares[0].clone(),
        shares[1].clone(),
        shares[2].clone(),
        shares[3].clone(),
    ])?;
    assert_eq!(&correct, secret.as_bytes());
    Ok(())
}

#[test]
fn test_corrupted_share_detected_by_digest() -> Result<(), ShamirError> {
    let secret = ClusterSecret::generate();
    let mut rng = rand::rng();
    let shares = shamir::split_secret(secret.as_bytes(), 2, 3, &mut rng)?;

    let original_digest = shamir::share_digest(&shares[0]);

    // Corrupt the share
    let mut corrupted = shares[0].clone();
    corrupted.y[0] ^= 0xFF;

    let corrupted_digest = shamir::share_digest(&corrupted);
    assert_ne!(original_digest, corrupted_digest);

    // Reconstructing with corrupted share produces wrong secret
    let wrong = shamir::reconstruct_secret(&[corrupted, shares[1].clone()])?;
    assert_ne!(&wrong, secret.as_bytes());
    Ok(())
}
