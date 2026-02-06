//! Client RPC Message Verification Specifications
//!
//! Formal specifications for message size bounds, CAS semantics,
//! and authentication token handling.
//!
//! # Key Invariants
//!
//! 1. **MSG-1: Size Bounds**: 4MB limit
//! 2. **MSG-2: Chunk Bounds**: 4MB chunk limit
//! 3. **MSG-3: Cluster Bounds**: 16 node limit
//! 4. **CAS-1: CompareAndSwap Atomicity**
//! 5. **CAS-2: CompareAndDelete Atomicity**
//! 6. **TOKEN-1: Wire Format**
//! 7. **TOKEN-2: Token Presence**
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-client-api/verus/message_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum client message size (4 MB)
    pub const MAX_CLIENT_MESSAGE_SIZE: u64 = 4 * 1024 * 1024;

    /// Maximum cluster nodes in response
    pub const MAX_CLUSTER_NODES: u64 = 16;

    /// Default git chunk size (1 MB)
    pub const DEFAULT_GIT_CHUNK_SIZE: u64 = 1024 * 1024;

    /// Maximum git chunk size (4 MB)
    pub const MAX_GIT_CHUNK_SIZE: u64 = 4 * 1024 * 1024;

    /// Maximum client connections
    pub const MAX_CLIENT_CONNECTIONS: u64 = 50;

    /// Maximum streams per connection
    pub const MAX_CLIENT_STREAMS_PER_CONNECTION: u64 = 10;

    // ========================================================================
    // State Models
    // ========================================================================

    /// Abstract capability token
    pub struct CapabilityTokenSpec {
        /// Token is present
        pub present: bool,
        /// Token bytes length
        pub len: u64,
    }

    /// Abstract authenticated request
    pub struct AuthenticatedRequestSpec {
        /// Request type (abstract)
        pub request_type: u64,
        /// Request payload size
        pub request_size: u64,
        /// Capability token
        pub token: Option<CapabilityTokenSpec>,
    }

    /// CAS operation specification
    pub struct CasOperationSpec {
        /// Key being operated on
        pub key_len: u64,
        /// Expected value (None = must not exist)
        pub expected: Option<u64>,  // length if present
        /// New value to set
        pub new_value_len: u64,
    }

    /// Cluster state response
    pub struct ClusterStateSpec {
        /// Number of nodes in response
        pub node_count: u64,
    }

    // ========================================================================
    // Invariant MSG-1: Message Size Bounds
    // ========================================================================

    /// MSG-1: Message size bounded
    pub open spec fn message_size_bounded(message_len: u64) -> bool {
        message_len <= MAX_CLIENT_MESSAGE_SIZE
    }

    /// Size check before deserialization
    pub open spec fn deserialize_pre(bytes_len: u64) -> bool {
        message_size_bounded(bytes_len)
    }

    /// Deserialization rejects oversized
    pub open spec fn deserialize_rejects_oversized(
        bytes_len: u64,
        result_is_ok: bool,
    ) -> bool {
        !message_size_bounded(bytes_len) ==> !result_is_ok
    }

    /// Proof: Size check prevents exhaustion
    pub proof fn size_check_prevents_exhaustion(bytes_len: u64)
        requires !deserialize_pre(bytes_len)
        ensures bytes_len > MAX_CLIENT_MESSAGE_SIZE
    {
        // If pre fails, bytes exceed limit
    }

    // ========================================================================
    // Invariant MSG-2: Chunk Bounds
    // ========================================================================

    /// MSG-2: Git chunk size bounded
    pub open spec fn chunk_size_bounded(chunk_len: u64) -> bool {
        chunk_len <= MAX_GIT_CHUNK_SIZE
    }

    /// Default chunk size is within bounds
    pub open spec fn default_chunk_valid() -> bool {
        DEFAULT_GIT_CHUNK_SIZE <= MAX_GIT_CHUNK_SIZE
    }

    /// Proof: Default is valid
    pub proof fn default_chunk_is_valid()
        ensures default_chunk_valid()
    {
        // 1MB <= 4MB
    }

    // ========================================================================
    // Invariant MSG-3: Cluster State Bounds
    // ========================================================================

    /// MSG-3: Cluster state node count bounded
    pub open spec fn cluster_state_bounded(node_count: u64) -> bool {
        node_count <= MAX_CLUSTER_NODES
    }

    /// Response construction respects bounds
    pub open spec fn cluster_response_bounded(state: ClusterStateSpec) -> bool {
        cluster_state_bounded(state.node_count)
    }

    /// Proof: Cluster responses are bounded
    pub proof fn cluster_response_always_bounded(node_count: u64)
        requires node_count <= 16
        ensures cluster_state_bounded(node_count)
    {
        // By definition
    }

    // ========================================================================
    // Invariant CAS-1: CompareAndSwap Atomicity
    // ========================================================================

    /// CAS-1: Compare-and-swap precondition
    pub open spec fn cas_pre(
        expected: Option<u64>,
        current_exists: bool,
        current_value_matches: bool,
    ) -> bool {
        match expected {
            None => {
                // Expected None: key must not exist
                !current_exists
            }
            Some(_) => {
                // Expected Some: key must exist with matching value
                current_exists && current_value_matches
            }
        }
    }

    /// CAS succeeds only when precondition holds
    pub open spec fn cas_succeeds_iff_pre(
        expected: Option<u64>,
        current_exists: bool,
        current_value_matches: bool,
        result_is_ok: bool,
    ) -> bool {
        result_is_ok == cas_pre(expected, current_exists, current_value_matches)
    }

    /// CAS atomicity: no interleaving
    pub open spec fn cas_atomic(
        pre_value: Option<u64>,
        post_value: u64,
        expected: Option<u64>,
        new_value: u64,
        success: bool,
    ) -> bool {
        success ==> (
            // Pre matches expected
            pre_value == expected &&
            // Post is new value
            post_value == new_value
        )
    }

    // ========================================================================
    // Invariant CAS-2: CompareAndDelete Atomicity
    // ========================================================================

    /// CAS-2: Compare-and-delete precondition
    pub open spec fn cad_pre(
        expected_len: u64,
        current_exists: bool,
        current_value_matches: bool,
    ) -> bool {
        // Key must exist with exactly the expected value
        current_exists && current_value_matches
    }

    /// Compare-and-delete succeeds only when value matches
    pub open spec fn cad_succeeds_iff_pre(
        expected_len: u64,
        current_exists: bool,
        current_value_matches: bool,
        result_is_ok: bool,
    ) -> bool {
        result_is_ok == cad_pre(expected_len, current_exists, current_value_matches)
    }

    /// Proof: Stale expected value causes failure
    pub proof fn stale_value_fails_cad(
        expected_len: u64,
        current_value_matches: bool,
    )
        requires !current_value_matches
        ensures !cad_pre(expected_len, true, current_value_matches)
    {
        // Value mismatch causes failure
    }

    // ========================================================================
    // Invariant TOKEN-1: Wire Format
    // ========================================================================

    /// Wire format tag values
    pub const WIRE_TAG_LEGACY: u8 = 0;
    pub const WIRE_TAG_AUTHENTICATED: u8 = 1;

    /// TOKEN-1: Wire format tag is correct
    pub open spec fn wire_tag_correct(
        has_token: bool,
        wire_tag: u8,
    ) -> bool {
        if has_token {
            wire_tag == WIRE_TAG_AUTHENTICATED
        } else {
            wire_tag == WIRE_TAG_LEGACY
        }
    }

    /// Parsing uses correct tag
    pub open spec fn parse_wire_format(
        wire_tag: u8,
        parsed_has_token: bool,
    ) -> bool {
        match wire_tag {
            0 => !parsed_has_token,  // Legacy: no token
            1 => parsed_has_token,   // Authenticated: has token
            _ => false,              // Unknown tag
        }
    }

    // ========================================================================
    // Invariant TOKEN-2: Token Presence
    // ========================================================================

    /// TOKEN-2: new() sets token to Some
    pub open spec fn new_has_token(request: AuthenticatedRequestSpec) -> bool {
        request.token.is_some()
    }

    /// unauthenticated() sets token to None
    pub open spec fn unauthenticated_no_token(request: AuthenticatedRequestSpec) -> bool {
        request.token.is_none()
    }

    /// new() postcondition
    pub open spec fn new_post(request: AuthenticatedRequestSpec) -> bool {
        new_has_token(request)
    }

    /// unauthenticated() postcondition
    pub open spec fn unauthenticated_post(request: AuthenticatedRequestSpec) -> bool {
        unauthenticated_no_token(request)
    }

    // ========================================================================
    // Connection Limits
    // ========================================================================

    /// Connection count bounded
    pub open spec fn connections_bounded(count: u64) -> bool {
        count <= MAX_CLIENT_CONNECTIONS
    }

    /// Streams per connection bounded
    pub open spec fn streams_bounded(count: u64) -> bool {
        count <= MAX_CLIENT_STREAMS_PER_CONNECTION
    }

    /// Total streams bounded
    pub open spec fn total_streams_bounded(
        connections: u64,
        streams_per_connection: u64,
    ) -> bool {
        connections <= MAX_CLIENT_CONNECTIONS &&
        streams_per_connection <= MAX_CLIENT_STREAMS_PER_CONNECTION
    }

    /// Maximum possible streams
    pub open spec fn max_total_streams() -> u64 {
        MAX_CLIENT_CONNECTIONS * MAX_CLIENT_STREAMS_PER_CONNECTION  // 500
    }

    // ========================================================================
    // Combined Message Invariant
    // ========================================================================

    /// Complete invariant for authenticated request
    pub open spec fn authenticated_request_invariant(
        request: AuthenticatedRequestSpec,
    ) -> bool {
        // Request size bounded
        message_size_bounded(request.request_size) &&
        // Token, if present, has valid length
        match request.token {
            Some(token) => token.len > 0,
            None => true,
        }
    }

    /// Proof: Well-formed requests satisfy invariant
    pub proof fn well_formed_satisfies_invariant(
        request: AuthenticatedRequestSpec,
    )
        requires
            request.request_size <= MAX_CLIENT_MESSAGE_SIZE,
            request.token.is_none() || request.token.unwrap().len > 0,
        ensures authenticated_request_invariant(request)
    {
        // By construction
    }

    // ========================================================================
    // Request Type Enumeration (Abstract)
    // ========================================================================

    /// Request types (abstract numeric IDs for verification)
    pub enum RequestTypeSpec {
        GetHealth,
        GetRaftMetrics,
        ReadKey,
        WriteKey,
        CompareAndSwap,
        CompareAndDelete,
        ScanKeys,
        // ... other types abstracted
    }

    /// Read requests don't modify state
    pub open spec fn is_read_only(request_type: RequestTypeSpec) -> bool {
        match request_type {
            RequestTypeSpec::GetHealth => true,
            RequestTypeSpec::GetRaftMetrics => true,
            RequestTypeSpec::ReadKey => true,
            RequestTypeSpec::ScanKeys => true,
            _ => false,
        }
    }

    /// Write requests modify state
    pub open spec fn is_write(request_type: RequestTypeSpec) -> bool {
        match request_type {
            RequestTypeSpec::WriteKey => true,
            RequestTypeSpec::CompareAndSwap => true,
            RequestTypeSpec::CompareAndDelete => true,
            _ => false,
        }
    }

    /// Reads and writes are mutually exclusive
    pub proof fn read_write_exclusive(request_type: RequestTypeSpec)
        ensures !(is_read_only(request_type) && is_write(request_type))
    {
        // By enumeration
    }
}
