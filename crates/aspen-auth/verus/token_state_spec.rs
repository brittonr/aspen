//! Token State Model and Invariants
//!
//! Abstract state model for formal verification of capability tokens.
//!
//! # State Model
//!
//! The token state captures:
//! - Issuer public key (32 bytes)
//! - Capabilities (bounded list)
//! - Timestamps (issued_at, expires_at)
//! - Delegation proof (optional parent hash)
//! - Signature (64 bytes)
//!
//! # Key Invariants
//!
//! 1. **AUTH-1: Capability Bounds**: capabilities <= MAX_CAPABILITIES
//! 2. **AUTH-2: Token Size Bounds**: encoded_size <= MAX_TOKEN_SIZE
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-auth/verus/token_state_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum capabilities per token
    pub const MAX_CAPABILITIES_PER_TOKEN: u64 = 32;

    /// Maximum token size in bytes (8 KB)
    pub const MAX_TOKEN_SIZE: u64 = 8 * 1024;

    /// Token version
    pub const TOKEN_VERSION: u8 = 1;

    /// Public key size in bytes
    pub const PUBLIC_KEY_SIZE: u64 = 32;

    /// Signature size in bytes
    pub const SIGNATURE_SIZE: u64 = 64;

    /// Nonce size in bytes
    pub const NONCE_SIZE: u64 = 16;

    /// Proof hash size in bytes
    pub const PROOF_HASH_SIZE: u64 = 32;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract token state
    ///
    /// Models CapabilityToken from token.rs
    pub struct TokenState {
        /// Token version
        pub version: u8,
        /// Number of capabilities
        pub capability_count: u64,
        /// Estimated serialized size in bytes
        pub encoded_size: u64,
        /// Unix timestamp when issued
        pub issued_at: u64,
        /// Unix timestamp when expires
        pub expires_at: u64,
        /// Whether token has a nonce
        pub has_nonce: bool,
        /// Whether token has a proof (delegation parent hash)
        pub has_proof: bool,
        /// Delegation depth (0 = root)
        pub delegation_depth: u8,
        /// Whether signature is present and non-zero
        pub has_signature: bool,
    }

    /// Abstract capability type
    ///
    /// Models Capability enum from capability.rs
    pub enum CapabilitySpec {
        /// Full access to key prefix
        Full { prefix_len: u64 },
        /// Read-only access to key prefix
        Read { prefix_len: u64 },
        /// Write-only access to key prefix
        Write { prefix_len: u64 },
        /// Can delete keys with prefix
        Delete { prefix_len: u64 },
        /// Can delegate capabilities
        Delegate,
    }

    // ========================================================================
    // Invariant 1: Capability Bounds
    // ========================================================================

    /// AUTH-1: Capability count is bounded
    pub open spec fn capability_bounds(token: TokenState) -> bool {
        token.capability_count <= MAX_CAPABILITIES_PER_TOKEN
    }

    // ========================================================================
    // Invariant 2: Token Size Bounds
    // ========================================================================

    /// AUTH-2: Encoded token size is bounded
    pub open spec fn token_size_bounds(token: TokenState) -> bool {
        token.encoded_size <= MAX_TOKEN_SIZE
    }

    /// Estimate minimum token size
    ///
    /// version (1) + issuer (32) + audience (varies) + caps (varies) +
    /// timestamps (16) + optional nonce (17) + optional proof (33) +
    /// depth (1) + signature (64)
    pub open spec fn estimate_min_token_size(token: TokenState) -> u64 {
        let base = 1 + PUBLIC_KEY_SIZE + 16 + 1 + SIGNATURE_SIZE;  // ~114 bytes
        let nonce_size = if token.has_nonce { 1 + NONCE_SIZE } else { 1 };
        let proof_size = if token.has_proof { 1 + PROOF_HASH_SIZE } else { 1 };
        base + nonce_size + proof_size
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined token invariant
    pub open spec fn token_invariant(token: TokenState) -> bool {
        // Version must be compatible
        token.version <= TOKEN_VERSION &&
        // Capabilities bounded
        capability_bounds(token) &&
        // Size bounded
        token_size_bounds(token) &&
        // Valid timestamp ordering
        token.issued_at <= token.expires_at &&
        // Signature must be present
        token.has_signature
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial token state (before capabilities added)
    pub open spec fn initial_token_state(
        issued_at: u64,
        expires_at: u64,
    ) -> TokenState {
        TokenState {
            version: TOKEN_VERSION,
            capability_count: 0,
            encoded_size: 114,  // Approximate base size
            issued_at,
            expires_at,
            has_nonce: false,
            has_proof: false,
            delegation_depth: 0,
            has_signature: false,  // Not signed yet
        }
    }

    /// Proof: Initial state before signing doesn't satisfy full invariant
    /// (signature required)
    pub proof fn initial_state_needs_signature(
        issued_at: u64,
        expires_at: u64,
    )
        requires issued_at <= expires_at
        ensures !token_invariant(initial_token_state(issued_at, expires_at))
    {
        // has_signature = false violates invariant
    }

    // ========================================================================
    // Builder Operations
    // ========================================================================

    /// Effect of adding a capability
    pub open spec fn add_capability_effect(
        token: TokenState,
        cap_size: u64,
    ) -> Option<TokenState> {
        if token.capability_count >= MAX_CAPABILITIES_PER_TOKEN {
            None  // Would exceed capability limit
        } else {
            let new_size = token.encoded_size + cap_size;
            if new_size > MAX_TOKEN_SIZE {
                None  // Would exceed size limit
            } else {
                Some(TokenState {
                    capability_count: token.capability_count + 1,
                    encoded_size: new_size,
                    ..token
                })
            }
        }
    }

    /// Effect of signing the token
    pub open spec fn sign_effect(token: TokenState) -> TokenState {
        TokenState {
            has_signature: true,
            ..token
        }
    }

    /// Effect of adding delegation proof
    pub open spec fn add_proof_effect(
        token: TokenState,
        parent_depth: u8,
    ) -> TokenState {
        TokenState {
            has_proof: true,
            delegation_depth: (parent_depth + 1) as u8,
            encoded_size: token.encoded_size + PROOF_HASH_SIZE,
            ..token
        }
    }

    /// Effect of adding nonce
    pub open spec fn add_nonce_effect(token: TokenState) -> TokenState {
        TokenState {
            has_nonce: true,
            encoded_size: token.encoded_size + NONCE_SIZE,
            ..token
        }
    }

    // ========================================================================
    // Operation Proofs
    // ========================================================================

    /// Proof: Adding capability preserves bounds
    pub proof fn add_capability_preserves_bounds(
        token: TokenState,
        cap_size: u64,
    )
        requires
            token.capability_count < MAX_CAPABILITIES_PER_TOKEN,
            token.encoded_size + cap_size <= MAX_TOKEN_SIZE,
        ensures {
            let post = add_capability_effect(token, cap_size).unwrap();
            capability_bounds(post) && token_size_bounds(post)
        }
    {
        // capability_count + 1 <= MAX_CAPABILITIES_PER_TOKEN
        // encoded_size + cap_size <= MAX_TOKEN_SIZE
    }

    /// Proof: Signing creates valid token
    pub proof fn signing_creates_valid_token(token: TokenState)
        requires
            token.version <= TOKEN_VERSION,
            capability_bounds(token),
            token_size_bounds(token),
            token.issued_at <= token.expires_at,
        ensures token_invariant(sign_effect(token))
    {
        // sign_effect sets has_signature = true
        // All other invariants preserved
    }

    /// Proof: Adding capabilities can fail at limit
    pub proof fn add_capability_fails_at_limit(token: TokenState)
        requires token.capability_count == MAX_CAPABILITIES_PER_TOKEN
        ensures add_capability_effect(token, 1).is_none()
    {
        // capability_count >= MAX_CAPABILITIES_PER_TOKEN
    }

    // ========================================================================
    // Audience Properties
    // ========================================================================

    /// Audience type enumeration
    pub enum AudienceSpec {
        /// Token bound to specific public key
        Key,
        /// Bearer token (anyone can use)
        Bearer,
    }

    /// Bearer tokens don't require presenter verification
    pub open spec fn bearer_needs_no_presenter(aud: AudienceSpec) -> bool {
        match aud {
            AudienceSpec::Bearer => true,
            AudienceSpec::Key => false,
        }
    }

    /// Key-bound tokens require matching presenter
    pub open spec fn key_requires_matching_presenter(
        aud: AudienceSpec,
        presenter_matches: bool,
    ) -> bool {
        match aud {
            AudienceSpec::Key => presenter_matches,
            AudienceSpec::Bearer => true,  // Always passes
        }
    }
}
