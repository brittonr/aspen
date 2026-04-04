//! Property-based tests for Shamir secret sharing.

use aspen_trust::shamir::SECRET_SIZE;
use aspen_trust::shamir::{self};
use proptest::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

fn arb_secret() -> impl Strategy<Value = [u8; SECRET_SIZE]> {
    prop::array::uniform32(any::<u8>())
}

fn arb_threshold_and_total() -> impl Strategy<Value = (u8, u8)> {
    // K in 2..=10, N >= K and N <= 20
    (2u8..=10).prop_flat_map(|k| (Just(k), k..=20))
}

proptest! {
    /// For all K in 2..10 and N >= K, split then reconstruct with K shares
    /// recovers the original secret.
    #[test]
    fn split_reconstruct_with_k_shares(
        secret in arb_secret(),
        (threshold, total) in arb_threshold_and_total(),
        seed: u64,
    ) {
        let mut rng = StdRng::seed_from_u64(seed);
        let shares = shamir::split_secret(&secret, threshold, total, &mut rng).unwrap();
        prop_assert_eq!(shares.len(), total as usize);

        // Take first K shares
        let reconstructed = shamir::reconstruct_secret(&shares[..threshold as usize]).unwrap();
        prop_assert_eq!(reconstructed, secret);
    }

    /// Reconstruct with fewer than K shares produces a different value.
    /// (Information-theoretic security: the result reveals nothing about the secret.)
    #[test]
    fn fewer_than_k_shares_produces_different_value(
        secret in arb_secret(),
        (threshold, total) in arb_threshold_and_total(),
        seed: u64,
    ) {
        let mut rng = StdRng::seed_from_u64(seed);
        let shares = shamir::split_secret(&secret, threshold, total, &mut rng).unwrap();

        // Use only (K - 1) shares
        let insufficient = &shares[..(threshold as usize - 1)];
        if insufficient.is_empty() {
            // K=1 case — can't have 0 shares, so skip
            return Ok(());
        }
        let wrong = shamir::reconstruct_secret(insufficient).unwrap();
        // With overwhelming probability, the wrong reconstruction differs
        // from the real secret. A false positive (wrong == secret) would
        // require all 32 Lagrange interpolations to accidentally hit the
        // correct value — probability ~2^{-256}.
        prop_assert_ne!(wrong, secret);
    }

    /// All N shares also reconstruct correctly.
    #[test]
    fn all_shares_reconstruct(
        secret in arb_secret(),
        (threshold, total) in arb_threshold_and_total(),
        seed: u64,
    ) {
        let mut rng = StdRng::seed_from_u64(seed);
        let shares = shamir::split_secret(&secret, threshold, total, &mut rng).unwrap();
        let reconstructed = shamir::reconstruct_secret(&shares).unwrap();
        prop_assert_eq!(reconstructed, secret);
    }

    /// Any K-subset of N shares reconstructs the original.
    #[test]
    fn any_k_subset_reconstructs(
        secret in arb_secret(),
        seed: u64,
    ) {
        // Fixed K=3, N=5 to keep combinatorics tractable (C(5,3) = 10 subsets)
        let threshold: u8 = 3;
        let total: u8 = 5;
        let mut rng = StdRng::seed_from_u64(seed);
        let shares = shamir::split_secret(&secret, threshold, total, &mut rng).unwrap();

        for i in 0..5usize {
            for j in (i + 1)..5 {
                for k in (j + 1)..5 {
                    let subset = vec![shares[i].clone(), shares[j].clone(), shares[k].clone()];
                    let reconstructed = shamir::reconstruct_secret(&subset).unwrap();
                    prop_assert_eq!(reconstructed, secret);
                }
            }
        }
    }

    /// Share digests change when any bit of the share is flipped.
    #[test]
    fn digest_detects_any_bit_flip(
        secret in arb_secret(),
        seed: u64,
        flip_byte in 0usize..SECRET_SIZE,
        flip_bit in 0u8..8,
    ) {
        let mut rng = StdRng::seed_from_u64(seed);
        let shares = shamir::split_secret(&secret, 3, 5, &mut rng).unwrap();
        let share = &shares[0];
        let original_digest = shamir::share_digest(share);

        let mut corrupted = share.clone();
        corrupted.y[flip_byte] ^= 1 << flip_bit;
        let corrupted_digest = shamir::share_digest(&corrupted);

        prop_assert_ne!(original_digest, corrupted_digest);
    }
}
