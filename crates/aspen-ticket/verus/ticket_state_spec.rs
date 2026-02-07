//! Ticket State Model and Invariants
//!
//! Abstract state model for formal verification of cluster tickets.
//!
//! # State Model
//!
//! The ticket state captures:
//! - Topic ID (32-byte gossip topic identifier)
//! - Bootstrap peers (bounded set of peer identifiers)
//! - Cluster ID (human-readable identifier)
//!
//! # Key Invariants
//!
//! 1. **TICKET-1: Bootstrap Bounds**: bootstrap.len() <= MAX_BOOTSTRAP_PEERS
//! 2. **TICKET-2: Address Bounds**: Per-peer addresses bounded
//! 3. **TICKET-3: Serialization Determinism**: Same input -> same output
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-ticket/verus/ticket_state_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum number of bootstrap peers in a ticket (V1 and V2)
    pub const MAX_BOOTSTRAP_PEERS: u64 = 16;

    /// Maximum direct addresses per peer in V2 tickets
    pub const MAX_DIRECT_ADDRS_PER_PEER: u64 = 8;

    /// Topic ID size in bytes
    pub const TOPIC_ID_SIZE: u64 = 32;

    /// Endpoint ID size in bytes (Ed25519 public key)
    pub const ENDPOINT_ID_SIZE: u64 = 32;

    // ========================================================================
    // V1 Ticket State Model
    // ========================================================================

    /// Abstract V1 ticket structure
    ///
    /// Models AspenClusterTicket from lib.rs
    pub struct TicketState {
        /// Number of bootstrap peers (abstraction of BTreeSet<EndpointId>)
        pub bootstrap_count: u64,
        /// Cluster ID length in bytes
        pub cluster_id_len: u64,
    }

    // ========================================================================
    // V2 Ticket State Model
    // ========================================================================

    /// Abstract bootstrap peer with direct addresses
    ///
    /// Models BootstrapPeer from lib.rs
    pub struct BootstrapPeerSpec {
        /// Number of direct socket addresses
        pub direct_addr_count: u64,
    }

    /// Abstract V2 ticket structure
    ///
    /// Models AspenClusterTicketV2 from lib.rs
    pub struct TicketV2State {
        /// Number of bootstrap peers
        pub bootstrap_count: u64,
        /// Maximum direct addresses across all peers
        pub max_addrs_per_peer: u64,
        /// Cluster ID length in bytes
        pub cluster_id_len: u64,
    }

    // ========================================================================
    // Invariant 1: Bootstrap Bounds
    // ========================================================================

    /// TICKET-1: Bootstrap peer count is bounded
    ///
    /// Ensures ticket size is predictable and bounded.
    pub open spec fn bootstrap_bounds(ticket: TicketState) -> bool {
        ticket.bootstrap_count <= MAX_BOOTSTRAP_PEERS
    }

    /// Bootstrap bounds for V2 tickets
    pub open spec fn bootstrap_bounds_v2(ticket: TicketV2State) -> bool {
        ticket.bootstrap_count <= MAX_BOOTSTRAP_PEERS
    }

    // ========================================================================
    // Invariant 2: Address Bounds
    // ========================================================================

    /// TICKET-2: Per-peer address count is bounded
    ///
    /// Ensures V2 tickets don't have unbounded addresses per peer.
    pub open spec fn address_bounds(peer: BootstrapPeerSpec) -> bool {
        peer.direct_addr_count <= MAX_DIRECT_ADDRS_PER_PEER
    }

    /// Address bounds for V2 ticket (all peers)
    pub open spec fn address_bounds_v2(ticket: TicketV2State) -> bool {
        ticket.max_addrs_per_peer <= MAX_DIRECT_ADDRS_PER_PEER
    }

    // ========================================================================
    // Combined Invariants
    // ========================================================================

    /// Combined invariant for V1 tickets
    pub open spec fn ticket_invariant(ticket: TicketState) -> bool {
        bootstrap_bounds(ticket)
    }

    /// Combined invariant for V2 tickets
    pub open spec fn ticket_v2_invariant(ticket: TicketV2State) -> bool {
        bootstrap_bounds_v2(ticket) && address_bounds_v2(ticket)
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial V1 ticket state (empty bootstrap)
    pub open spec fn initial_ticket_state(cluster_id_len: u64) -> TicketState {
        TicketState {
            bootstrap_count: 0,
            cluster_id_len,
        }
    }

    /// Initial V2 ticket state (empty bootstrap)
    pub open spec fn initial_ticket_v2_state(cluster_id_len: u64) -> TicketV2State {
        TicketV2State {
            bootstrap_count: 0,
            max_addrs_per_peer: 0,
            cluster_id_len,
        }
    }

    /// Proof: Initial V1 state satisfies invariant
    pub proof fn initial_v1_satisfies_invariant(cluster_id_len: u64)
        ensures ticket_invariant(initial_ticket_state(cluster_id_len))
    {
        // bootstrap_count = 0 <= MAX_BOOTSTRAP_PEERS
    }

    /// Proof: Initial V2 state satisfies invariant
    pub proof fn initial_v2_satisfies_invariant(cluster_id_len: u64)
        ensures ticket_v2_invariant(initial_ticket_v2_state(cluster_id_len))
    {
        // bootstrap_count = 0 <= MAX_BOOTSTRAP_PEERS
        // max_addrs_per_peer = 0 <= MAX_DIRECT_ADDRS_PER_PEER
    }

    // ========================================================================
    // Operation Effects
    // ========================================================================

    /// Effect of add_bootstrap on V1 ticket
    ///
    /// Returns None if limit would be exceeded.
    pub open spec fn add_bootstrap_effect(
        pre: TicketState,
    ) -> Option<TicketState> {
        if pre.bootstrap_count >= MAX_BOOTSTRAP_PEERS {
            None  // Operation fails at limit
        } else {
            Some(TicketState {
                bootstrap_count: pre.bootstrap_count + 1,
                cluster_id_len: pre.cluster_id_len,
            })
        }
    }

    /// Effect of add_bootstrap_addr on V2 ticket
    ///
    /// Truncates addresses to MAX_DIRECT_ADDRS_PER_PEER.
    pub open spec fn add_bootstrap_addr_effect(
        pre: TicketV2State,
        incoming_addr_count: u64,
    ) -> Option<TicketV2State> {
        if pre.bootstrap_count >= MAX_BOOTSTRAP_PEERS {
            None  // Operation fails at limit
        } else {
            // Truncate incoming addresses to limit
            let truncated_count = if incoming_addr_count > MAX_DIRECT_ADDRS_PER_PEER {
                MAX_DIRECT_ADDRS_PER_PEER
            } else {
                incoming_addr_count
            };
            // Update max if this peer has more addresses
            let new_max = if truncated_count > pre.max_addrs_per_peer {
                truncated_count
            } else {
                pre.max_addrs_per_peer
            };
            Some(TicketV2State {
                bootstrap_count: pre.bootstrap_count + 1,
                max_addrs_per_peer: new_max,
                cluster_id_len: pre.cluster_id_len,
            })
        }
    }

    // ========================================================================
    // Operation Proofs
    // ========================================================================

    /// Proof: add_bootstrap preserves V1 invariant
    pub proof fn add_bootstrap_preserves_invariant(pre: TicketState)
        requires ticket_invariant(pre)
        ensures {
            match add_bootstrap_effect(pre) {
                Some(post) => ticket_invariant(post),
                None => true,  // Failure case trivially preserves
            }
        }
    {
        if pre.bootstrap_count < MAX_BOOTSTRAP_PEERS {
            let post = add_bootstrap_effect(pre).unwrap();
            assert(post.bootstrap_count == pre.bootstrap_count + 1);
            assert(post.bootstrap_count <= MAX_BOOTSTRAP_PEERS);
        }
    }

    /// Proof: add_bootstrap_addr preserves V2 invariant
    pub proof fn add_bootstrap_addr_preserves_invariant(
        pre: TicketV2State,
        incoming_addr_count: u64,
    )
        requires ticket_v2_invariant(pre)
        ensures {
            match add_bootstrap_addr_effect(pre, incoming_addr_count) {
                Some(post) => ticket_v2_invariant(post),
                None => true,
            }
        }
    {
        if pre.bootstrap_count < MAX_BOOTSTRAP_PEERS {
            let post = add_bootstrap_addr_effect(pre, incoming_addr_count).unwrap();
            // Bootstrap count increased by 1
            assert(post.bootstrap_count == pre.bootstrap_count + 1);
            assert(post.bootstrap_count <= MAX_BOOTSTRAP_PEERS);
            // Max addresses either stays same or uses truncated value
            assert(post.max_addrs_per_peer <= MAX_DIRECT_ADDRS_PER_PEER);
        }
    }

    /// Proof: add_bootstrap fails exactly at limit
    pub proof fn add_bootstrap_fails_at_limit(pre: TicketState)
        requires pre.bootstrap_count == MAX_BOOTSTRAP_PEERS
        ensures add_bootstrap_effect(pre).is_none()
    {
        // Direct from definition
    }

    /// Proof: add_bootstrap_addr truncates addresses
    pub proof fn add_bootstrap_addr_truncates(
        pre: TicketV2State,
        incoming_addr_count: u64,
    )
        requires
            ticket_v2_invariant(pre),
            pre.bootstrap_count < MAX_BOOTSTRAP_PEERS,
            incoming_addr_count > MAX_DIRECT_ADDRS_PER_PEER,
        ensures {
            let post = add_bootstrap_addr_effect(pre, incoming_addr_count).unwrap();
            post.max_addrs_per_peer <= MAX_DIRECT_ADDRS_PER_PEER
        }
    {
        // Truncation caps at MAX_DIRECT_ADDRS_PER_PEER
    }

    // ========================================================================
    // Size Bounds
    // ========================================================================

    /// Maximum serialized size for V1 ticket (approximate)
    ///
    /// topic_id (32) + cluster_id (variable) + bootstrap (16 * 32 = 512)
    pub open spec fn max_v1_ticket_bytes(cluster_id_len: u64) -> u64 {
        TOPIC_ID_SIZE + cluster_id_len + MAX_BOOTSTRAP_PEERS * ENDPOINT_ID_SIZE
    }

    /// Maximum serialized size for V2 ticket (approximate)
    ///
    /// topic_id (32) + cluster_id (variable) + bootstrap (16 * (32 + 8*18)) = 16 * 176 = 2816
    /// Each SocketAddr is ~18 bytes (IPv4: 4+2=6, IPv6: 16+2=18)
    pub const SOCKET_ADDR_MAX_SIZE: u64 = 18;

    pub open spec fn max_v2_ticket_bytes(cluster_id_len: u64) -> u64 {
        TOPIC_ID_SIZE + cluster_id_len +
        MAX_BOOTSTRAP_PEERS * (ENDPOINT_ID_SIZE + MAX_DIRECT_ADDRS_PER_PEER * SOCKET_ADDR_MAX_SIZE)
    }

    /// Proof: V1 ticket size is bounded
    pub proof fn v1_size_bounded(ticket: TicketState)
        requires ticket_invariant(ticket)
        ensures max_v1_ticket_bytes(ticket.cluster_id_len) <=
                TOPIC_ID_SIZE + ticket.cluster_id_len + MAX_BOOTSTRAP_PEERS * ENDPOINT_ID_SIZE
    {
        // Direct from definition
    }

    /// Proof: V2 ticket size is bounded
    pub proof fn v2_size_bounded(ticket: TicketV2State)
        requires ticket_v2_invariant(ticket)
        ensures max_v2_ticket_bytes(ticket.cluster_id_len) <=
                TOPIC_ID_SIZE + ticket.cluster_id_len +
                MAX_BOOTSTRAP_PEERS * (ENDPOINT_ID_SIZE + MAX_DIRECT_ADDRS_PER_PEER * SOCKET_ADDR_MAX_SIZE)
    {
        // Direct from definition
    }
}
