//! Cluster root secret type.
//!
//! A 32-byte secret generated from `OsRng` with zeroize-on-drop and
//! constant-time equality. This is the root key material that gets split
//! into Shamir shares during cluster initialization.

use rand::TryRngCore;
use rand::rngs::OsRng;
use subtle::ConstantTimeEq;
use zeroize::Zeroize;
use zeroize::ZeroizeOnDrop;

use crate::verified;

/// A 32-byte cluster root secret.
///
/// Generated from `OsRng`, zeroed on drop. All comparisons are constant-time.
/// The raw bytes should only be accessed for splitting into shares or for
/// key derivation — never stored directly.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct ClusterSecret {
    bytes: [u8; 32],
}

impl ClusterSecret {
    /// Generate a new random cluster secret from the OS CSPRNG.
    ///
    /// # Panics
    /// Panics if the generated secret is all zeros (astronomically unlikely,
    /// but checked as a safety invariant).
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        OsRng.try_fill_bytes(&mut bytes).expect("OS CSPRNG failure");
        // Safety invariant: secret must not be all zeros
        assert!(!bytes.iter().all(|&b| b == 0), "generated secret is all zeros (CSPRNG failure)");
        ClusterSecret { bytes }
    }

    /// Create a `ClusterSecret` from existing bytes.
    ///
    /// Returns `None` if the bytes are all zeros.
    pub fn from_bytes(bytes: [u8; 32]) -> Option<Self> {
        if bytes.iter().all(|&b| b == 0) {
            return None;
        }
        Some(ClusterSecret { bytes })
    }

    /// Access the raw secret bytes.
    ///
    /// Use only for splitting into shares or key derivation.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

impl ConstantTimeEq for ClusterSecret {
    fn ct_eq(&self, other: &Self) -> subtle::Choice {
        self.bytes.ct_eq(&other.bytes)
    }
}

impl PartialEq for ClusterSecret {
    fn eq(&self, other: &Self) -> bool {
        bool::from(self.ct_eq(other))
    }
}

impl Eq for ClusterSecret {}

impl core::fmt::Debug for ClusterSecret {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ClusterSecret").field("bytes", &"[REDACTED]").finish()
    }
}

/// Threshold configuration for Shamir secret sharing.
///
/// Wraps a `u8` representing the minimum number of shares needed to
/// reconstruct the cluster secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Threshold(u8);

impl Threshold {
    /// Create a threshold value.
    ///
    /// Returns `None` if the value is 0.
    pub fn new(value: u8) -> Option<Self> {
        if value == 0 { None } else { Some(Threshold(value)) }
    }

    /// Compute the default threshold for a given cluster size: `(n/2) + 1`.
    pub fn default_for_cluster_size(n: u32) -> Self {
        Threshold(verified::default_threshold_for_size(n))
    }

    /// Get the raw threshold value.
    pub fn value(self) -> u8 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_not_all_zeros() {
        // Generate several secrets and verify none are all-zeros
        for _ in 0..100 {
            let secret = ClusterSecret::generate();
            assert!(!secret.bytes.iter().all(|&b| b == 0));
        }
    }

    #[test]
    fn test_from_bytes_rejects_all_zeros() {
        assert!(ClusterSecret::from_bytes([0u8; 32]).is_none());
    }

    #[test]
    fn test_from_bytes_accepts_nonzero() {
        let mut bytes = [0u8; 32];
        bytes[0] = 1;
        assert!(ClusterSecret::from_bytes(bytes).is_some());
    }

    #[test]
    fn test_constant_time_equality() {
        let a = ClusterSecret::from_bytes([1u8; 32]).unwrap();
        let b = ClusterSecret::from_bytes([1u8; 32]).unwrap();
        let c = ClusterSecret::from_bytes([2u8; 32]).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_debug_redacts() {
        let secret = ClusterSecret::from_bytes([0xAB; 32]).unwrap();
        let debug = format!("{secret:?}");
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("AB"));
    }

    #[test]
    fn test_threshold_default_for_cluster_size() {
        assert_eq!(Threshold::default_for_cluster_size(1).value(), 1);
        assert_eq!(Threshold::default_for_cluster_size(3).value(), 2);
        assert_eq!(Threshold::default_for_cluster_size(5).value(), 3);
        assert_eq!(Threshold::default_for_cluster_size(7).value(), 4);
    }

    #[test]
    fn test_threshold_new_rejects_zero() {
        assert!(Threshold::new(0).is_none());
        assert!(Threshold::new(1).is_some());
    }
}
