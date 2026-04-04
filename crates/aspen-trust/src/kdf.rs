//! HKDF-SHA3-256 key derivation.
//!
//! Derives purpose-specific 32-byte keys from the cluster root secret.
//! Each derived key is bound to a context string, cluster ID, and epoch,
//! preventing cross-purpose key reuse (confused deputy).
//!
//! Pattern from Oxide's trust-quorum: `b"aspen-v1-<purpose>"` + cluster_id + epoch.

use hkdf::Hkdf;
use sha3::Sha3_256;
use zeroize::Zeroize;

// ============================================================================
// Standard context constants
// ============================================================================

/// Context for deriving keys used to encrypt secrets at rest in redb.
pub const CONTEXT_SECRETS_AT_REST: &[u8] = b"aspen-v1-secrets-at-rest";

/// Context for deriving transit encryption keys (inter-node secret transfer).
pub const CONTEXT_TRANSIT_KEYS: &[u8] = b"aspen-v1-transit-keys";

/// Context for deriving rack-level secrets (physical topology isolation).
pub const CONTEXT_RACK_SECRETS: &[u8] = b"aspen-v1-rack-secrets";

/// Derive a 32-byte key from the cluster secret using HKDF-SHA3-256.
///
/// The `info` parameter is constructed from:
/// - `context`: purpose string (e.g., `CONTEXT_SECRETS_AT_REST`)
/// - `cluster_id`: unique cluster identifier
/// - `epoch`: key rotation epoch (monotonically increasing)
///
/// The cluster secret is used as the HKDF input keying material (IKM).
/// No salt is used (HKDF extracts from IKM directly).
///
/// # Panics
/// Panics if the derived key is all zeros (HKDF implementation bug).
pub fn derive_key(secret: &[u8; 32], context: &[u8], cluster_id: &[u8], epoch: u64) -> [u8; 32] {
    // Build the info parameter: context || cluster_id || epoch (big-endian)
    let epoch_bytes = epoch.to_be_bytes();
    let info_len = context.len() + cluster_id.len() + epoch_bytes.len();
    let mut info = Vec::with_capacity(info_len);
    info.extend_from_slice(context);
    info.extend_from_slice(cluster_id);
    info.extend_from_slice(&epoch_bytes);

    // HKDF-SHA3-256: extract then expand
    let hk = Hkdf::<Sha3_256>::new(None, secret);
    let mut output = [0u8; 32];
    hk.expand(&info, &mut output).expect("32-byte output is within HKDF-SHA3-256 limits");

    // Safety invariant: derived key must not be all zeros
    assert!(!output.iter().all(|&b| b == 0), "derived key is all zeros (HKDF implementation bug)");

    // Zeroize intermediate info
    info.zeroize();

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_secret() -> [u8; 32] {
        let mut s = [0u8; 32];
        s[0] = 0xDE;
        s[1] = 0xAD;
        s
    }

    #[test]
    fn test_deterministic_derivation() {
        let secret = test_secret();
        let k1 = derive_key(&secret, CONTEXT_SECRETS_AT_REST, b"cluster-1", 0);
        let k2 = derive_key(&secret, CONTEXT_SECRETS_AT_REST, b"cluster-1", 0);
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_different_contexts_produce_different_keys() {
        let secret = test_secret();
        let k1 = derive_key(&secret, CONTEXT_SECRETS_AT_REST, b"cluster-1", 0);
        let k2 = derive_key(&secret, CONTEXT_TRANSIT_KEYS, b"cluster-1", 0);
        let k3 = derive_key(&secret, CONTEXT_RACK_SECRETS, b"cluster-1", 0);
        assert_ne!(k1, k2);
        assert_ne!(k1, k3);
        assert_ne!(k2, k3);
    }

    #[test]
    fn test_different_cluster_ids_produce_different_keys() {
        let secret = test_secret();
        let k1 = derive_key(&secret, CONTEXT_SECRETS_AT_REST, b"cluster-1", 0);
        let k2 = derive_key(&secret, CONTEXT_SECRETS_AT_REST, b"cluster-2", 0);
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_different_epochs_produce_different_keys() {
        let secret = test_secret();
        let k1 = derive_key(&secret, CONTEXT_SECRETS_AT_REST, b"cluster-1", 0);
        let k2 = derive_key(&secret, CONTEXT_SECRETS_AT_REST, b"cluster-1", 1);
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_derived_key_not_all_zeros() {
        let secret = test_secret();
        for epoch in 0u64..100 {
            let key = derive_key(&secret, CONTEXT_SECRETS_AT_REST, b"test", epoch);
            assert!(!key.iter().all(|&b| b == 0));
        }
    }

    #[test]
    fn test_different_secrets_produce_different_keys() {
        let mut s1 = [0u8; 32];
        s1[0] = 1;
        let mut s2 = [0u8; 32];
        s2[0] = 2;
        let k1 = derive_key(&s1, CONTEXT_SECRETS_AT_REST, b"cluster-1", 0);
        let k2 = derive_key(&s2, CONTEXT_SECRETS_AT_REST, b"cluster-1", 0);
        assert_ne!(k1, k2);
    }
}
