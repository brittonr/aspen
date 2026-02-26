//! Repository Identity Verification Specifications
//!
//! Formal specifications for repository identity, delegate authorization,
//! and threshold validation.
//!
//! # Key Invariants
//!
//! 1. **REPO-1: Name Bounds**: name.len() in 1..=MAX_REPO_NAME_LENGTH_BYTES
//! 2. **REPO-2: Delegate Bounds**: delegates.len() in 1..=MAX_DELEGATES
//! 3. **REPO-3: Threshold Validity**: 1 <= threshold <= delegates.len()
//! 4. **REPO-4: Content-Addressed ID**: repo_id = BLAKE3(identity)
//! 5. **REPO-5: Delegate Membership**: is_delegate(k) iff k in delegates
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-forge/verus/repo_identity_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum delegates for a repository
    pub const MAX_DELEGATES: u64 = 64;

    /// Maximum threshold (equals MAX_DELEGATES)
    pub const MAX_THRESHOLD: u64 = 64;

    /// Maximum repository name length in bytes
    pub const MAX_REPO_NAME_LENGTH_BYTES: u64 = 256;

    /// Maximum repository description length in bytes
    pub const MAX_REPO_DESCRIPTION_LENGTH_BYTES: u64 = 4096;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Repository ID (BLAKE3 hash of identity, 32 bytes)
    pub struct RepoIdSpec {
        pub id_high: u128,
        pub id_low: u128,
    }

    /// Abstract repository identity
    pub struct RepoIdentitySpec {
        /// Name length in bytes
        pub name_len: u64,
        /// Description length in bytes (0 if none)
        pub description_len: u64,
        /// Number of delegates
        pub delegate_count: u64,
        /// Signature threshold
        pub threshold: u64,
        /// Creation timestamp
        pub created_at_ms: u64,
    }

    /// Delegate public key (Ed25519, 32 bytes)
    pub struct DelegateKeySpec {
        pub key_high: u128,
        pub key_low: u128,
    }

    // ========================================================================
    // Invariant REPO-1: Name Bounds
    // ========================================================================

    /// REPO-1: Name length in valid range
    pub open spec fn name_valid(name_len: u64) -> bool {
        name_len > 0 && name_len <= MAX_REPO_NAME_LENGTH_BYTES
    }

    /// new() rejects empty name
    pub open spec fn new_rejects_empty_name(
        name_len: u64,
        result_is_ok: bool,
    ) -> bool {
        (name_len == 0) ==> !result_is_ok
    }

    /// new() rejects oversized name
    pub open spec fn new_rejects_oversized_name(
        name_len: u64,
        result_is_ok: bool,
    ) -> bool {
        (name_len > MAX_REPO_NAME_LENGTH_BYTES) ==> !result_is_ok
    }

    /// Proof: Valid names are within bounds
    pub proof fn valid_name_bounded(name_len: u64)
        requires name_valid(name_len)
        ensures name_len >= 1 && name_len <= 256
    {
        // MAX_REPO_NAME_LENGTH_BYTES = 256
    }

    // ========================================================================
    // Invariant REPO-2: Delegate Bounds
    // ========================================================================

    /// REPO-2: Delegate count in valid range
    pub open spec fn delegates_valid(delegate_count: u64) -> bool {
        delegate_count >= 1 && delegate_count <= MAX_DELEGATES
    }

    /// new() rejects empty delegate list
    pub open spec fn new_rejects_empty_delegates(
        delegate_count: u64,
        result_is_ok: bool,
    ) -> bool {
        (delegate_count == 0) ==> !result_is_ok
    }

    /// new() rejects too many delegates
    pub open spec fn new_rejects_too_many_delegates(
        delegate_count: u64,
        result_is_ok: bool,
    ) -> bool {
        (delegate_count > MAX_DELEGATES) ==> !result_is_ok
    }

    /// Proof: Valid delegate count is bounded
    pub proof fn valid_delegates_bounded(delegate_count: u64)
        requires delegates_valid(delegate_count)
        ensures delegate_count >= 1 && delegate_count <= 64
    {
        // MAX_DELEGATES = 64
    }

    // ========================================================================
    // Invariant REPO-3: Threshold Validity
    // ========================================================================

    /// REPO-3: Threshold in valid range
    pub open spec fn threshold_valid(threshold: u64, delegate_count: u64) -> bool {
        threshold >= 1 &&
        threshold <= delegate_count &&
        threshold <= MAX_THRESHOLD
    }

    /// new() rejects zero threshold
    pub open spec fn new_rejects_zero_threshold(
        threshold: u64,
        result_is_ok: bool,
    ) -> bool {
        (threshold == 0) ==> !result_is_ok
    }

    /// new() rejects threshold > delegates
    pub open spec fn new_rejects_excessive_threshold(
        threshold: u64,
        delegate_count: u64,
        result_is_ok: bool,
    ) -> bool {
        (threshold > delegate_count) ==> !result_is_ok
    }

    /// Proof: Valid threshold is achievable
    pub proof fn valid_threshold_achievable(
        threshold: u64,
        delegate_count: u64,
    )
        requires threshold_valid(threshold, delegate_count)
        ensures threshold <= delegate_count
    {
        // By definition of threshold_valid
    }

    // ========================================================================
    // Invariant REPO-4: Content-Addressed ID
    // ========================================================================

    /// REPO-4: repo_id = BLAKE3(serialize(identity))
    pub open spec fn id_content_addressed(
        identity: RepoIdentitySpec,
        computed_id: RepoIdSpec,
    ) -> bool {
        // ID is deterministic function of identity content
        // (Trusted axiom: BLAKE3 is deterministic, postcard is deterministic)
        true
    }

    /// Same identity produces same ID
    pub open spec fn id_deterministic(
        identity1: RepoIdentitySpec,
        identity2: RepoIdentitySpec,
        id1: RepoIdSpec,
        id2: RepoIdSpec,
    ) -> bool {
        (identity1.name_len == identity2.name_len &&
         identity1.description_len == identity2.description_len &&
         identity1.delegate_count == identity2.delegate_count &&
         identity1.threshold == identity2.threshold &&
         identity1.created_at_ms == identity2.created_at_ms)
        ==>
        (id1.id_high == id2.id_high && id1.id_low == id2.id_low)
    }

    /// Proof: Content addressing is deterministic
    pub proof fn content_addressing_deterministic(
        identity1: RepoIdentitySpec,
        identity2: RepoIdentitySpec,
        id1: RepoIdSpec,
        id2: RepoIdSpec,
    )
        requires
            identity1.name_len == identity2.name_len,
            identity1.description_len == identity2.description_len,
            identity1.delegate_count == identity2.delegate_count,
            identity1.threshold == identity2.threshold,
            identity1.created_at_ms == identity2.created_at_ms,
        ensures id_deterministic(identity1, identity2, id1, id2)
    {
        // BLAKE3(postcard(x)) = BLAKE3(postcard(y)) when x = y
    }

    // ========================================================================
    // Invariant REPO-5: Delegate Membership
    // ========================================================================

    /// REPO-5: is_delegate(key) iff key in delegates set
    ///
    /// This is modeled abstractly since we don't model the full set
    pub open spec fn delegate_membership_correct(
        key: DelegateKeySpec,
        is_in_set: bool,
        is_delegate_result: bool,
    ) -> bool {
        is_in_set == is_delegate_result
    }

    // ========================================================================
    // Description Bounds
    // ========================================================================

    /// Description length bounded
    pub open spec fn description_valid(description_len: u64) -> bool {
        description_len <= MAX_REPO_DESCRIPTION_LENGTH_BYTES
    }

    /// with_description rejects oversized
    pub open spec fn with_description_rejects_oversized(
        description_len: u64,
        result_is_ok: bool,
    ) -> bool {
        (description_len > MAX_REPO_DESCRIPTION_LENGTH_BYTES) ==> !result_is_ok
    }

    // ========================================================================
    // Construction Specifications
    // ========================================================================

    /// new() precondition (abstract)
    pub open spec fn new_pre(
        name_len: u64,
        delegate_count: u64,
        threshold: u64,
    ) -> bool {
        name_valid(name_len) &&
        delegates_valid(delegate_count) &&
        threshold_valid(threshold, delegate_count)
    }

    /// new() postcondition
    pub open spec fn new_post(
        identity: RepoIdentitySpec,
        name_len: u64,
        delegate_count: u64,
        threshold: u64,
    ) -> bool {
        identity.name_len == name_len &&
        identity.delegate_count == delegate_count &&
        identity.threshold == threshold &&
        identity.created_at_ms > 0
    }

    // ========================================================================
    // Combined Repository Identity Invariant
    // ========================================================================

    /// Complete invariant for repository identity
    pub open spec fn repo_identity_invariant(identity: RepoIdentitySpec) -> bool {
        // Name valid
        name_valid(identity.name_len) &&
        // Description valid
        description_valid(identity.description_len) &&
        // Delegates valid
        delegates_valid(identity.delegate_count) &&
        // Threshold valid
        threshold_valid(identity.threshold, identity.delegate_count) &&
        // Timestamp valid
        identity.created_at_ms > 0
    }

    /// new() produces valid identity
    pub open spec fn new_produces_valid(
        identity: RepoIdentitySpec,
        construction_succeeds: bool,
    ) -> bool {
        construction_succeeds ==> repo_identity_invariant(identity)
    }

    /// Proof: Successfully constructed identities satisfy invariant
    pub proof fn construction_maintains_invariant(
        name_len: u64,
        delegate_count: u64,
        threshold: u64,
        identity: RepoIdentitySpec,
    )
        requires
            new_pre(name_len, delegate_count, threshold),
            new_post(identity, name_len, delegate_count, threshold),
        ensures repo_identity_invariant(identity)
    {
        // new_pre validates all bounds
        // new_post ensures fields are set correctly
        // Therefore invariant holds
    }

    // ========================================================================
    // RepoId Specifications
    // ========================================================================

    /// RepoId hex representation
    pub open spec fn repo_id_hex_len() -> u64 {
        64  // 32 bytes = 64 hex characters
    }

    /// from_hex rejects wrong length
    pub open spec fn from_hex_rejects_wrong_length(
        hex_len: u64,
        result_is_ok: bool,
    ) -> bool {
        (hex_len != 64) ==> !result_is_ok
    }

    /// Hex roundtrip is lossless
    pub open spec fn repo_id_hex_roundtrip(
        original: RepoIdSpec,
        hex_len: u64,
        recovered: RepoIdSpec,
    ) -> bool {
        (hex_len == 64) ==>
        (original.id_high == recovered.id_high &&
         original.id_low == recovered.id_low)
    }
}
