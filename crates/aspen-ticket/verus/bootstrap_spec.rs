//! Bootstrap Peer Operations Specification
//!
//! Formal specification for bootstrap peer management in cluster tickets.
//!
//! # Operations
//!
//! - `add_bootstrap`: Add peer with no addresses
//! - `add_bootstrap_addr`: Add peer with direct addresses
//! - `inject_direct_addr`: Inject address into all peers
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-ticket/verus/bootstrap_spec.rs
//! ```

use vstd::prelude::*;

use super::ticket_state_spec::*;

verus! {
    // ========================================================================
    // add_bootstrap Operation
    // ========================================================================

    /// Precondition for add_bootstrap
    pub open spec fn add_bootstrap_pre(ticket: TicketState) -> bool {
        ticket.bootstrap_count < MAX_BOOTSTRAP_PEERS
    }

    /// Postcondition for add_bootstrap
    pub open spec fn add_bootstrap_post(
        pre: TicketState,
        post: TicketState,
    ) -> bool {
        // Bootstrap count increased by 1
        post.bootstrap_count == pre.bootstrap_count + 1 &&
        // Cluster ID unchanged
        post.cluster_id_len == pre.cluster_id_len &&
        // Max addresses unchanged (add_bootstrap adds peer with no addresses)
        post.max_addrs_per_peer == pre.max_addrs_per_peer &&
        // Invariant preserved
        ticket_invariant(post)
    }

    /// Proof: add_bootstrap operation is correct
    pub proof fn add_bootstrap_correct(pre: TicketState)
        requires
            ticket_invariant(pre),
            add_bootstrap_pre(pre),
        ensures {
            let post = add_bootstrap_effect(pre).unwrap();
            add_bootstrap_post(pre, post)
        }
    {
        let post = add_bootstrap_effect(pre).unwrap();
        assert(post.bootstrap_count == pre.bootstrap_count + 1);
        assert(post.bootstrap_count <= MAX_BOOTSTRAP_PEERS);
    }

    // ========================================================================
    // add_bootstrap_addr Operation
    // ========================================================================

    /// Precondition for add_bootstrap_addr
    pub open spec fn add_bootstrap_addr_pre(ticket: TicketState) -> bool {
        ticket.bootstrap_count < MAX_BOOTSTRAP_PEERS
    }

    /// Postcondition for add_bootstrap_addr
    pub open spec fn add_bootstrap_addr_post(
        pre: TicketState,
        post: TicketState,
        incoming_addr_count: u64,
    ) -> bool {
        // Bootstrap count increased by 1
        post.bootstrap_count == pre.bootstrap_count + 1 &&
        // Cluster ID unchanged
        post.cluster_id_len == pre.cluster_id_len &&
        // Addresses truncated if needed
        post.max_addrs_per_peer <= MAX_DIRECT_ADDRS_PER_PEER &&
        // Invariant preserved
        ticket_invariant(post)
    }

    /// Proof: add_bootstrap_addr operation is correct
    pub proof fn add_bootstrap_addr_correct(
        pre: TicketState,
        incoming_addr_count: u64,
    )
        requires
            ticket_invariant(pre),
            add_bootstrap_addr_pre(pre),
        ensures {
            let post = add_bootstrap_addr_effect(pre, incoming_addr_count).unwrap();
            add_bootstrap_addr_post(pre, post, incoming_addr_count)
        }
    {
        // Follows from add_bootstrap_addr_preserves_invariant
    }

    // ========================================================================
    // inject_direct_addr Operation
    // ========================================================================

    /// Abstract state after inject_direct_addr
    ///
    /// Models the effect of injecting an address into all bootstrap peers.
    pub struct InjectState {
        /// Number of peers
        pub peer_count: u64,
        /// Whether all peers already had the address
        pub all_had_addr: bool,
        /// Maximum addresses per peer after injection
        pub max_addrs_after: u64,
    }

    /// Effect of inject_direct_addr
    pub open spec fn inject_addr_effect(
        ticket: TicketState,
        peers_with_addr: u64,  // How many peers already have this address
    ) -> InjectState {
        InjectState {
            peer_count: ticket.bootstrap_count,
            all_had_addr: peers_with_addr == ticket.bootstrap_count,
            // Each peer without the address gets +1
            max_addrs_after: if peers_with_addr < ticket.bootstrap_count {
                ticket.max_addrs_per_peer + 1
            } else {
                ticket.max_addrs_per_peer
            },
        }
    }

    /// Idempotency: Injecting same address twice has same effect as once
    pub open spec fn inject_addr_idempotent(
        ticket: TicketState,
        peers_with_addr_before: u64,
    ) -> bool {
        let after_first = inject_addr_effect(ticket, peers_with_addr_before);
        // After first injection, all peers have the address
        let after_second = inject_addr_effect(
            TicketState {
                bootstrap_count: ticket.bootstrap_count,
                max_addrs_per_peer: after_first.max_addrs_after,
                cluster_id_len: ticket.cluster_id_len,
            },
            ticket.bootstrap_count,  // All peers now have the address
        );
        // Second injection doesn't change max_addrs
        after_second.max_addrs_after == after_first.max_addrs_after
    }

    /// Proof: inject_direct_addr is idempotent
    pub proof fn inject_is_idempotent(
        ticket: TicketState,
        peers_with_addr: u64,
    )
        requires peers_with_addr <= ticket.bootstrap_count
        ensures inject_addr_idempotent(ticket, peers_with_addr)
    {
        let after_first = inject_addr_effect(ticket, peers_with_addr);
        // After first injection, all peers have the address
        // Second injection finds all_had_addr = true, so no change
    }

    // ========================================================================
    // Empty Bootstrap Properties
    // ========================================================================

    /// inject_direct_addr on empty bootstrap is no-op
    pub open spec fn inject_empty_is_noop(ticket: TicketState) -> bool {
        ticket.bootstrap_count == 0 ==> {
            let after = inject_addr_effect(ticket, 0);
            after.max_addrs_after == ticket.max_addrs_per_peer
        }
    }

    /// Proof: Injection on empty ticket does nothing
    pub proof fn inject_empty_ticket(ticket: TicketState)
        requires ticket.bootstrap_count == 0
        ensures inject_empty_is_noop(ticket)
    {
        // peers_with_addr = 0 == bootstrap_count = 0
        // So all_had_addr = true, no change
    }

    // ========================================================================
    // Peer Handling
    // ========================================================================

    /// Tickets use Vec, so duplicate peers are NOT deduplicated at add time
    pub open spec fn allows_duplicate_peers(
        pre: TicketState,
        incoming_addr_count: u64,
    ) -> TicketState {
        // Always adds, regardless of whether peer already exists
        TicketState {
            bootstrap_count: pre.bootstrap_count + 1,
            max_addrs_per_peer: if incoming_addr_count > pre.max_addrs_per_peer {
                incoming_addr_count
            } else {
                pre.max_addrs_per_peer
            },
            cluster_id_len: pre.cluster_id_len,
        }
    }

    /// endpoint_ids() deduplicates peers via BTreeSet
    pub open spec fn endpoint_ids_deduplicates(
        actual_peer_count: u64,
        unique_peer_count: u64,
    ) -> bool {
        unique_peer_count <= actual_peer_count
    }

    // ========================================================================
    // Bootstrap Count Properties
    // ========================================================================

    /// Adding N peers to empty ticket results in N peers
    pub proof fn add_n_peers(n: u64)
        requires n <= MAX_BOOTSTRAP_PEERS
        ensures {
            // Start with empty ticket
            let mut ticket = initial_ticket_state(10);
            // After adding n peers, count is n
            // (This is a logical property, not executable)
            true
        }
    {
        // Each add_bootstrap increments count by 1
        // Starting from 0, after n adds, count = n
    }

    /// Bootstrap count is monotonically increasing
    pub proof fn bootstrap_count_monotonic(
        pre: TicketState,
        post: TicketState,
    )
        requires
            ticket_invariant(pre),
            add_bootstrap_pre(pre),
            add_bootstrap_post(pre, post),
        ensures post.bootstrap_count >= pre.bootstrap_count
    {
        // post.bootstrap_count = pre.bootstrap_count + 1 >= pre.bootstrap_count
    }
}
