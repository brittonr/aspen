//! Plugin signing using Ed25519 + BLAKE3.

use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;

use crate::signature::PluginSignature;

/// Sign a WASM plugin binary.
///
/// Computes `BLAKE3(wasm_bytes)` and signs the hash with the provided
/// Ed25519 secret key. Returns a [`PluginSignature`] containing all
/// hex-encoded fields needed for verification.
pub fn sign_plugin(wasm_bytes: &[u8], signing_key: &SigningKey) -> PluginSignature {
    let hash = blake3::hash(wasm_bytes);
    let signature = signing_key.sign(hash.as_bytes());
    let pubkey = signing_key.verifying_key();

    let now_ms =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

    PluginSignature {
        author_pubkey: hex::encode(pubkey.as_bytes()),
        signature: hex::encode(signature.to_bytes()),
        wasm_hash: hash.to_hex().to_string(),
        signed_at_ms: now_ms,
    }
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::SigningKey;

    use super::*;

    #[test]
    fn sign_produces_valid_lengths() {
        let key = SigningKey::generate(&mut rand_core::OsRng);
        let wasm = b"fake wasm binary content";
        let sig = sign_plugin(wasm, &key);

        assert_eq!(sig.author_pubkey.len(), 64, "pubkey should be 64 hex chars");
        assert_eq!(sig.signature.len(), 128, "signature should be 128 hex chars");
        assert_eq!(sig.wasm_hash.len(), 64, "hash should be 64 hex chars");
        assert!(sig.signed_at_ms > 0, "timestamp should be nonzero");
    }

    #[test]
    fn sign_is_deterministic_for_same_content() {
        let key = SigningKey::generate(&mut rand_core::OsRng);
        let wasm = b"deterministic test content";
        let sig1 = sign_plugin(wasm, &key);
        let sig2 = sign_plugin(wasm, &key);

        assert_eq!(sig1.wasm_hash, sig2.wasm_hash);
        assert_eq!(sig1.signature, sig2.signature);
        assert_eq!(sig1.author_pubkey, sig2.author_pubkey);
    }
}
