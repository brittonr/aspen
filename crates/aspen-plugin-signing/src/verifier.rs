//! Plugin signature verification.

use ed25519_dalek::Verifier;
use ed25519_dalek::VerifyingKey;

use crate::error::SigningError;
use crate::signature::PluginSignature;

/// Verify a plugin signature against the WASM binary.
///
/// Steps:
/// 1. Compute `BLAKE3(wasm_bytes)` and check it matches `sig.wasm_hash`
/// 2. Decode the public key and signature from hex
/// 3. Verify the Ed25519 signature over the hash
pub fn verify_plugin(wasm_bytes: &[u8], sig: &PluginSignature) -> Result<(), SigningError> {
    // Step 1: Check hash matches
    let actual_hash = blake3::hash(wasm_bytes);
    let actual_hash_hex = actual_hash.to_hex().to_string();
    if actual_hash_hex != sig.wasm_hash {
        return Err(SigningError::HashMismatch {
            expected: sig.wasm_hash.clone(),
            actual: actual_hash_hex,
        });
    }

    // Step 2: Decode public key
    let pubkey_bytes = hex::decode(&sig.author_pubkey)?;
    if pubkey_bytes.len() != 32 {
        return Err(SigningError::InvalidPublicKeyLength(pubkey_bytes.len()));
    }
    let pubkey_array: [u8; 32] = pubkey_bytes.try_into().expect("length checked above");
    let verifying_key = VerifyingKey::from_bytes(&pubkey_array)?;

    // Step 3: Decode and verify signature
    let sig_bytes = hex::decode(&sig.signature)?;
    if sig_bytes.len() != 64 {
        return Err(SigningError::InvalidSignatureLength(sig_bytes.len()));
    }
    let sig_array: [u8; 64] = sig_bytes.try_into().expect("length checked above");
    let ed_sig = ed25519_dalek::Signature::from_bytes(&sig_array);

    verifying_key.verify(actual_hash.as_bytes(), &ed_sig).map_err(|_| SigningError::VerificationFailed)
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::SigningKey;

    use super::*;
    use crate::signer::sign_plugin;

    #[test]
    fn sign_then_verify_roundtrip() {
        let key = SigningKey::generate(&mut rand_core::OsRng);
        let wasm = b"valid wasm plugin binary";
        let sig = sign_plugin(wasm, &key);
        assert!(verify_plugin(wasm, &sig).is_ok());
    }

    #[test]
    fn tampered_bytes_fail() {
        let key = SigningKey::generate(&mut rand_core::OsRng);
        let wasm = b"original content";
        let sig = sign_plugin(wasm, &key);

        let tampered = b"tampered content";
        let err = verify_plugin(tampered, &sig).unwrap_err();
        assert!(matches!(err, SigningError::HashMismatch { .. }));
    }

    #[test]
    fn wrong_key_fails() {
        let key1 = SigningKey::generate(&mut rand_core::OsRng);
        let key2 = SigningKey::generate(&mut rand_core::OsRng);
        let wasm = b"signed by key1";
        let mut sig = sign_plugin(wasm, &key1);

        // Replace pubkey with key2's pubkey — signature won't match
        sig.author_pubkey = hex::encode(key2.verifying_key().as_bytes());
        let err = verify_plugin(wasm, &sig).unwrap_err();
        assert!(matches!(err, SigningError::VerificationFailed));
    }

    #[test]
    fn invalid_hex_pubkey() {
        let sig = PluginSignature {
            author_pubkey: "not-valid-hex!".into(),
            signature: "aa".repeat(64),
            wasm_hash: blake3::hash(b"x").to_hex().to_string(),
            signed_at_ms: 0,
        };
        let err = verify_plugin(b"x", &sig).unwrap_err();
        assert!(matches!(err, SigningError::InvalidHex(_)));
    }

    #[test]
    fn wrong_length_pubkey() {
        let sig = PluginSignature {
            author_pubkey: hex::encode([0u8; 16]), // 16 bytes, not 32
            signature: "aa".repeat(64),
            wasm_hash: blake3::hash(b"x").to_hex().to_string(),
            signed_at_ms: 0,
        };
        let err = verify_plugin(b"x", &sig).unwrap_err();
        assert!(matches!(err, SigningError::InvalidPublicKeyLength(16)));
    }

    #[test]
    fn wrong_length_signature() {
        let key = SigningKey::generate(&mut rand_core::OsRng);
        let sig = PluginSignature {
            author_pubkey: hex::encode(key.verifying_key().as_bytes()),
            signature: hex::encode([0u8; 32]), // 32 bytes, not 64
            wasm_hash: blake3::hash(b"x").to_hex().to_string(),
            signed_at_ms: 0,
        };
        let err = verify_plugin(b"x", &sig).unwrap_err();
        assert!(matches!(err, SigningError::InvalidSignatureLength(32)));
    }
}
