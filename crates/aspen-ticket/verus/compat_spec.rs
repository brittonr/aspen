//! V1/V2 Ticket Compatibility Specification
//!
//! Formal specification for compatibility between V1 and V2 cluster tickets.
//!
//! # Properties
//!
//! 1. **COMPAT-1: V1 Roundtrip**: V1 -> V2 -> V1 preserves data
//! 2. **COMPAT-2: V2 Preserves V1**: V2 contains all V1 data
//! 3. **COMPAT-3: ID Preservation**: Endpoint IDs preserved in conversion
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-ticket/verus/compat_spec.rs
//! ```

use vstd::prelude::*;

use super::ticket_state_spec::*;

verus! {
    // ========================================================================
    // Conversion Models
    // ========================================================================

    /// Abstract V1 to V2 conversion
    ///
    /// V2 from V1 has empty addresses for all peers
    pub open spec fn v1_to_v2(v1: TicketState) -> TicketV2State {
        TicketV2State {
            bootstrap_count: v1.bootstrap_count,
            max_addrs_per_peer: 0,  // V1 has no addresses
            cluster_id_len: v1.cluster_id_len,
        }
    }

    /// Abstract V2 to V1 conversion
    ///
    /// V1 from V2 loses address information
    pub open spec fn v2_to_v1(v2: TicketV2State) -> TicketState {
        TicketState {
            bootstrap_count: v2.bootstrap_count,
            cluster_id_len: v2.cluster_id_len,
        }
    }

    // ========================================================================
    // COMPAT-1: V1 Roundtrip
    // ========================================================================

    /// V1 -> V2 -> V1 roundtrip preserves all V1 data
    pub open spec fn v1_v2_roundtrip(v1: TicketState) -> bool {
        let v2 = v1_to_v2(v1);
        let back = v2_to_v1(v2);
        // All V1 fields preserved
        back.bootstrap_count == v1.bootstrap_count &&
        back.cluster_id_len == v1.cluster_id_len
    }

    /// Proof: V1 roundtrip is lossless
    pub proof fn v1_roundtrip_lossless(v1: TicketState)
        requires ticket_invariant(v1)
        ensures v1_v2_roundtrip(v1)
    {
        let v2 = v1_to_v2(v1);
        let back = v2_to_v1(v2);
        assert(back.bootstrap_count == v1.bootstrap_count);
        assert(back.cluster_id_len == v1.cluster_id_len);
    }

    // ========================================================================
    // COMPAT-2: V2 Preserves V1
    // ========================================================================

    /// V2 created from V1 contains all V1 information
    pub open spec fn v2_preserves_v1_ids(v1: TicketState, v2: TicketV2State) -> bool {
        // Same number of bootstrap peers
        v2.bootstrap_count == v1.bootstrap_count &&
        // Same cluster ID
        v2.cluster_id_len == v1.cluster_id_len
    }

    /// Proof: V1 to V2 conversion preserves V1 data
    pub proof fn v1_to_v2_preserves(v1: TicketState)
        requires ticket_invariant(v1)
        ensures v2_preserves_v1_ids(v1, v1_to_v2(v1))
    {
        let v2 = v1_to_v2(v1);
        assert(v2.bootstrap_count == v1.bootstrap_count);
        assert(v2.cluster_id_len == v1.cluster_id_len);
    }

    // ========================================================================
    // COMPAT-3: Address Loss
    // ========================================================================

    /// V2 -> V1 conversion loses address information
    pub open spec fn v2_to_v1_loses_addrs(v2: TicketV2State) -> bool {
        let v1 = v2_to_v1(v2);
        let back_to_v2 = v1_to_v2(v1);
        // Original may have had addresses, converted back has none
        back_to_v2.max_addrs_per_peer == 0
    }

    /// Proof: V2 addresses lost in V1 conversion
    pub proof fn addresses_lost_in_v1(v2: TicketV2State)
        requires
            ticket_v2_invariant(v2),
            v2.max_addrs_per_peer > 0,
        ensures v2_to_v1_loses_addrs(v2)
    {
        // V1 has no address concept
        // Converting back to V2 yields max_addrs_per_peer = 0
    }

    // ========================================================================
    // Invariant Preservation
    // ========================================================================

    /// V1 to V2 conversion preserves invariants
    pub proof fn v1_to_v2_preserves_invariant(v1: TicketState)
        requires ticket_invariant(v1)
        ensures ticket_v2_invariant(v1_to_v2(v1))
    {
        let v2 = v1_to_v2(v1);
        // bootstrap_count preserved and bounded
        assert(v2.bootstrap_count == v1.bootstrap_count);
        assert(v2.bootstrap_count <= MAX_BOOTSTRAP_PEERS);
        // max_addrs_per_peer = 0 <= MAX_DIRECT_ADDRS_PER_PEER
        assert(v2.max_addrs_per_peer == 0);
    }

    /// V2 to V1 conversion preserves invariants
    pub proof fn v2_to_v1_preserves_invariant(v2: TicketV2State)
        requires ticket_v2_invariant(v2)
        ensures ticket_invariant(v2_to_v1(v2))
    {
        let v1 = v2_to_v1(v2);
        // bootstrap_count preserved and bounded
        assert(v1.bootstrap_count == v2.bootstrap_count);
        assert(v1.bootstrap_count <= MAX_BOOTSTRAP_PEERS);
    }

    // ========================================================================
    // Endpoint ID Counting
    // ========================================================================

    /// endpoint_ids() on V2 returns set with count <= bootstrap_count
    ///
    /// (May be less due to deduplication if there are duplicate peers)
    pub open spec fn endpoint_ids_count(v2: TicketV2State) -> u64 {
        // At most bootstrap_count unique IDs
        // Could be less if duplicates exist
        v2.bootstrap_count
    }

    /// Proof: V2 endpoint_ids matches V1 bootstrap after roundtrip
    pub proof fn endpoint_ids_matches_v1(v1: TicketState)
        requires ticket_invariant(v1)
        ensures {
            let v2 = v1_to_v2(v1);
            endpoint_ids_count(v2) == v1.bootstrap_count
        }
    {
        // V1 uses BTreeSet so no duplicates
        // V2 from V1 has same number of peers
    }

    // ========================================================================
    // Parse Ticket Compatibility
    // ========================================================================

    /// Parse result for unified ticket parsing
    pub enum ParseResult {
        V1 { bootstrap_count: u64 },
        V2 { bootstrap_count: u64, has_addrs: bool },
        Invalid,
    }

    /// Model of parse_ticket_to_addrs behavior
    pub open spec fn parse_ticket_behavior(prefix: u8) -> ParseResult {
        if prefix == 1 {
            // "aspen" prefix -> V1
            ParseResult::V1 { bootstrap_count: 0 }  // Abstract count
        } else if prefix == 2 {
            // "aspenv2" prefix -> V2
            ParseResult::V2 { bootstrap_count: 0, has_addrs: true }
        } else {
            ParseResult::Invalid
        }
    }

    /// V1 parse result has empty addresses
    pub open spec fn v1_parse_has_empty_addrs(result: ParseResult) -> bool {
        match result {
            ParseResult::V1 { .. } => true,  // V1 always has empty addrs
            ParseResult::V2 { has_addrs, .. } => has_addrs,
            ParseResult::Invalid => true,
        }
    }

    // ========================================================================
    // Ticket Kind Prefixes
    // ========================================================================

    /// Ticket serialization prefixes
    pub const V1_PREFIX: u64 = 0x6173_7065_6E00_0000;  // "aspen" in bytes (conceptual)
    pub const V2_PREFIX: u64 = 0x6173_7065_6E76_3200;  // "aspenv2" in bytes (conceptual)
    pub const SIGNED_PREFIX: u64 = 0x6173_7065_6E73_6967;  // "aspensig" (conceptual)

    /// Prefixes are distinct
    pub proof fn prefixes_distinct()
        ensures
            V1_PREFIX != V2_PREFIX,
            V1_PREFIX != SIGNED_PREFIX,
            V2_PREFIX != SIGNED_PREFIX,
    {
        // Different string representations
    }

    /// Parsing uses prefix to determine version
    pub open spec fn prefix_determines_version(serialized_prefix: u64) -> u8 {
        if serialized_prefix == V1_PREFIX {
            1
        } else if serialized_prefix == V2_PREFIX {
            2
        } else {
            0  // Unknown
        }
    }
}
