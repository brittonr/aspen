//! Shamir secret sharing over GF(2^8).
//!
//! Splits a 32-byte secret into N shares with a K-of-N reconstruction threshold.
//! Each byte of the secret is treated as the constant term of an independent
//! random polynomial of degree K-1 over GF(2^8). Shares are evaluations of
//! these polynomials at distinct nonzero x-coordinates.
//!
//! Ported from the approach used in Oxide's `gfss` crate.

use rand::Rng;
use serde::Deserialize;
use serde::Serialize;
use sha3::Digest;
use sha3::Sha3_256;
use snafu::Snafu;
use subtle::ConstantTimeEq;
use zeroize::Zeroize;
use zeroize::ZeroizeOnDrop;

use crate::gf256;

/// The size of the secret in bytes.
pub const SECRET_SIZE: usize = 32;
const SERIALIZED_SHARE_SIZE: usize = SECRET_SIZE.saturating_add(1);

/// A single Shamir share: 1-byte x-coordinate + 32-byte y-values.
///
/// The x-coordinate identifies this share (nonzero, 1-indexed).
/// The y-values are the polynomial evaluations for each of the 32 secret bytes.
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Share {
    /// Nonzero x-coordinate in GF(2^8). Range: 1..=255.
    pub x: u8,
    /// 32 y-values, one per byte of the secret.
    pub y: [u8; SECRET_SIZE],
}

impl Share {
    /// Serialize the share to 33 bytes: [x, y[0], y[1], ..., y[31]].
    pub fn to_bytes(&self) -> [u8; SERIALIZED_SHARE_SIZE] {
        let mut buf = [0u8; SERIALIZED_SHARE_SIZE];
        buf[0] = self.x;
        buf[1..].copy_from_slice(&self.y);
        buf
    }

    /// Deserialize a share from 33 bytes.
    pub fn from_bytes(bytes: &[u8; SERIALIZED_SHARE_SIZE]) -> Result<Self, ShamirError> {
        if bytes[0] == 0 {
            return Err(ShamirError::ZeroShareCoordinate);
        }
        let mut y = [0u8; SECRET_SIZE];
        y.copy_from_slice(&bytes[1..]);
        Ok(Share { x: bytes[0], y })
    }
}

impl ConstantTimeEq for Share {
    fn ct_eq(&self, other: &Self) -> subtle::Choice {
        self.x.ct_eq(&other.x) & self.y.ct_eq(&other.y)
    }
}

impl PartialEq for Share {
    fn eq(&self, other: &Self) -> bool {
        bool::from(self.ct_eq(other))
    }
}

impl Eq for Share {}

impl core::fmt::Debug for Share {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Share").field("x", &self.x).field("y", &"[REDACTED]").finish()
    }
}

/// SHA3-256 digest of a share, used for tamper detection.
pub type ShareDigest = [u8; 32];

/// Compute the SHA3-256 digest of a share for tamper detection.
///
/// The digest covers the full 33-byte serialized form (x-coordinate + y-values).
pub fn share_digest(share: &Share) -> ShareDigest {
    let bytes = share.to_bytes();
    let mut hasher = Sha3_256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&result);
    digest
}

/// Errors from Shamir secret sharing operations.
#[derive(Debug, Snafu)]
pub enum ShamirError {
    /// Threshold must be at least 1.
    #[snafu(display("threshold must be >= 1, got {threshold}"))]
    ThresholdTooLow { threshold: u8 },

    /// Threshold must not exceed total share count.
    #[snafu(display("threshold {threshold} exceeds total shares {total}"))]
    ThresholdExceedsTotal { threshold: u8, total: u8 },

    /// Total share count must not exceed 255 (GF(2^8) has 255 nonzero elements).
    #[snafu(display("total shares {total} exceeds maximum 255"))]
    TotalTooLarge { total: u8 },

    /// Need at least threshold shares to reconstruct.
    #[snafu(display("insufficient shares: got {got}, need at least {threshold}"))]
    InsufficientShares { got: usize, threshold: u8 },

    /// Share x-coordinate must be nonzero.
    #[snafu(display("share x-coordinate must be nonzero"))]
    ZeroShareCoordinate,

    /// Duplicate x-coordinates in share set.
    #[snafu(display("duplicate x-coordinate: {x}"))]
    DuplicateShareCoordinate { x: u8 },
}

