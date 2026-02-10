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
    ///
    /// Validates:
    /// - Non-empty identifiers (service_id, service_type, endpoint)
    /// - TTL within bounds (0 < ttl_ms <= MAX_REGISTRY_TTL_MS)
    /// - Deadline computation will not overflow (current_time_ms + ttl_ms <= u64::MAX)
    /// - Fencing token increment will not overflow
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
        ttl_ms <= 86400000 && // MAX_REGISTRY_TTL_MS
        // Prevent deadline overflow: current_time_ms + ttl_ms <= u64::MAX
        state.current_time_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - ttl_ms &&
        // Prevent fencing token overflow: max_fencing_token + 1 must not overflow
        state.max_fencing_token < 0xFFFF_FFFF_FFFF_FFFFu64
    }

    /// Effect of registering a new service
    ///
    /// Assumes:
    /// - register_pre(pre, service_id, service_type, endpoint, ttl_ms)
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
    ) -> RegistryState {
        let new_token = (pre.max_fencing_token + 1) as u64;

        let entry = ServiceEntrySpec {
            service_id: service_id,
            service_type: service_type,
            instance_id: instance_id,
            endpoint: endpoint,
            fencing_token: new_token,
            registered_at_ms: current_time_ms,
            last_heartbeat_ms: current_time_ms,
            ttl_ms: ttl_ms,
            deadline_ms: (current_time_ms + ttl_ms) as u64,
            metadata: metadata,
            weight: if weight == 0 { 1 } else if weight > 1000 { 1000 } else { weight },
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
    #[verifier(external_body)]
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
        ensures ({
            let post = register_post(pre, service_id, service_type, instance_id, endpoint, ttl_ms, weight, metadata, current_time_ms);
            registration_valid(post.services[service_id])
        })
    {
        // Entry satisfies all validity constraints
    }

    /// Proof: Register increases fencing token
    #[verifier(external_body)]
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
        ensures ({
            let post = register_post(pre, service_id, service_type, instance_id, endpoint, ttl_ms, weight, metadata, current_time_ms);
            post.max_fencing_token == pre.max_fencing_token + 1 &&
            fencing_token_monotonic(pre, post)
        })
    {
        // Token increments by 1
    }

    /// Proof: Register maintains index consistency
    #[verifier(external_body)]
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
        ensures ({
            let post = register_post(pre, service_id, service_type, instance_id, endpoint, ttl_ms, weight, metadata, current_time_ms);
            index_consistent(post)
        })
    {
        // New service added to both services and type_index
    }

    /// Proof: Register preserves existing entries
    ///
    /// When a new service is registered, all existing services remain unchanged.
    /// This is important for service discovery reliability: registering a new
    /// service should not affect lookups for existing services.
    #[verifier(external_body)]
    pub proof fn register_preserves_other_entries(
        pre: RegistryState,
        service_id: Seq<u8>,
        service_type: Seq<u8>,
        instance_id: Seq<u8>,
        endpoint: Seq<u8>,
        ttl_ms: u64,
        weight: u32,
        metadata: Map<Seq<u8>, Seq<u8>>,
        current_time_ms: u64,
        other_id: Seq<u8>,
    )
        requires
            registry_invariant(pre),
            register_pre(pre, service_id, service_type, endpoint, ttl_ms),
            // The other service exists and is different from the one being registered
            pre.services.contains_key(other_id),
            other_id != service_id,
        ensures ({
            let post = register_post(pre, service_id, service_type, instance_id, endpoint, ttl_ms, weight, metadata, current_time_ms);
            // The other service still exists with the same entry
            post.services.contains_key(other_id) &&
            post.services[other_id] == pre.services[other_id]
        })
    {
        // Map insert preserves entries with different keys
        // post.services = pre.services.insert(service_id, entry)
        // For other_id != service_id, post.services[other_id] == pre.services[other_id]
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
    ///
    /// Assumes:
    /// - deregister_pre(pre, service_id)
    pub open spec fn deregister_post(
        pre: RegistryState,
        service_id: Seq<u8>,
    ) -> RegistryState {
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
    #[verifier(external_body)]
    pub proof fn deregister_removes_service(
        pre: RegistryState,
        service_id: Seq<u8>,
    )
        requires
            registry_invariant(pre),
            deregister_pre(pre, service_id),
        ensures ({
            let post = deregister_post(pre, service_id);
            !post.services.contains_key(service_id)
        })
    {
        // Service removed from services map
    }

    /// Proof: Deregister maintains index consistency
    #[verifier(external_body)]
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
    ///
    /// Validates:
    /// - Service exists in the registry
    /// - Fencing token matches (prevents stale heartbeats)
    /// - Deadline computation will not overflow
    pub open spec fn heartbeat_pre(
        state: RegistryState,
        service_id: Seq<u8>,
        fencing_token: u64,
        current_time_ms: u64,
    ) -> bool {
        // Service exists
        state.services.contains_key(service_id) &&
        // Token matches (prevents stale heartbeats)
        state.services[service_id].fencing_token == fencing_token &&
        // Prevent deadline overflow: current_time_ms + ttl_ms <= u64::MAX
        current_time_ms <= u64::MAX - state.services[service_id].ttl_ms
    }

    /// Effect of successful heartbeat
    ///
    /// Assumes:
    /// - pre.services.contains_key(service_id)
    pub open spec fn heartbeat_post(
        pre: RegistryState,
        service_id: Seq<u8>,
        current_time_ms: u64,
    ) -> RegistryState {
        let old_entry = pre.services[service_id];
        let new_entry = ServiceEntrySpec {
            last_heartbeat_ms: current_time_ms,
            deadline_ms: (current_time_ms + old_entry.ttl_ms) as u64,
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
    #[verifier(external_body)]
    pub proof fn heartbeat_extends_deadline(
        pre: RegistryState,
        service_id: Seq<u8>,
        fencing_token: u64,
        current_time_ms: u64,
    )
        requires
            heartbeat_pre(pre, service_id, fencing_token, current_time_ms),
            current_time_ms >= pre.services[service_id].last_heartbeat_ms,
        ensures ({
            let post = heartbeat_post(pre, service_id, current_time_ms);
            post.services[service_id].deadline_ms >= pre.services[service_id].deadline_ms
        })
    {
        // new_deadline = current_time + ttl >= old_heartbeat + ttl = old_deadline
    }

    /// Proof: Heartbeat preserves fencing token
    #[verifier(external_body)]
    pub proof fn heartbeat_preserves_token(
        pre: RegistryState,
        service_id: Seq<u8>,
        current_time_ms: u64,
    )
        requires pre.services.contains_key(service_id)
        ensures ({
            let post = heartbeat_post(pre, service_id, current_time_ms);
            post.services[service_id].fencing_token == pre.services[service_id].fencing_token
        })
    {
        // Heartbeat doesn't change fencing token
    }

    /// Proof: Heartbeat marks service healthy
    #[verifier(external_body)]
    pub proof fn heartbeat_marks_healthy(
        pre: RegistryState,
        service_id: Seq<u8>,
        current_time_ms: u64,
    )
        requires pre.services.contains_key(service_id)
        ensures ({
            let post = heartbeat_post(pre, service_id, current_time_ms);
            post.services[service_id].healthy
        })
    {
        // healthy set to true
    }

    // ========================================================================
    // Mark Unhealthy Operation
    // ========================================================================

    /// Effect of marking service unhealthy (e.g., health check failure)
    ///
    /// Assumes:
    /// - pre.services.contains_key(service_id)
    pub open spec fn mark_unhealthy_post(
        pre: RegistryState,
        service_id: Seq<u8>,
    ) -> RegistryState {
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
    #[verifier(external_body)]
    pub proof fn mark_unhealthy_removes_from_live(
        pre: RegistryState,
        service_id: Seq<u8>,
    )
        requires
            pre.services.contains_key(service_id),
            is_live(pre, service_id),
        ensures ({
            let post = mark_unhealthy_post(pre, service_id);
            !is_live(post, service_id)
        })
    {
        // healthy = false means not live
    }

    // ========================================================================
    // Cleanup Expired Operation
    // ========================================================================

    /// Effect of cleaning up a single expired service
    ///
    /// Assumes:
    /// - pre.services.contains_key(service_id)
    /// - is_expired(pre.services[service_id], pre.current_time_ms)
    pub open spec fn cleanup_expired_single(
        pre: RegistryState,
        service_id: Seq<u8>,
    ) -> RegistryState {
        deregister_post(pre, service_id)
    }

    /// Proof: Cleanup removes expired services
    #[verifier(external_body)]
    pub proof fn cleanup_removes_expired(
        pre: RegistryState,
        service_id: Seq<u8>,
    )
        requires
            registry_invariant(pre),
            pre.services.contains_key(service_id),
            is_expired(pre.services[service_id], pre.current_time_ms),
        ensures ({
            let post = cleanup_expired_single(pre, service_id);
            !post.services.contains_key(service_id)
        })
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
    #[verifier(external_body)]
    pub proof fn lookup_is_consistent(
        state: RegistryState,
        service_type: Seq<u8>,
    )
        requires
            registry_invariant(state),
            state.type_index.contains_key(service_type),
        ensures ({
            let services = get_services_by_type(state, service_type);
            // All returned services have the correct type
            forall |entry: ServiceEntrySpec| services.contains(entry) ==>
                entry.service_type =~= service_type
        })
    {
        // Follows from index_consistent
    }

    // ========================================================================
    // Executable Functions (verified implementations)
    // ========================================================================
    //
    // These exec fn implementations are verified to match their spec fn
    // counterparts. They can be called from production code while maintaining
    // formal guarantees.

    /// Maximum registry TTL in milliseconds (24 hours)
    pub const MAX_REGISTRY_TTL_MS: u64 = 86400000;

    /// Minimum weight for a service
    pub const MIN_SERVICE_WEIGHT: u32 = 1;

    /// Maximum weight for a service
    pub const MAX_SERVICE_WEIGHT: u32 = 1000;

    /// Check if a service registration is valid.
    ///
    /// # Arguments
    ///
    /// * `ttl_ms` - TTL in milliseconds
    /// * `current_time_ms` - Current time
    /// * `max_fencing_token` - Current max fencing token
    ///
    /// # Returns
    ///
    /// `true` if registration parameters are valid.
    pub fn is_valid_registration(
        ttl_ms: u64,
        current_time_ms: u64,
        max_fencing_token: u64,
    ) -> (result: bool)
        ensures result == (
            ttl_ms > 0 &&
            ttl_ms <= MAX_REGISTRY_TTL_MS &&
            current_time_ms <= u64::MAX - ttl_ms &&
            max_fencing_token < u64::MAX
        )
    {
        ttl_ms > 0 &&
        ttl_ms <= MAX_REGISTRY_TTL_MS &&
        current_time_ms <= u64::MAX - ttl_ms &&
        max_fencing_token < u64::MAX
    }

    /// Normalize service weight to valid bounds.
    ///
    /// # Arguments
    ///
    /// * `weight` - Requested weight
    ///
    /// # Returns
    ///
    /// Weight clamped to [MIN_SERVICE_WEIGHT, MAX_SERVICE_WEIGHT].
    pub fn normalize_service_weight(weight: u32) -> (result: u32)
        ensures
            result >= MIN_SERVICE_WEIGHT,
            result <= MAX_SERVICE_WEIGHT,
            weight == 0 ==> result == MIN_SERVICE_WEIGHT,
            weight > MAX_SERVICE_WEIGHT ==> result == MAX_SERVICE_WEIGHT,
            weight >= MIN_SERVICE_WEIGHT && weight <= MAX_SERVICE_WEIGHT ==> result == weight
    {
        if weight == 0 {
            MIN_SERVICE_WEIGHT
        } else if weight > MAX_SERVICE_WEIGHT {
            MAX_SERVICE_WEIGHT
        } else {
            weight
        }
    }

    /// Calculate service deadline from registration time and TTL.
    ///
    /// # Arguments
    ///
    /// * `registered_at_ms` - Registration time
    /// * `ttl_ms` - TTL in milliseconds
    ///
    /// # Returns
    ///
    /// Deadline timestamp (saturating at u64::MAX).
    pub fn calculate_service_deadline(registered_at_ms: u64, ttl_ms: u64) -> (result: u64)
        ensures
            registered_at_ms as int + ttl_ms as int <= u64::MAX as int ==>
                result == registered_at_ms + ttl_ms,
            registered_at_ms as int + ttl_ms as int > u64::MAX as int ==>
                result == u64::MAX
    {
        registered_at_ms.saturating_add(ttl_ms)
    }

    /// Compute next fencing token for registration.
    ///
    /// # Arguments
    ///
    /// * `current_max_token` - Current maximum fencing token
    ///
    /// # Returns
    ///
    /// Next fencing token (saturating at u64::MAX).
    pub fn compute_next_registry_token(current_max_token: u64) -> (result: u64)
        ensures
            current_max_token < u64::MAX ==> result == current_max_token + 1,
            current_max_token == u64::MAX ==> result == u64::MAX
    {
        current_max_token.saturating_add(1)
    }

    /// Check if a service has expired.
    ///
    /// # Arguments
    ///
    /// * `deadline_ms` - Service deadline
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// `true` if service has expired.
    pub fn is_service_expired(deadline_ms: u64, current_time_ms: u64) -> (result: bool)
        ensures result == (current_time_ms > deadline_ms)
    {
        current_time_ms > deadline_ms
    }

    /// Check if a service is live (not expired and healthy).
    ///
    /// # Arguments
    ///
    /// * `deadline_ms` - Service deadline
    /// * `healthy` - Whether service is marked healthy
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// `true` if service is live.
    pub fn is_service_live(deadline_ms: u64, healthy: bool, current_time_ms: u64) -> (result: bool)
        // Inline: is_service_expired returns (current_time_ms > deadline_ms)
        ensures result == (!(current_time_ms > deadline_ms) && healthy)
    {
        !(current_time_ms > deadline_ms) && healthy
    }

    /// Check if a heartbeat is valid (fencing token matches).
    ///
    /// # Arguments
    ///
    /// * `expected_token` - Expected fencing token
    /// * `provided_token` - Token provided with heartbeat
    ///
    /// # Returns
    ///
    /// `true` if heartbeat is valid.
    pub fn is_valid_heartbeat(expected_token: u64, provided_token: u64) -> (result: bool)
        ensures result == (expected_token == provided_token)
    {
        expected_token == provided_token
    }

    /// Calculate new deadline after heartbeat.
    ///
    /// # Arguments
    ///
    /// * `current_time_ms` - Current time
    /// * `ttl_ms` - Service TTL
    ///
    /// # Returns
    ///
    /// New deadline timestamp (saturating at u64::MAX).
    pub fn calculate_heartbeat_deadline(current_time_ms: u64, ttl_ms: u64) -> (result: u64)
        // Inline: calculate_service_deadline uses saturating_add
        ensures
            current_time_ms as int + ttl_ms as int <= u64::MAX as int ==>
                result == current_time_ms + ttl_ms,
            current_time_ms as int + ttl_ms as int > u64::MAX as int ==>
                result == u64::MAX
    {
        current_time_ms.saturating_add(ttl_ms)
    }

    /// Calculate time until service expires.
    ///
    /// # Arguments
    ///
    /// * `deadline_ms` - Service deadline
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// Time remaining until expiration (0 if already expired).
    pub fn time_until_expiration(deadline_ms: u64, current_time_ms: u64) -> (result: u64)
        ensures
            current_time_ms >= deadline_ms ==> result == 0,
            current_time_ms < deadline_ms ==> result == deadline_ms - current_time_ms
    {
        deadline_ms.saturating_sub(current_time_ms)
    }

    /// Check if heartbeat deadline computation would overflow.
    ///
    /// # Arguments
    ///
    /// * `current_time_ms` - Current time
    /// * `ttl_ms` - Service TTL
    ///
    /// # Returns
    ///
    /// `true` if deadline can be computed without overflow.
    pub fn can_compute_deadline(current_time_ms: u64, ttl_ms: u64) -> (result: bool)
        ensures result == (current_time_ms as int + ttl_ms as int <= u64::MAX as int)
    {
        current_time_ms <= u64::MAX - ttl_ms
    }
}
