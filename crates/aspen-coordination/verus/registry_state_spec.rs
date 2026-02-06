//! Service Registry State Machine Model
//!
//! Abstract state model for formal verification of service registry operations.
//!
//! # State Model
//!
//! The `RegistryState` captures:
//! - Registered services with metadata
//! - TTL-based liveness tracking
//! - Fencing tokens for registration epochs
//!
//! # Key Invariants
//!
//! 1. **REG-1: Registration Validity**: Valid TTLs and timestamps
//! 2. **REG-2: Fencing Token Monotonicity**: Tokens increase on re-registration
//! 3. **REG-3: Service Index Consistency**: Index entries match registrations
//! 4. **REG-4: TTL Expiration Safety**: Expired services become available
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/registry_state_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Service registration entry
    pub struct ServiceEntrySpec {
        /// Service identifier
        pub service_id: Seq<u8>,
        /// Service type/name
        pub service_type: Seq<u8>,
        /// Instance identifier
        pub instance_id: Seq<u8>,
        /// Service endpoint address
        pub endpoint: Seq<u8>,
        /// Fencing token (monotonically increasing)
        pub fencing_token: u64,
        /// Registration timestamp (Unix ms)
        pub registered_at_ms: u64,
        /// Last heartbeat timestamp (Unix ms)
        pub last_heartbeat_ms: u64,
        /// TTL in milliseconds
        pub ttl_ms: u64,
        /// Deadline = last_heartbeat_ms + ttl_ms
        pub deadline_ms: u64,
        /// Service metadata (key-value pairs)
        pub metadata: Map<Seq<u8>, Seq<u8>>,
        /// Service weight for load balancing
        pub weight: u32,
        /// Whether service is healthy
        pub healthy: bool,
    }

    /// Service type index entry (for lookups by type)
    pub struct ServiceTypeIndexSpec {
        /// Service type name
        pub service_type: Seq<u8>,
        /// Set of service IDs of this type
        pub service_ids: Set<Seq<u8>>,
    }

    /// Complete registry state
    pub struct RegistryState {
        /// All registered services (service_id -> entry)
        pub services: Map<Seq<u8>, ServiceEntrySpec>,
        /// Service type index (type -> list of service_ids)
        pub type_index: Map<Seq<u8>, Set<Seq<u8>>>,
        /// Maximum fencing token ever issued (for monotonicity)
        pub max_fencing_token: u64,
        /// Current time (for expiration checks)
        pub current_time_ms: u64,
    }

    // ========================================================================
    // Invariant 1: Registration Validity
    // ========================================================================

    /// REG-1: Valid TTLs and timestamps
    pub open spec fn registration_valid(entry: ServiceEntrySpec) -> bool {
        // TTL is positive and bounded
        entry.ttl_ms > 0 &&
        entry.ttl_ms <= 86400000 && // MAX_REGISTRY_TTL_MS (24h)
        // Deadline computed correctly
        entry.deadline_ms == entry.last_heartbeat_ms + entry.ttl_ms &&
        // Registration time <= last heartbeat
        entry.registered_at_ms <= entry.last_heartbeat_ms &&
        // Weight is reasonable
        entry.weight > 0 && entry.weight <= 1000 &&
        // Service ID, type, and endpoint are non-empty
        entry.service_id.len() > 0 &&
        entry.service_type.len() > 0 &&
        entry.endpoint.len() > 0
    }

    /// All registrations are valid
    pub open spec fn all_registrations_valid(state: RegistryState) -> bool {
        forall |id: Seq<u8>| state.services.contains_key(id) ==>
            registration_valid(state.services[id])
    }

    // ========================================================================
    // Invariant 2: Fencing Token Monotonicity
    // ========================================================================

    /// REG-2: Fencing tokens increase on re-registration
    pub open spec fn fencing_token_monotonic(pre: RegistryState, post: RegistryState) -> bool {
        post.max_fencing_token >= pre.max_fencing_token
    }

    /// Entry token bounded by max
    pub open spec fn entry_token_bounded(state: RegistryState) -> bool {
        forall |id: Seq<u8>| state.services.contains_key(id) ==>
            state.services[id].fencing_token <= state.max_fencing_token
    }

    // ========================================================================
    // Invariant 3: Service Index Consistency
    // ========================================================================

    /// REG-3: Index entries match registrations
    pub open spec fn index_consistent(state: RegistryState) -> bool {
        // Forward: every service in index exists in services
        (forall |svc_type: Seq<u8>, svc_id: Seq<u8>>|
            state.type_index.contains_key(svc_type) &&
            state.type_index[svc_type].contains(svc_id) ==>
            state.services.contains_key(svc_id) &&
            state.services[svc_id].service_type =~= svc_type
        ) &&
        // Backward: every service appears in its type's index
        (forall |svc_id: Seq<u8>| state.services.contains_key(svc_id) ==> {
            let entry = state.services[svc_id];
            state.type_index.contains_key(entry.service_type) &&
            state.type_index[entry.service_type].contains(svc_id)
        })
    }

    // ========================================================================
    // Invariant 4: TTL Expiration Safety
    // ========================================================================

    /// Check if a service is expired
    pub open spec fn is_expired(entry: ServiceEntrySpec, current_time_ms: u64) -> bool {
        current_time_ms > entry.deadline_ms
    }

    /// REG-4: Expired services can be re-registered
    pub open spec fn expired_is_available(state: RegistryState, service_id: Seq<u8>) -> bool {
        state.services.contains_key(service_id) &&
        is_expired(state.services[service_id], state.current_time_ms)
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined registry invariant
    pub open spec fn registry_invariant(state: RegistryState) -> bool {
        all_registrations_valid(state) &&
        entry_token_bounded(state) &&
        index_consistent(state)
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial empty registry state
    pub open spec fn initial_registry_state() -> RegistryState {
        RegistryState {
            services: Map::empty(),
            type_index: Map::empty(),
            max_fencing_token: 0,
            current_time_ms: 0,
        }
    }

    /// Proof: Initial state satisfies invariant
    pub proof fn initial_state_invariant()
        ensures registry_invariant(initial_registry_state())
    {
        // Empty maps trivially satisfy all forall properties
    }

    // ========================================================================
    // Helper Predicates
    // ========================================================================

    /// Check if service exists and is live
    pub open spec fn is_live(state: RegistryState, service_id: Seq<u8>) -> bool {
        state.services.contains_key(service_id) &&
        !is_expired(state.services[service_id], state.current_time_ms) &&
        state.services[service_id].healthy
    }

    /// Get all live services of a type
    pub open spec fn live_services_of_type(
        state: RegistryState,
        service_type: Seq<u8>,
    ) -> Set<Seq<u8>> {
        Set::new(|id: Seq<u8>|
            state.services.contains_key(id) &&
            state.services[id].service_type =~= service_type &&
            is_live(state, id)
        )
    }

    /// Count of live services
    pub open spec fn live_service_count(state: RegistryState) -> int {
        // Abstract count - would need to sum over services
        0
    }
}
