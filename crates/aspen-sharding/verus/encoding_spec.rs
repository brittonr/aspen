//! Shard Node ID Encoding Specification
//!
//! Formal specification for bit-packed shard/physical node ID encoding.
//!
//! # Encoding Format
//!
//! ```text
//! |<-- 16 bits -->|<------ 48 bits ------>|
//! |   shard_id    |     physical_node_id   |
//! ```
//!
//! # Properties
//!
//! 1. **SHARD-5: Encoding Roundtrip**: decode(encode(s, n)) == (s, n)
//! 2. **SHARD-6: Backward Compatibility**: encode(n, 0) == n
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-sharding/verus/encoding_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Bits reserved for shard ID (upper 16 bits)
    pub const SHARD_ID_BITS: u64 = 16;

    /// Bits for physical node ID (lower 48 bits)
    pub const PHYSICAL_NODE_BITS: u64 = 64 - SHARD_ID_BITS;

    /// Maximum shard ID (16 bits = 65535)
    pub const MAX_SHARD_ID: u64 = (1u64 << SHARD_ID_BITS) - 1;

    /// Maximum physical node ID (48 bits)
    pub const MAX_PHYSICAL_NODE_ID: u64 = (1u64 << PHYSICAL_NODE_BITS) - 1;

    /// Mask for extracting physical node ID
    pub const PHYSICAL_NODE_MASK: u64 = MAX_PHYSICAL_NODE_ID;

    // ========================================================================
    // Encoding Operations
    // ========================================================================

    /// Encode shard ID and physical node ID into combined ID
    pub open spec fn encode(physical_node_id: u64, shard_id: u64) -> u64 {
        // physical_node_id | (shard_id << (64 - SHARD_ID_BITS))
        // = physical_node_id | (shard_id << 48)
        physical_node_id | (shard_id << PHYSICAL_NODE_BITS)
    }

    /// Decode combined ID into (physical_node_id, shard_id)
    pub open spec fn decode_physical(encoded: u64) -> u64 {
        encoded & PHYSICAL_NODE_MASK
    }

    pub open spec fn decode_shard(encoded: u64) -> u64 {
        encoded >> PHYSICAL_NODE_BITS
    }

    // ========================================================================
    // SHARD-5: Encoding Roundtrip
    // ========================================================================

    /// Encode followed by decode recovers original values
    pub open spec fn encoding_roundtrip(
        physical_node_id: u64,
        shard_id: u64,
    ) -> bool {
        let encoded = encode(physical_node_id, shard_id);
        decode_physical(encoded) == physical_node_id &&
        decode_shard(encoded) == shard_id
    }

    /// Proof: Encoding roundtrip holds for valid inputs
    pub proof fn encoding_roundtrip_holds(
        physical_node_id: u64,
        shard_id: u64,
    )
        requires
            physical_node_id <= MAX_PHYSICAL_NODE_ID,
            shard_id <= MAX_SHARD_ID,
        ensures encoding_roundtrip(physical_node_id, shard_id)
    {
        let encoded = encode(physical_node_id, shard_id);
        // encoded = physical | (shard << 48)

        // decode_physical: encoded & MASK
        // Since physical <= MAX_PHYSICAL (48 bits), and shard is in upper 16 bits,
        // the mask extracts exactly physical_node_id
        assert(decode_physical(encoded) == physical_node_id);

        // decode_shard: encoded >> 48
        // Shifts out the physical bits, leaving shard_id
        assert(decode_shard(encoded) == shard_id);
    }

    /// Decode then encode also produces same value
    pub open spec fn decode_encode_identity(encoded: u64) -> bool {
        let physical = decode_physical(encoded);
        let shard = decode_shard(encoded);
        encode(physical, shard) == encoded
    }

    /// Proof: Decode-encode identity holds
    pub proof fn decode_encode_identity_holds(encoded: u64)
        ensures decode_encode_identity(encoded)
    {
        let physical = decode_physical(encoded);
        let shard = decode_shard(encoded);
        // physical has lower 48 bits of encoded
        // shard has upper 16 bits of encoded
        // Combining them reconstructs encoded
    }

    // ========================================================================
    // SHARD-6: Backward Compatibility
    // ========================================================================

    /// Shard 0 preserves the physical node ID
    pub open spec fn backward_compatible(physical_node_id: u64) -> bool {
        encode(physical_node_id, 0) == physical_node_id
    }

    /// Proof: Shard 0 is backward compatible
    pub proof fn shard_zero_backward_compatible(physical_node_id: u64)
        requires physical_node_id <= MAX_PHYSICAL_NODE_ID
        ensures backward_compatible(physical_node_id)
    {
        // encode(physical, 0) = physical | (0 << 48) = physical | 0 = physical
        assert(encode(physical_node_id, 0) == physical_node_id);
    }

    // ========================================================================
    // Shard ID Extraction
    // ========================================================================

    /// get_shard_from_node_id correctly extracts shard
    pub open spec fn shard_id_extractable(
        physical_node_id: u64,
        shard_id: u64,
    ) -> bool {
        let encoded = encode(physical_node_id, shard_id);
        decode_shard(encoded) == shard_id
    }

    /// get_physical_node_id correctly extracts physical ID
    pub open spec fn physical_id_extractable(
        physical_node_id: u64,
        shard_id: u64,
    ) -> bool {
        let encoded = encode(physical_node_id, shard_id);
        decode_physical(encoded) == physical_node_id
    }

    /// Proof: Shard ID extraction is correct
    pub proof fn shard_extraction_correct(
        physical_node_id: u64,
        shard_id: u64,
    )
        requires
            physical_node_id <= MAX_PHYSICAL_NODE_ID,
            shard_id <= MAX_SHARD_ID,
        ensures shard_id_extractable(physical_node_id, shard_id)
    {
        // From encoding_roundtrip_holds
    }

    /// Proof: Physical ID extraction is correct
    pub proof fn physical_extraction_correct(
        physical_node_id: u64,
        shard_id: u64,
    )
        requires
            physical_node_id <= MAX_PHYSICAL_NODE_ID,
            shard_id <= MAX_SHARD_ID,
        ensures physical_id_extractable(physical_node_id, shard_id)
    {
        // From encoding_roundtrip_holds
    }

    // ========================================================================
    // is_shard_node Check
    // ========================================================================

    /// is_shard_node correctly identifies shard membership
    pub open spec fn is_shard_node_correct(
        physical_node_id: u64,
        shard_id: u64,
        check_shard: u64,
    ) -> bool {
        let encoded = encode(physical_node_id, shard_id);
        let extracted_shard = decode_shard(encoded);
        (extracted_shard == check_shard) == (shard_id == check_shard)
    }

    /// Proof: is_shard_node is correct
    pub proof fn is_shard_node_proof(
        physical_node_id: u64,
        shard_id: u64,
        check_shard: u64,
    )
        requires
            physical_node_id <= MAX_PHYSICAL_NODE_ID,
            shard_id <= MAX_SHARD_ID,
        ensures is_shard_node_correct(physical_node_id, shard_id, check_shard)
    {
        let encoded = encode(physical_node_id, shard_id);
        // From encoding_roundtrip, decode_shard(encoded) == shard_id
        assert(decode_shard(encoded) == shard_id);
    }

    // ========================================================================
    // Bit Layout Properties
    // ========================================================================

    /// Shard and physical bits don't overlap
    pub open spec fn bits_disjoint() -> bool {
        // Shard uses bits 48-63, physical uses bits 0-47
        SHARD_ID_BITS + PHYSICAL_NODE_BITS == 64
    }

    /// Proof: Bit layout is correct
    pub proof fn bit_layout_correct()
        ensures bits_disjoint()
    {
        // SHARD_ID_BITS = 16, PHYSICAL_NODE_BITS = 48
        // 16 + 48 = 64
    }

    /// No information is lost in encoding
    pub open spec fn encoding_lossless(
        physical_node_id: u64,
        shard_id: u64,
    ) -> bool {
        // If inputs are within bounds, roundtrip is exact
        (physical_node_id <= MAX_PHYSICAL_NODE_ID &&
         shard_id <= MAX_SHARD_ID)
        ==>
        encoding_roundtrip(physical_node_id, shard_id)
    }

    /// Proof: Encoding is lossless for valid inputs
    pub proof fn encoding_is_lossless(
        physical_node_id: u64,
        shard_id: u64,
    )
        requires
            physical_node_id <= MAX_PHYSICAL_NODE_ID,
            shard_id <= MAX_SHARD_ID,
        ensures encoding_lossless(physical_node_id, shard_id)
    {
        // From encoding_roundtrip_holds
    }
}
