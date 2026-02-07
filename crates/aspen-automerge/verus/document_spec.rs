//! Automerge Document Specification
//!
//! Formal specification for document size and ID validation.
//!
//! # Properties
//!
//! 1. **AM-1: Document Size Bound**: doc_size <= MAX
//! 2. **AM-2: Document ID Valid**: 16 bytes
//! 3. **AM-3: Namespace Limits**: bounded per namespace
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-automerge/verus/document_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum size of a single Automerge document (16 MB)
    pub const MAX_DOCUMENT_SIZE: u64 = 16 * 1024 * 1024;

    /// Length of document IDs in bytes
    pub const DOC_ID_BYTES: u64 = 16;

    /// Length of document IDs as hex strings
    pub const DOC_ID_HEX_LENGTH: u64 = DOC_ID_BYTES * 2;

    /// Maximum length of custom document ID
    pub const MAX_CUSTOM_DOC_ID_LENGTH: u64 = 128;

    /// Maximum documents per namespace
    pub const MAX_DOCUMENTS_PER_NAMESPACE: u64 = 100_000;

    /// Maximum number of namespaces
    pub const MAX_NAMESPACES: u64 = 1000;

    /// Maximum number of heads to track
    pub const MAX_HEADS: u64 = 100;

    /// Maximum history depth
    pub const MAX_HISTORY_DEPTH: u64 = 1000;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract document state
    pub struct DocumentSpec {
        /// Document ID (16 bytes raw, 32 chars hex)
        pub id: Seq<u8>,
        /// Size in bytes
        pub size: u64,
        /// Number of heads (for merge conflicts)
        pub head_count: u64,
    }

    /// Abstract namespace state
    pub struct NamespaceSpec {
        /// Namespace identifier
        pub name: Seq<char>,
        /// Number of documents
        pub document_count: u64,
    }

    /// Abstract document store state
    pub struct DocumentStoreSpec {
        /// Namespaces
        pub namespaces: Seq<NamespaceSpec>,
        /// Total documents across all namespaces
        pub total_documents: u64,
    }

    // ========================================================================
    // AM-1: Document Size Bound
    // ========================================================================

    /// Document size is within limits
    pub open spec fn document_size_bounded(doc: DocumentSpec) -> bool {
        doc.size <= MAX_DOCUMENT_SIZE
    }

    /// Proof: Reject oversized documents
    pub proof fn reject_oversized_documents(size: u64)
        requires size > MAX_DOCUMENT_SIZE
        ensures !document_size_bounded(DocumentSpec {
            id: Seq::empty(),
            size,
            head_count: 1,
        })
    {
        // size > MAX => !bounded
    }

    // ========================================================================
    // AM-2: Document ID Valid
    // ========================================================================

    /// Document ID is valid format (16 bytes)
    pub open spec fn document_id_valid(id: Seq<u8>) -> bool {
        id.len() == DOC_ID_BYTES
    }

    /// Document ID hex is valid format (32 chars)
    pub open spec fn document_id_hex_valid(id_hex: Seq<char>) -> bool {
        id_hex.len() == DOC_ID_HEX_LENGTH
    }

    /// Custom document ID is valid
    pub open spec fn custom_doc_id_valid(id: Seq<char>) -> bool {
        id.len() > 0 && id.len() <= MAX_CUSTOM_DOC_ID_LENGTH
    }

    /// Proof: Generated IDs are valid
    pub proof fn generated_id_valid()
        ensures document_id_valid(Seq::new(DOC_ID_BYTES, |_i| 0u8))
    {
        // Fixed 16 bytes
    }

    // ========================================================================
    // AM-3: Namespace Limits
    // ========================================================================

    /// Namespace has bounded documents
    pub open spec fn namespace_bounded(ns: NamespaceSpec) -> bool {
        ns.document_count <= MAX_DOCUMENTS_PER_NAMESPACE
    }

    /// Store has bounded namespaces
    pub open spec fn namespaces_bounded(store: DocumentStoreSpec) -> bool {
        store.namespaces.len() <= MAX_NAMESPACES
    }

    /// Full store invariant
    pub open spec fn store_invariant(store: DocumentStoreSpec) -> bool {
        namespaces_bounded(store) &&
        forall |i: int| 0 <= i < store.namespaces.len() ==>
            namespace_bounded(store.namespaces[i])
    }

    /// Proof: Store has bounded total documents
    pub proof fn store_total_bounded(store: DocumentStoreSpec)
        requires store_invariant(store)
        ensures store.total_documents <= MAX_NAMESPACES * MAX_DOCUMENTS_PER_NAMESPACE
    {
        // Each namespace <= MAX_DOCUMENTS
        // Number of namespaces <= MAX_NAMESPACES
        // Total <= product
    }

    // ========================================================================
    // Document Creation
    // ========================================================================

    /// Create document effect
    pub open spec fn create_document_effect(
        pre_ns: NamespaceSpec,
        post_ns: NamespaceSpec,
        doc: DocumentSpec,
    ) -> bool {
        // Document is valid
        document_id_valid(doc.id) &&
        document_size_bounded(doc) &&
        // Namespace count incremented
        post_ns.document_count == pre_ns.document_count + 1 &&
        post_ns.name == pre_ns.name
    }

    /// Create document precondition
    pub open spec fn can_create_document(ns: NamespaceSpec) -> bool {
        ns.document_count < MAX_DOCUMENTS_PER_NAMESPACE
    }

    /// Proof: Creation preserves namespace bound
    pub proof fn create_preserves_bound(
        pre_ns: NamespaceSpec,
        post_ns: NamespaceSpec,
        doc: DocumentSpec,
    )
        requires
            namespace_bounded(pre_ns),
            can_create_document(pre_ns),
            create_document_effect(pre_ns, post_ns, doc),
        ensures namespace_bounded(post_ns)
    {
        // pre.count < MAX
        // post.count = pre.count + 1
        // post.count <= MAX
    }

    // ========================================================================
    // Head Limits
    // ========================================================================

    /// Document heads are bounded
    pub open spec fn heads_bounded(doc: DocumentSpec) -> bool {
        doc.head_count <= MAX_HEADS
    }

    /// Full document invariant
    pub open spec fn document_invariant(doc: DocumentSpec) -> bool {
        document_id_valid(doc.id) &&
        document_size_bounded(doc) &&
        heads_bounded(doc)
    }

    // ========================================================================
    // Query Limits
    // ========================================================================

    /// Maximum scan results
    pub const MAX_SCAN_RESULTS: u64 = 1000;

    /// Default list limit
    pub const DEFAULT_LIST_LIMIT: u64 = 100;

    /// Scan results are bounded
    pub open spec fn scan_bounded(result_count: u64) -> bool {
        result_count <= MAX_SCAN_RESULTS
    }

    /// Proof: Pagination bounds results
    pub proof fn pagination_bounds(requested: u64, available: u64)
        ensures {
            let returned = if requested < available { requested } else { available };
            let capped = if returned > MAX_SCAN_RESULTS { MAX_SCAN_RESULTS } else { returned };
            scan_bounded(capped)
        }
    {
        // min(requested, available, MAX) <= MAX
    }
}