/// Split a 32-byte secret into `total` shares with reconstruction `threshold`.
///
/// Each share is an evaluation of 32 random degree-(threshold-1) polynomials
/// at a distinct nonzero x-coordinate. The constant term of each polynomial
/// is the corresponding byte of the secret.
///
/// # Constraints
/// - `threshold >= 1`
/// - `threshold <= total`
/// - `total <= 255` (GF(2^8) has 255 nonzero elements)
#[allow(unknown_lints)]
#[allow(
    ambiguous_params,
    reason = "threshold and total are well-named Shamir parameters with different semantics"
)]
pub fn split_secret<R: Rng>(
    secret: &[u8; SECRET_SIZE],
    threshold: u8,
    total: u8,
    rng: &mut R,
) -> Result<Vec<Share>, ShamirError> {
    debug_assert!(secret.len() == SECRET_SIZE, "secret must be exactly SECRET_SIZE bytes");
    if threshold < 1 {
        return Err(ShamirError::ThresholdTooLow { threshold });
    }
    if threshold > total {
        return Err(ShamirError::ThresholdExceedsTotal { threshold, total });
    }
    // total == 0 is caught by threshold > total (since threshold >= 1)
    // total can be at most 255 since u8 max is 255, but check explicitly
    // for clarity. total == 0 is already handled.
    if total == 0 {
        return Err(ShamirError::TotalTooLarge { total });
    }

    // Generate 32 random polynomials (one per secret byte).
    // Each polynomial has degree (threshold - 1) with the secret byte as constant term.
    let mut polynomials: Vec<Vec<u8>> = Vec::with_capacity(SECRET_SIZE);
    for &secret_byte in secret {
        let mut coeffs = vec![0u8; usize::from(threshold)];
        coeffs[0] = secret_byte;
        for coeff in coeffs.iter_mut().skip(1) {
            *coeff = rng.random();
        }
        polynomials.push(coeffs);
    }

    // Evaluate all 32 polynomials at each x-coordinate to produce shares
    let mut shares = Vec::with_capacity(usize::from(total));
    for share_idx in 0..total {
        let x = share_idx.saturating_add(1);
        let mut y = [0u8; SECRET_SIZE];
        for (byte_idx, coeffs) in polynomials.iter().enumerate() {
            y[byte_idx] = gf256::eval_polynomial(coeffs, x);
        }
        shares.push(Share { x, y });
    }

    // Zeroize polynomial coefficients
    for coeffs in &mut polynomials {
        for c in coeffs.iter_mut() {
            *c = 0;
        }
    }

    Ok(shares)
}

