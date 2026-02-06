//! Service Registry Operations Specification
//!
//! Formal specifications for registry register, deregister, and heartbeat operations.
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/registry_ops_spec.rs
//! ```

use vstd::prelude::*;

// Import from registry_state_spec
use crate::registry_state_spec::*;

verus! {
    // ========================================================================
    // Register Operation
    // ========================================================================

    /// Precondition for registering a service
    pub open spec fn register_pre(
        state: RegistryState,
        service_id: Seq<u8>,
        service_type: Seq<u8>,
        endpoint: Seq<u8>,
        ttl_ms: u64,
    ) -> bool {
        // Non-empty identifiers
        service_id.len() > 0 &&
        service_type.len() > 0 &&
        endpoint.len() > 0 &&
        // TTL within bounds
        ttl_ms > 0 &&
        ttl_ms <= 86400000 // MAX_REGISTRY_TTL_MS
    }

    /// Effect of registering a new service
    pub open spec fn register_post(
        pre: RegistryState,
        service_id: Seq<u8>,
        service_type: Seq<u8>,
        instance_id: Seq<u8>,
        endpoint: Seq<u8>,
        ttl_ms: u64,
        weight: u32,
        metadata: Map<Seq<u8>, Seq<u8>>,
        current_time_ms: u64,
    ) -> RegistryState
        recommends register_pre(pre, service_id, service_type, endpoint, ttl_ms)
    {
        let new_token = pre.max_fencing_token + 1;

        let entry = ServiceEntrySpec {
            service_id: service_id,
            service_type: service_type,
            instance_id: instance_id,
            endpoint: endpoint,
            fencing_token: new_token,
            registered_at_ms: current_time_ms,
            last_heartbeat_ms: current_time_ms,
            ttl_ms: ttl_ms,
            deadline_ms: current_time_ms + ttl_ms,
            metadata: metadata,
            weight: if weight == 0 { 1 } else { weight.min(1000) },
            healthy: true,
        };

        // Update type index
        let old_type_set = if pre.type_index.contains_key(service_type) {
            pre.type_index[service_type]
        } else {
            Set::empty()
        };
        let new_type_set = old_type_set.insert(service_id);

        RegistryState {
            services: pre.services.insert(service_id, entry),
            type_index: pre.type_index.insert(service_type, new_type_set),
            max_fencing_token: new_token,
            current_time_ms: current_time_ms,
        }
    }

    /// Proof: Register creates valid entry
    pub proof fn register_creates_valid_entry(
        pre: RegistryState,
        service_id: Seq<u8>,
        service_type: Seq<u8>,
        instance_id: Seq<u8>,
        endpoint: Seq<u8>,
        ttl_ms: u64,
        weight: u32,
        metadata: Map<Seq<u8>, Seq<u8>>,
        current_time_ms: u64,
    )
        requires
            register_pre(pre, service_id, service_type, endpoint, ttl_ms),
            weight > 0,
        ensures {
            let post = register_post(pre, service_id, service_type, instance_id, endpoint, ttl_ms, weight, metadata, current_time_ms);
            registration_valid(post.services[service_id])
        }
    {
        // Entry satisfies all validity constraints
    }

    /// Proof: Register increases fencing token
    pub proof fn register_increases_token(
        pre: RegistryState,
        service_id: Seq<u8>,
        service_type: Seq<u8>,
        instance_id: Seq<u8>,
        endpoint: Seq<u8>,
        ttl_ms: u64,
        weight: u32,
        metadata: Map<Seq<u8>, Seq<u8>>,
        current_time_ms: u64,
    )
        requires register_pre(pre, service_id, service_type, endpoint, ttl_ms)
        ensures {
            let post = register_post(pre, service_id, service_type, instance_id, endpoint, ttl_ms, weight, metadata, current_time_ms);
            post.max_fencing_token == pre.max_fencing_token + 1 &&
            fencing_token_monotonic(pre, post)
        }
    {
        // Token increments by 1
    }

    /// Proof: Register maintains index consistency
    pub proof fn register_maintains_index(
        pre: RegistryState,
        service_id: Seq<u8>,
        service_type: Seq<u8>,
        instance_id: Seq<u8>,
        endpoint: Seq<u8>,
        ttl_ms: u64,
        weight: u32,
        metadata: Map<Seq<u8>, Seq<u8>>,
        current_time_ms: u64,
    )
        requires
            registry_invariant(pre),
            register_pre(pre, service_id, service_type, endpoint, ttl_ms),
            weight > 0,
        ensures {
            let post = register_post(pre, service_id, service_type, instance_id, endpoint, ttl_ms, weight, metadata, current_time_ms);
            index_consistent(post)
        }
    {
        // New service added to both services and type_index
    }

    // ========================================================================
    // Deregister Operation
    // ========================================================================

    /// Precondition for deregistering
    pub open spec fn deregister_pre(
        state: RegistryState,
        service_id: Seq<u8>,
    ) -> bool {
        state.services.contains_key(service_id)
    }

    /// Effect of deregistering a service
    pub open spec fn deregister_post(
        pre: RegistryState,
        service_id: Seq<u8>,
    ) -> RegistryState
        recommends deregister_pre(pre, service_id)
    {
        let entry = pre.services[service_id];
        let service_type = entry.service_type;

        // Update type index
        let old_type_set = pre.type_index[service_type];
        let new_type_set = old_type_set.remove(service_id);

        RegistryState {
            services: pre.services.remove(service_id),
            type_index: if new_type_set.len() == 0 {
                pre.type_index.remove(service_type)
            } else {
                pre.type_index.insert(service_type, new_type_set)
            },
            max_fencing_token: pre.max_fencing_token,
            current_time_ms: pre.current_time_ms,
        }
    }

    /// Proof: Deregister removes service
    pub proof fn deregister_removes_service(
        pre: RegistryState,
        service_id: Seq<u8>,
    )
        requires
            registry_invariant(pre),
            deregister_pre(pre, service_id),
        ensures {
            let post = deregister_post(pre, service_id);
            !post.services.contains_key(service_id)
        }
    {
        // Service removed from services map
    }

    /// Proof: Deregister maintains index consistency
    pub proof fn deregister_maintains_index(
        pre: RegistryState,
        service_id: Seq<u8>,
    )
        requires
            registry_invariant(pre),
            deregister_pre(pre, service_id),
        ensures index_consistent(deregister_post(pre, service_id))
    {
        // Service removed from both services and type_index
    }

    // ========================================================================
    // Heartbeat Operation
    // ========================================================================

    /// Precondition for heartbeat
    pub open spec fn heartbeat_pre(
        state: RegistryState,
        service_id: Seq<u8>,
        fencing_token: u64,
    ) -> bool {
        // Service exists
        state.services.contains_key(service_id) &&
        // Token matches (prevents stale heartbeats)
        state.services[service_id].fencing_token == fencing_token
    }

    /// Effect of successful heartbeat
    pub open spec fn heartbeat_post(
        pre: RegistryState,
        service_id: Seq<u8>,
        current_time_ms: u64,
    ) -> RegistryState
        recommends pre.services.contains_key(service_id)
    {
        let old_entry = pre.services[service_id];
        let new_entry = ServiceEntrySpec {
            last_heartbeat_ms: current_time_ms,
            deadline_ms: current_time_ms + old_entry.ttl_ms,
            healthy: true,
            ..old_entry
        };

        RegistryState {
            services: pre.services.insert(service_id, new_entry),
            current_time_ms: current_time_ms,
            ..pre
        }
    }

    /// Proof: Heartbeat extends deadline
    pub proof fn heartbeat_extends_deadline(
        pre: RegistryState,
        service_id: Seq<u8>,
        fencing_token: u64,
        current_time_ms: u64,
    )
        requires
            heartbeat_pre(pre, service_id, fencing_token),
            current_time_ms >= pre.services[service_id].last_heartbeat_ms,
        ensures {
            let post = heartbeat_post(pre, service_id, current_time_ms);
            post.services[service_id].deadline_ms >= pre.services[service_id].deadline_ms
        }
    {
        // new_deadline = current_time + ttl >= old_heartbeat + ttl = old_deadline
    }

    /// Proof: Heartbeat preserves fencing token
    pub proof fn heartbeat_preserves_token(
        pre: RegistryState,
        service_id: Seq<u8>,
        current_time_ms: u64,
    )
        requires pre.services.contains_key(service_id)
        ensures {
            let post = heartbeat_post(pre, service_id, current_time_ms);
            post.services[service_id].fencing_token == pre.services[service_id].fencing_token
        }
    {
        // Heartbeat doesn't change fencing token
    }

    /// Proof: Heartbeat marks service healthy
    pub proof fn heartbeat_marks_healthy(
        pre: RegistryState,
        service_id: Seq<u8>,
        current_time_ms: u64,
    )
        requires pre.services.contains_key(service_id)
        ensures {
            let post = heartbeat_post(pre, service_id, current_time_ms);
            post.services[service_id].healthy
        }
    {
        // healthy set to true
    }

    // ========================================================================
    // Mark Unhealthy Operation
    // ========================================================================

    /// Effect of marking service unhealthy (e.g., health check failure)
    pub open spec fn mark_unhealthy_post(
        pre: RegistryState,
        service_id: Seq<u8>,
    ) -> RegistryState
        recommends pre.services.contains_key(service_id)
    {
        let old_entry = pre.services[service_id];
        let new_entry = ServiceEntrySpec {
            healthy: false,
            ..old_entry
        };

        RegistryState {
            services: pre.services.insert(service_id, new_entry),
            ..pre
        }
    }

    /// Proof: Mark unhealthy removes from live set
    pub proof fn mark_unhealthy_removes_from_live(
        pre: RegistryState,
        service_id: Seq<u8>,
    )
        requires
            pre.services.contains_key(service_id),
            is_live(pre, service_id),
        ensures {
            let post = mark_unhealthy_post(pre, service_id);
            !is_live(post, service_id)
        }
    {
        // healthy = false means not live
    }

    // ========================================================================
    // Cleanup Expired Operation
    // ========================================================================

    /// Effect of cleaning up a single expired service
    pub open spec fn cleanup_expired_single(
        pre: RegistryState,
        service_id: Seq<u8>,
    ) -> RegistryState
        recommends
            pre.services.contains_key(service_id),
            is_expired(pre.services[service_id], pre.current_time_ms)
    {
        deregister_post(pre, service_id)
    }

    /// Proof: Cleanup removes expired services
    pub proof fn cleanup_removes_expired(
        pre: RegistryState,
        service_id: Seq<u8>,
    )
        requires
            registry_invariant(pre),
            pre.services.contains_key(service_id),
            is_expired(pre.services[service_id], pre.current_time_ms),
        ensures {
            let post = cleanup_expired_single(pre, service_id);
            !post.services.contains_key(service_id)
        }
    {
        // Same as deregister_removes_service
    }

    // ========================================================================
    // Lookup Operations
    // ========================================================================

    /// Get all services of a type
    pub open spec fn get_services_by_type(
        state: RegistryState,
        service_type: Seq<u8>,
    ) -> Set<ServiceEntrySpec> {
        if !state.type_index.contains_key(service_type) {
            Set::empty()
        } else {
            Set::new(|entry: ServiceEntrySpec|
                state.services.contains_key(entry.service_id) &&
                state.services[entry.service_id] == entry &&
                entry.service_type =~= service_type
            )
        }
    }

    /// Get live services of a type (for load balancing)
    pub open spec fn get_live_services_by_type(
        state: RegistryState,
        service_type: Seq<u8>,
    ) -> Set<ServiceEntrySpec> {
        Set::new(|entry: ServiceEntrySpec|
            state.services.contains_key(entry.service_id) &&
            state.services[entry.service_id] == entry &&
            entry.service_type =~= service_type &&
            !is_expired(entry, state.current_time_ms) &&
            entry.healthy
        )
    }

    /// Proof: Lookup returns consistent results
    pub proof fn lookup_is_consistent(
        state: RegistryState,
        service_type: Seq<u8>,
    )
        requires
            registry_invariant(state),
            state.type_index.contains_key(service_type),
        ensures {
            let services = get_services_by_type(state, service_type);
            // All returned services have the correct type
            forall |entry: ServiceEntrySpec| services.contains(entry) ==>
                entry.service_type =~= service_type
        }
    {
        // Follows from index_consistent
    }
}

mod registry_state_spec;
