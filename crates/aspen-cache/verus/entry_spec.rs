//! Cache Entry Specification
//!
//! Formal specification for cache entry bounds and validation.
//!
//! # Properties
//!
//! 1. **CACHE-1: Reference Bound**: references bounded
//! 2. **CACHE-2: Deriver Bound**: deriver path bounded
//! 3. **CACHE-3: Store Path Bound**: store path bounded
//! 4. **CACHE-4: Hash Valid**: hash format valid
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-cache/verus/entry_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum number of references per cache entry
    pub const MAX_REFERENCES: u64 = 1000;

    /// Maximum length of deriver path
    pub const MAX_DERIVER_LENGTH: u64 = 1024;

    /// Maximum length of store path
    pub const MAX_STORE_PATH_LENGTH: u64 = 512;

    /// Store hash length (32 bytes)
    pub const STORE_HASH_LENGTH: u64 = 32;

    /// BLAKE3 hash length (32 bytes)
    pub const BLOB_HASH_LENGTH: u64 = 32;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract cache entry
    pub struct CacheEntrySpec {
        /// Full Nix store path
        pub store_path: Seq<char>,
        /// Hash portion of store path
        pub store_hash: Seq<char>,
        /// BLAKE3 hash of blob
        pub blob_hash: Seq<char>,
        /// Size of NAR archive
        pub nar_size: u64,
        /// SHA256 hash of NAR
        pub nar_hash: Seq<char>,
        /// Store path references (dependencies)
        pub references: Seq<Seq<char>>,
        /// Deriver store path
        pub deriver: Option<Seq<char>>,
        /// Creation timestamp
        pub created_at: u64,
        /// Creating node ID
        pub created_by_node: u64,
    }

    // ========================================================================
    // CACHE-1: Reference Bound
    // ========================================================================

    /// References are within limits
    pub open spec fn reference_bounded(entry: CacheEntrySpec) -> bool {
        entry.references.len() <= MAX_REFERENCES
    }

    /// Proof: Reject too many references
    pub proof fn reject_too_many_references(refs: Seq<Seq<char>>)
        requires refs.len() > MAX_REFERENCES
        ensures {
            let entry = CacheEntrySpec {
                store_path: Seq::empty(),
                store_hash: Seq::empty(),
                blob_hash: Seq::empty(),
                nar_size: 0,
                nar_hash: Seq::empty(),
                references: refs,
                deriver: None,
                created_at: 0,
                created_by_node: 0,
            };
            !reference_bounded(entry)
        }
    {
        // refs.len() > MAX => !bounded
    }

    // ========================================================================
    // CACHE-2: Deriver Bound
    // ========================================================================

    /// Deriver path is within limits
    pub open spec fn deriver_bounded(entry: CacheEntrySpec) -> bool {
        match entry.deriver {
            Some(d) => d.len() <= MAX_DERIVER_LENGTH,
            None => true,
        }
    }

    /// Proof: No deriver is always bounded
    pub proof fn no_deriver_bounded()
        ensures {
            let entry = CacheEntrySpec {
                store_path: Seq::empty(),
                store_hash: Seq::empty(),
                blob_hash: Seq::empty(),
                nar_size: 0,
                nar_hash: Seq::empty(),
                references: Seq::empty(),
                deriver: None,
                created_at: 0,
                created_by_node: 0,
            };
            deriver_bounded(entry)
        }
    {
        // None => true
    }

    // ========================================================================
    // CACHE-3: Store Path Bound
    // ========================================================================

    /// Store path is within limits
    pub open spec fn store_path_bounded(entry: CacheEntrySpec) -> bool {
        entry.store_path.len() <= MAX_STORE_PATH_LENGTH
    }

    // ========================================================================
    // CACHE-4: Hash Valid
    // ========================================================================

    /// Store hash has valid format
    pub open spec fn hash_valid(entry: CacheEntrySpec) -> bool {
        entry.store_hash.len() == STORE_HASH_LENGTH ||
        entry.store_hash.len() == 52  // base32 encoding
    }

    /// Blob hash has valid format
    pub open spec fn blob_hash_valid(entry: CacheEntrySpec) -> bool {
        entry.blob_hash.len() == BLOB_HASH_LENGTH ||
        entry.blob_hash.len() == BLOB_HASH_LENGTH * 2  // hex encoding
    }

    // ========================================================================
    // Combined Entry Invariant
    // ========================================================================

    /// Full entry invariant
    pub open spec fn entry_invariant(entry: CacheEntrySpec) -> bool {
        reference_bounded(entry) &&
        deriver_bounded(entry) &&
        store_path_bounded(entry) &&
        hash_valid(entry) &&
        blob_hash_valid(entry)
    }

    // ========================================================================
    // Entry Creation
    // ========================================================================

    /// Entry creation with validation
    pub open spec fn create_entry_validated(
        store_path: Seq<char>,
        store_hash: Seq<char>,
        blob_hash: Seq<char>,
        nar_size: u64,
        nar_hash: Seq<char>,
        created_at: u64,
        created_by_node: u64,
    ) -> Option<CacheEntrySpec> {
        let entry = CacheEntrySpec {
            store_path,
            store_hash,
            blob_hash,
            nar_size,
            nar_hash,
            references: Seq::empty(),
            deriver: None,
            created_at,
            created_by_node,
        };
        if entry_invariant(entry) {
            Some(entry)
        } else {
            None
        }
    }

    /// Proof: Successful creation implies invariant
    pub proof fn creation_ensures_invariant(
        store_path: Seq<char>,
        store_hash: Seq<char>,
        blob_hash: Seq<char>,
        nar_size: u64,
        nar_hash: Seq<char>,
        created_at: u64,
        created_by_node: u64,
    )
        ensures {
            let result = create_entry_validated(
                store_path, store_hash, blob_hash, nar_size, nar_hash,
                created_at, created_by_node
            );
            result.is_some() ==> entry_invariant(result.unwrap())
        }
    {
        // Direct from create_entry_validated definition
    }

    // ========================================================================
    // Add References
    // ========================================================================

    /// Add references effect
    pub open spec fn add_references_effect(
        pre: CacheEntrySpec,
        post: CacheEntrySpec,
        new_refs: Seq<Seq<char>>,
    ) -> bool {
        post.store_path == pre.store_path &&
        post.store_hash == pre.store_hash &&
        post.blob_hash == pre.blob_hash &&
        post.nar_size == pre.nar_size &&
        post.nar_hash == pre.nar_hash &&
        post.references == new_refs &&
        post.deriver == pre.deriver &&
        post.created_at == pre.created_at &&
        post.created_by_node == pre.created_by_node
    }

    /// Add references precondition
    pub open spec fn can_add_references(new_refs: Seq<Seq<char>>) -> bool {
        new_refs.len() <= MAX_REFERENCES
    }

    /// Proof: Adding valid references preserves invariant
    pub proof fn add_refs_preserves_invariant(
        pre: CacheEntrySpec,
        post: CacheEntrySpec,
        new_refs: Seq<Seq<char>>,
    )
        requires
            entry_invariant(pre),
            can_add_references(new_refs),
            add_references_effect(pre, post, new_refs),
        ensures entry_invariant(post)
    {
        // Only references changed, and new_refs is bounded
    }

    // ========================================================================
    // KV Key Generation
    // ========================================================================

    /// KV key prefix
    pub const CACHE_KEY_PREFIX_LEN: u64 = 15;  // "_cache:narinfo:"

    /// Generate KV key for entry
    pub open spec fn kv_key_for_entry(entry: CacheEntrySpec) -> Seq<char> {
        // Prefix + store_hash
        // Actual implementation would concatenate
        entry.store_hash  // Simplified
    }

    /// KV key is valid
    pub open spec fn kv_key_valid(key: Seq<char>) -> bool {
        key.len() > 0 && key.len() <= CACHE_KEY_PREFIX_LEN + STORE_HASH_LENGTH
    }
}