/// Reconstruct a 32-byte secret from at least `threshold` shares.
///
/// Uses Lagrange interpolation at x=0 over GF(2^8). The shares must have
/// been produced by `split_secret` with the same threshold. Providing fewer
/// than threshold shares produces an unrelated value (information-theoretic
/// security).
///
/// # Errors
/// - Duplicate x-coordinates
/// - Zero x-coordinate
pub fn reconstruct_secret(shares: &[Share]) -> Result<[u8; SECRET_SIZE], ShamirError> {
    debug_assert!(shares.iter().all(|s| s.y.len() == SECRET_SIZE), "all shares must be SECRET_SIZE bytes");
    if shares.is_empty() {
        return Err(ShamirError::InsufficientShares { got: 0, threshold: 1 });
    }

    // Validate: no duplicate x-coordinates, no zero x
    let xs: Vec<u8> = shares.iter().map(|s| s.x).collect();
    for (i, &x) in xs.iter().enumerate() {
        if x == 0 {
            return Err(ShamirError::ZeroShareCoordinate);
        }
        for &other in &xs[..i] {
            if x == other {
                return Err(ShamirError::DuplicateShareCoordinate { x });
            }
        }
    }

    let mut secret = [0u8; SECRET_SIZE];

    // Lagrange interpolation at x=0 for each of the 32 bytes
    for (byte_idx, secret_byte) in secret.iter_mut().enumerate() {
        let mut value: u8 = 0;
        for (i, share) in shares.iter().enumerate() {
            let share_index = u32::try_from(i).unwrap_or(u32::MAX);
            let li = gf256::lagrange_basis_at_zero(&xs, share_index);
            value ^= gf256::mul(share.y[byte_idx], li);
        }
        *secret_byte = value;
    }

    Ok(secret)
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::*;

    fn test_rng() -> StdRng {
        StdRng::seed_from_u64(12345)
    }

    #[test]
    fn test_share_roundtrip_bytes() -> Result<(), ShamirError> {
        let share = Share {
            x: 3,
            y: [7u8; SECRET_SIZE],
        };
        let bytes = share.to_bytes();
        let recovered = Share::from_bytes(&bytes)?;
        assert!(bool::from(share.ct_eq(&recovered)));
        Ok(())
    }

    #[test]
    fn test_share_from_bytes_rejects_zero_x() {
        let bytes = [0u8; SERIALIZED_SHARE_SIZE];
        assert!(Share::from_bytes(&bytes).is_err());
    }

    #[test]
    fn test_split_reconstruct_basic() -> Result<(), ShamirError> {
        let mut rng = test_rng();
        let secret = [42u8; SECRET_SIZE];
        let shares = split_secret(&secret, 3, 5, &mut rng)?;
        assert_eq!(shares.len(), 5);

        // Reconstruct with first 3 shares
        let reconstructed = reconstruct_secret(&shares[..3])?;
        assert_eq!(reconstructed, secret);
        Ok(())
    }

    #[test]
    fn test_split_reconstruct_all_shares() -> Result<(), ShamirError> {
        let mut rng = test_rng();
        let secret: [u8; SECRET_SIZE] = core::array::from_fn(|index| u8::try_from(index).unwrap_or_default());
        let shares = split_secret(&secret, 3, 5, &mut rng)?;

        let reconstructed = reconstruct_secret(&shares)?;
        assert_eq!(reconstructed, secret);
        Ok(())
    }

    #[test]
    fn test_split_reconstruct_any_k_subset() -> Result<(), ShamirError> {
        let mut rng = test_rng();
        let secret = [0xAB; SECRET_SIZE];
        let shares = split_secret(&secret, 3, 5, &mut rng)?;

        // Try all 3-element subsets of 5 shares
        for i in 0..5usize {
            for j in (i).saturating_add(1)..5 {
                for k in (j).saturating_add(1)..5 {
                    let subset = vec![shares[i].clone(), shares[j].clone(), shares[k].clone()];
                    let reconstructed = reconstruct_secret(&subset)?;
                    assert_eq!(reconstructed, secret, "failed with shares ({i}, {j}, {k})");
                }
            }
        }
        Ok(())
    }

    #[test]
    fn test_fewer_than_threshold_produces_wrong_value() -> Result<(), ShamirError> {
        let mut rng = test_rng();
        let secret = [0xFF; SECRET_SIZE];
        let shares = split_secret(&secret, 3, 5, &mut rng)?;

        // Only 2 shares (threshold is 3) — should NOT recover the secret
        let wrong = reconstruct_secret(&shares[..2])?;
        assert_ne!(wrong, secret);
        Ok(())
    }

    #[test]
    fn test_threshold_1_trivial() -> Result<(), ShamirError> {
        let mut rng = test_rng();
        let secret = [99u8; SECRET_SIZE];
        let shares = split_secret(&secret, 1, 3, &mut rng)?;

        // Any single share should reconstruct (threshold = 1)
        for share in &shares {
            let reconstructed = reconstruct_secret(core::slice::from_ref(share))?;
            assert_eq!(reconstructed, secret);
        }
        Ok(())
    }

    #[test]
    fn test_split_validation_errors() {
        let mut rng = test_rng();
        let secret = [0u8; SECRET_SIZE];

        assert!(split_secret(&secret, 0, 5, &mut rng).is_err()); // threshold too low
        assert!(split_secret(&secret, 6, 5, &mut rng).is_err()); // threshold > total
    }

    #[test]
    fn test_reconstruct_rejects_duplicate_x() {
        let s1 = Share {
            x: 1,
            y: [1; SECRET_SIZE],
        };
        let s2 = Share {
            x: 1,
            y: [2; SECRET_SIZE],
        };
        assert!(matches!(reconstruct_secret(&[s1, s2]), Err(ShamirError::DuplicateShareCoordinate { x: 1 })));
    }

    #[test]
    fn test_share_digest_deterministic() {
        let share = Share {
            x: 5,
            y: [42u8; SECRET_SIZE],
        };
        let d1 = share_digest(&share);
        let d2 = share_digest(&share);
        assert_eq!(d1, d2);
    }

    #[test]
    fn test_share_digest_detects_corruption() {
        let share = Share {
            x: 5,
            y: [42u8; SECRET_SIZE],
        };
        let original_digest = share_digest(&share);

        // Corrupt one bit
        let mut corrupted = share.clone();
        corrupted.y[0] ^= 1;
        let corrupted_digest = share_digest(&corrupted);

        assert_ne!(original_digest, corrupted_digest);
    }

    #[test]
    fn test_share_debug_redacts_y() {
        let share = Share {
            x: 1,
            y: [0xDE; SECRET_SIZE],
        };
        let debug = format!("{share:?}");
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("DE"));
    }
}
