//! SNIX Path Info Specification
//!
//! Formal specification for store path references and signatures.
//!
//! # Properties
//!
//! 1. **SNIX-5: Reference Bound**: refs <= MAX
//! 2. **SNIX-6: Signature Bound**: sigs <= MAX
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-snix/verus/pathinfo_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum number of references per store path
    pub const MAX_PATH_REFERENCES: u64 = 10_000;

    /// Maximum length of deriver path
    pub const MAX_DERIVER_LENGTH: u64 = 1024;

    /// Maximum number of signatures
    pub const MAX_SIGNATURES: u64 = 100;

    /// Store path digest length (20 bytes = 160 bits)
    pub const STORE_PATH_DIGEST_LENGTH: u64 = 20;

    /// BLAKE3 digest length (32 bytes)
    pub const B3_DIGEST_LENGTH: u64 = 32;

    /// Path info operation timeout (ms)
    pub const PATHINFO_TIMEOUT_MS: u64 = 10_000;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract store path
    pub struct StorePathSpec {
        /// Digest portion (20 bytes, base32 encoded in string form)
        pub digest: Seq<u8>,
        /// Name portion
        pub name: Seq<char>,
    }

    /// Abstract CA (content-addressed) node
    pub enum CaNodeSpec {
        /// Text file with content
        Text { digest: Seq<u8>, size: u64 },
        /// Flat file with content
        Flat { digest: Seq<u8>, size: u64 },
        /// NAR archive
        Nar { nar_digest: Seq<u8>, nar_size: u64 },
    }

    /// Signature on path info
    pub struct SignatureSpec {
        /// Key name
        pub key_name: Seq<char>,
        /// Signature bytes
        pub signature: Seq<u8>,
    }

    /// Abstract path info
    pub struct PathInfoSpec {
        /// Store path
        pub store_path: StorePathSpec,
        /// Content-addressed node
        pub node: CaNodeSpec,
        /// Store path references (dependencies)
        pub references: Seq<StorePathSpec>,
        /// Narinfo string (for compatibility)
        pub narinfo: Option<Seq<char>>,
        /// Deriver store path
        pub deriver: Option<StorePathSpec>,
        /// Signatures
        pub signatures: Seq<SignatureSpec>,
    }

    // ========================================================================
    // SNIX-5: Reference Bound
    // ========================================================================

    /// References are within limits
    pub open spec fn reference_bounded(info: PathInfoSpec) -> bool {
        info.references.len() <= MAX_PATH_REFERENCES
    }

    /// Proof: Reject too many references
    pub proof fn reject_too_many_refs(refs: Seq<StorePathSpec>)
        requires refs.len() > MAX_PATH_REFERENCES
        ensures {
            let info = PathInfoSpec {
                store_path: StorePathSpec { digest: Seq::empty(), name: Seq::empty() },
                node: CaNodeSpec::Flat { digest: Seq::empty(), size: 0 },
                references: refs,
                narinfo: None,
                deriver: None,
                signatures: Seq::empty(),
            };
            !reference_bounded(info)
        }
    {
        // refs.len() > MAX => !bounded
    }

    // ========================================================================
    // SNIX-6: Signature Bound
    // ========================================================================

    /// Signatures are within limits
    pub open spec fn signature_bounded(info: PathInfoSpec) -> bool {
        info.signatures.len() <= MAX_SIGNATURES
    }

    /// Proof: Reject too many signatures
    pub proof fn reject_too_many_sigs(sigs: Seq<SignatureSpec>)
        requires sigs.len() > MAX_SIGNATURES
        ensures {
            let info = PathInfoSpec {
                store_path: StorePathSpec { digest: Seq::empty(), name: Seq::empty() },
                node: CaNodeSpec::Flat { digest: Seq::empty(), size: 0 },
                references: Seq::empty(),
                narinfo: None,
                deriver: None,
                signatures: sigs,
            };
            !signature_bounded(info)
        }
    {
        // sigs.len() > MAX => !bounded
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Store path is valid
    pub open spec fn store_path_valid(path: StorePathSpec) -> bool {
        path.digest.len() == STORE_PATH_DIGEST_LENGTH &&
        path.name.len() > 0
    }

    /// Deriver is valid if present
    pub open spec fn deriver_valid(info: PathInfoSpec) -> bool {
        match info.deriver {
            Some(d) => store_path_valid(d),
            None => true,
        }
    }

    /// Full path info invariant
    pub open spec fn pathinfo_invariant(info: PathInfoSpec) -> bool {
        reference_bounded(info) &&
        signature_bounded(info) &&
        store_path_valid(info.store_path) &&
        deriver_valid(info)
    }

    // ========================================================================
    // Path Info Operations
    // ========================================================================

    /// Put path info effect
    pub open spec fn put_pathinfo_effect(
        info: PathInfoSpec,
    ) -> bool {
        pathinfo_invariant(info)
    }

    /// Get path info effect
    pub open spec fn get_pathinfo_effect(
        digest: Seq<u8>,
        result: Option<PathInfoSpec>,
    ) -> bool {
        result.is_some() ==>
            pathinfo_invariant(result.unwrap())
    }

    // ========================================================================
    // Reference Operations
    // ========================================================================

    /// Add reference effect
    pub open spec fn add_reference_effect(
        pre: PathInfoSpec,
        post: PathInfoSpec,
        new_ref: StorePathSpec,
    ) -> bool {
        store_path_valid(new_ref) &&
        post.references.len() == pre.references.len() + 1 &&
        post.references[post.references.len() - 1] == new_ref &&
        // Other fields unchanged
        post.store_path == pre.store_path &&
        post.node == pre.node &&
        post.signatures == pre.signatures
    }

    /// Can add reference
    pub open spec fn can_add_reference(info: PathInfoSpec) -> bool {
        info.references.len() < MAX_PATH_REFERENCES
    }

    /// Proof: Adding reference preserves invariant
    pub proof fn add_ref_preserves_invariant(
        pre: PathInfoSpec,
        post: PathInfoSpec,
        new_ref: StorePathSpec,
    )
        requires
            pathinfo_invariant(pre),
            can_add_reference(pre),
            add_reference_effect(pre, post, new_ref),
        ensures reference_bounded(post)
    {
        // pre.refs < MAX
        // post.refs = pre.refs + 1 <= MAX
    }

    // ========================================================================
    // Signature Operations
    // ========================================================================

    /// Add signature effect
    pub open spec fn add_signature_effect(
        pre: PathInfoSpec,
        post: PathInfoSpec,
        new_sig: SignatureSpec,
    ) -> bool {
        post.signatures.len() == pre.signatures.len() + 1 &&
        post.signatures[post.signatures.len() - 1] == new_sig &&
        // Other fields unchanged
        post.store_path == pre.store_path &&
        post.node == pre.node &&
        post.references == pre.references
    }

    /// Can add signature
    pub open spec fn can_add_signature(info: PathInfoSpec) -> bool {
        info.signatures.len() < MAX_SIGNATURES
    }

    /// Proof: Adding signature preserves invariant
    pub proof fn add_sig_preserves_invariant(
        pre: PathInfoSpec,
        post: PathInfoSpec,
        new_sig: SignatureSpec,
    )
        requires
            pathinfo_invariant(pre),
            can_add_signature(pre),
            add_signature_effect(pre, post, new_sig),
        ensures signature_bounded(post)
    {
        // pre.sigs < MAX
        // post.sigs = pre.sigs + 1 <= MAX
    }

    // ========================================================================
    // CA Node Types
    // ========================================================================

    /// Node has valid digest
    pub open spec fn node_valid(node: CaNodeSpec) -> bool {
        match node {
            CaNodeSpec::Text { digest, size } =>
                digest.len() == B3_DIGEST_LENGTH,
            CaNodeSpec::Flat { digest, size } =>
                digest.len() == B3_DIGEST_LENGTH,
            CaNodeSpec::Nar { nar_digest, nar_size } =>
                nar_digest.len() == B3_DIGEST_LENGTH,
        }
    }

    /// Get node size
    pub open spec fn node_size(node: CaNodeSpec) -> u64 {
        match node {
            CaNodeSpec::Text { digest, size } => size,
            CaNodeSpec::Flat { digest, size } => size,
            CaNodeSpec::Nar { nar_digest, nar_size } => nar_size,
        }
    }

    // ========================================================================
    // Closure Calculation
    // ========================================================================

    /// Closure includes all transitive references
    /// (This is a simplified spec; real closure would be recursive)
    pub open spec fn closure_includes(
        info: PathInfoSpec,
        closure: Seq<StorePathSpec>,
    ) -> bool {
        // Info's path is in closure
        exists |i: int| 0 <= i < closure.len() && closure[i] == info.store_path
    }

    /// Closure is bounded by sum of all reference counts
    pub open spec fn closure_bounded(closure: Seq<StorePathSpec>) -> bool {
        // Simplified: closure size bounded
        closure.len() <= MAX_PATH_REFERENCES * 10  // Arbitrary factor
    }
}
