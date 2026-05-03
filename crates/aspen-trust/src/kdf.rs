//! HKDF-SHA3-256 key derivation.
//!
//! Derives purpose-specific 32-byte keys from the cluster root secret.
//! Each derived key is bound to a context string, cluster ID, and epoch,
//! preventing cross-purpose key reuse (confused deputy).
//!
//! Pattern from Oxide's trust-quorum: `b"aspen-v1-<purpose>"` + cluster_id + epoch,
//! encoded with explicit field lengths so variable-width fields cannot collide.

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

const INFO_DOMAIN: &[u8] = b"aspen-hkdf-info-v1";

fn append_len_prefixed(info: &mut Vec<u8>, value: &[u8]) {
    let len = u64::try_from(value.len()).expect("slice length must fit in u64");
    info.extend_from_slice(&len.to_be_bytes());
    info.extend_from_slice(value);
}

fn encoded_info(context: &[u8], cluster_id: &[u8], epoch: u64) -> Vec<u8> {
    let len_prefix_bytes = 8usize;
    let epoch_bytes = epoch.to_be_bytes();
    let info_len = INFO_DOMAIN
        .len()
        .saturating_add(len_prefix_bytes)
        .saturating_add(context.len())
        .saturating_add(len_prefix_bytes)
        .saturating_add(cluster_id.len())
        .saturating_add(epoch_bytes.len());
    let mut info = Vec::with_capacity(info_len);
    info.extend_from_slice(INFO_DOMAIN);
    append_len_prefixed(&mut info, context);
    append_len_prefixed(&mut info, cluster_id);
    info.extend_from_slice(&epoch_bytes);
    info
}

/// Derive a 32-byte key from the cluster secret using HKDF-SHA3-256.
///
/// The `info` parameter is structurally encoded from:
/// - a fixed Aspen HKDF info-domain tag
/// - `context`: purpose string (e.g., `CONTEXT_SECRETS_AT_REST`), length-prefixed
/// - `cluster_id`: unique cluster identifier, length-prefixed
/// - `epoch`: key rotation epoch (monotonically increasing), fixed-width big-endian
///
/// The cluster secret is used as the HKDF input keying material (IKM).
/// No salt is used (HKDF extracts from IKM directly).
///
/// # Panics
/// Panics if the derived key is all zeros (HKDF implementation bug).
pub fn derive_key(secret: &[u8; 32], context: &[u8], cluster_id: &[u8], epoch: u64) -> [u8; 32] {
    let mut info = encoded_info(context, cluster_id, epoch);

    // HKDF-SHA3-256: extract then expand
    let hk = Hkdf::<Sha3_256>::new(None, secret);
    let mut output = [0u8; 32];
    if hk.expand(&info, &mut output).is_err() {
        // HKDF-SHA3-256 with 32-byte output cannot fail (max: 255 × 32 = 8160 bytes).
        // The all-zeros safety assertion below will catch this impossible case.
        debug_assert!(false, "HKDF-SHA3-256 with 32-byte output cannot fail");
    }

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
    fn test_ambiguous_context_cluster_splits_produce_different_keys() {
        let secret = test_secret();
        let k1 = derive_key(&secret, b"ab", b"c", 0);
        let k2 = derive_key(&secret, b"a", b"bc", 0);
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_encoded_info_includes_length_boundaries() {
        let i1 = encoded_info(b"ab", b"c", 0);
        let i2 = encoded_info(b"a", b"bc", 0);

        assert_ne!(i1, i2);
        assert!(i1.starts_with(INFO_DOMAIN));
        assert!(i2.starts_with(INFO_DOMAIN));
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
