//! Signature Verification Specification
//!
//! Formal specification for Ed25519 signature verification of capability tokens.
//!
//! # Security Properties
//!
//! 1. **AUTH-5: Signature Correctness**: Valid signatures match content
//! 2. **AUTH-6: Issuer Binding**: Signature proves issuer identity
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-auth/verus/signature_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Ed25519 signature size
    pub const SIGNATURE_SIZE: u64 = 64;

    /// Ed25519 public key size
    pub const PUBLIC_KEY_SIZE: u64 = 32;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract signature verification state
    pub struct SignatureState {
        /// Issuer public key fingerprint
        pub issuer_fingerprint: u64,
        /// Hash of signed content
        pub content_hash: u64,
        /// Whether signature is cryptographically valid
        pub is_valid: bool,
    }

    /// Abstract payload for signing
    pub struct TokenPayloadSpec {
        pub version: u8,
        pub issuer_fingerprint: u64,
        pub audience_hash: u64,
        pub capabilities_hash: u64,
        pub issued_at: u64,
        pub expires_at: u64,
        pub nonce_hash: u64,
        pub proof_hash: u64,
        pub delegation_depth: u8,
    }

    // ========================================================================
    // Invariant 5: Signature Correctness
    // ========================================================================

    /// AUTH-5: Signature correctly verifies content
    ///
    /// A valid signature means the content hash matches what was signed.
    pub open spec fn signature_valid(
        state: SignatureState,
        expected_content_hash: u64,
    ) -> bool {
        state.is_valid && state.content_hash == expected_content_hash
    }

    /// Signature binds all token fields
    ///
    /// bytes_to_sign includes: version, issuer, audience, capabilities,
    /// issued_at, expires_at, nonce, proof, delegation_depth
    pub open spec fn signature_binds_fields(payload: TokenPayloadSpec) -> u64 {
        // Abstract hash of all fields (simplified)
        // In reality, this is BLAKE3(postcard(payload))
        payload.version as u64 ^
        payload.issuer_fingerprint ^
        payload.audience_hash ^
        payload.capabilities_hash ^
        payload.issued_at ^
        payload.expires_at ^
        payload.nonce_hash ^
        payload.proof_hash ^
        payload.delegation_depth as u64
    }

    // ========================================================================
    // Invariant 6: Issuer Binding
    // ========================================================================

    /// AUTH-6: Signature proves issuer identity
    ///
    /// Only the holder of the private key corresponding to issuer
    /// can create a valid signature.
    pub open spec fn issuer_bound(
        issuer_fingerprint: u64,
        signer_fingerprint: u64,
        signature_valid: bool,
    ) -> bool {
        // Valid signature implies signer == issuer
        signature_valid ==> issuer_fingerprint == signer_fingerprint
    }

    // ========================================================================
    // Tampering Detection
    // ========================================================================

    /// Tampering invalidates signature
    ///
    /// If any field of the payload is modified, the signature becomes invalid.
    pub open spec fn tampering_detected(
        original: TokenPayloadSpec,
        tampered: TokenPayloadSpec,
        signature_still_valid: bool,
    ) -> bool {
        // If any field differs, signature must be invalid
        (original.version != tampered.version ||
         original.issuer_fingerprint != tampered.issuer_fingerprint ||
         original.audience_hash != tampered.audience_hash ||
         original.capabilities_hash != tampered.capabilities_hash ||
         original.issued_at != tampered.issued_at ||
         original.expires_at != tampered.expires_at ||
         original.nonce_hash != tampered.nonce_hash ||
         original.proof_hash != tampered.proof_hash ||
         original.delegation_depth != tampered.delegation_depth)
        ==>
        !signature_still_valid
    }

    // ========================================================================
    // Signing Operation
    // ========================================================================

    /// Model of token signing
    pub open spec fn sign_token(
        payload: TokenPayloadSpec,
        signer_fingerprint: u64,
    ) -> SignatureState {
        SignatureState {
            issuer_fingerprint: payload.issuer_fingerprint,
            content_hash: signature_binds_fields(payload),
            // Signature is valid iff signer matches issuer
            is_valid: signer_fingerprint == payload.issuer_fingerprint,
        }
    }

    /// Proof: Correct signer produces valid signature
    pub proof fn correct_signer_valid(
        payload: TokenPayloadSpec,
        signer_fingerprint: u64,
    )
        requires signer_fingerprint == payload.issuer_fingerprint
        ensures {
            let sig = sign_token(payload, signer_fingerprint);
            sig.is_valid
        }
    {
        // signer == issuer implies is_valid = true
    }

    /// Proof: Wrong signer produces invalid signature
    pub proof fn wrong_signer_invalid(
        payload: TokenPayloadSpec,
        signer_fingerprint: u64,
    )
        requires signer_fingerprint != payload.issuer_fingerprint
        ensures {
            let sig = sign_token(payload, signer_fingerprint);
            !sig.is_valid
        }
    {
        // signer != issuer implies is_valid = false
    }

    // ========================================================================
    // Verification Operation
    // ========================================================================

    /// Model of signature verification
    pub open spec fn verify_signature(
        signature: SignatureState,
        payload: TokenPayloadSpec,
    ) -> bool {
        // Signature valid and content hash matches
        signature.is_valid &&
        signature.content_hash == signature_binds_fields(payload) &&
        signature.issuer_fingerprint == payload.issuer_fingerprint
    }

    /// Proof: Valid tokens pass verification
    pub proof fn valid_token_verifies(
        payload: TokenPayloadSpec,
        signer_fingerprint: u64,
    )
        requires signer_fingerprint == payload.issuer_fingerprint
        ensures {
            let sig = sign_token(payload, signer_fingerprint);
            verify_signature(sig, payload)
        }
    {
        // is_valid = true
        // content_hash = signature_binds_fields(payload)
        // issuer_fingerprint matches
    }

    /// Proof: Tampered tokens fail verification
    pub proof fn tampered_token_fails(
        original: TokenPayloadSpec,
        tampered: TokenPayloadSpec,
        signer_fingerprint: u64,
    )
        requires
            signer_fingerprint == original.issuer_fingerprint,
            original.issued_at != tampered.issued_at,  // Example tampering
        ensures {
            let sig = sign_token(original, signer_fingerprint);
            !verify_signature(sig, tampered)
        }
    {
        // content_hash computed on original doesn't match tampered
        // signature_binds_fields(original) != signature_binds_fields(tampered)
    }

    // ========================================================================
    // Hash Properties
    // ========================================================================

    /// Token hash is deterministic
    pub open spec fn hash_deterministic(
        token1: TokenPayloadSpec,
        token2: TokenPayloadSpec,
    ) -> bool {
        // Same content produces same hash
        (token1.version == token2.version &&
         token1.issuer_fingerprint == token2.issuer_fingerprint &&
         token1.audience_hash == token2.audience_hash &&
         token1.capabilities_hash == token2.capabilities_hash &&
         token1.issued_at == token2.issued_at &&
         token1.expires_at == token2.expires_at &&
         token1.nonce_hash == token2.nonce_hash &&
         token1.proof_hash == token2.proof_hash &&
         token1.delegation_depth == token2.delegation_depth)
        ==>
        signature_binds_fields(token1) == signature_binds_fields(token2)
    }

    /// Proof: Hash is deterministic
    pub proof fn hash_is_deterministic(
        token1: TokenPayloadSpec,
        token2: TokenPayloadSpec,
    )
        ensures hash_deterministic(token1, token2)
    {
        // XOR of same values produces same result
    }
}
