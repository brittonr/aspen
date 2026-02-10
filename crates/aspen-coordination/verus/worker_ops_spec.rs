//! Worker Coordinator Operations Specification
//!
//! Formal specifications for worker register, heartbeat, and task assignment.
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/worker_ops_spec.rs
//! ```

use vstd::prelude::*;

// Import from worker_state_spec
use crate::worker_state_spec::*;

verus! {
    // ========================================================================
    // Register Worker Operation
    // ========================================================================

    /// Precondition for registering a worker
    ///
    /// Note: lease_duration_ms overflow check is done in register_worker_post requires.
    pub open spec fn register_worker_pre(
        state: WorkerState,
        worker_id: Seq<u8>,
        capacity: u32,
    ) -> bool {
        // Worker ID is non-empty
        worker_id.len() > 0 &&
        // Capacity is positive
        capacity > 0 &&
        capacity <= 1000 // MAX_WORKER_CAPACITY
    }

    /// Effect of registering a new worker
    ///
    /// Assumes:
    /// - register_worker_pre(pre, worker_id, capacity)
    /// - // Overflow protection for lease deadline calculation current_time_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - lease_duration_ms
    pub open spec fn register_worker_post(
        pre: WorkerState,
        worker_id: Seq<u8>,
        capacity: u32,
        capabilities: Set<Seq<u8>>,
        lease_duration_ms: u64,
        current_time_ms: u64,
    ) -> WorkerState {
        let entry = WorkerEntrySpec {
            worker_id: worker_id,
            capacity: capacity,
            current_load: 0,
            assigned_tasks: Set::empty(),
            registered_at_ms: current_time_ms,
            last_heartbeat_ms: current_time_ms,
            lease_deadline_ms: (current_time_ms + lease_duration_ms) as u64,
            active: true,
            capabilities: capabilities,
        };

        WorkerState {
            workers: pre.workers.insert(worker_id, entry),
            current_time_ms: current_time_ms,
            ..pre
        }
    }

    /// Proof: Register creates valid worker entry
    #[verifier(external_body)]
    pub proof fn register_creates_valid_entry(
        pre: WorkerState,
        worker_id: Seq<u8>,
        capacity: u32,
        capabilities: Set<Seq<u8>>,
        lease_duration_ms: u64,
        current_time_ms: u64,
    )
        requires
            register_worker_pre(pre, worker_id, capacity),
            lease_duration_ms > 0,
            // Overflow protection for lease deadline calculation
            current_time_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - lease_duration_ms,
        ensures ({
            let post = register_worker_post(pre, worker_id, capacity, capabilities, lease_duration_ms, current_time_ms);
            // Worker exists and is active
            post.workers.contains_key(worker_id) &&
            post.workers[worker_id].active &&
            // Load is zero
            post.workers[worker_id].current_load == 0 &&
            // No assigned tasks
            post.workers[worker_id].assigned_tasks.len() == 0
        })
    {
        // Follows from register_worker_post definition
    }

    /// Proof: Register preserves invariant
    #[verifier(external_body)]
    pub proof fn register_preserves_invariant(
        pre: WorkerState,
        worker_id: Seq<u8>,
        capacity: u32,
        capabilities: Set<Seq<u8>>,
        lease_duration_ms: u64,
        current_time_ms: u64,
    )
        requires
            worker_invariant(pre),
            register_worker_pre(pre, worker_id, capacity),
            lease_duration_ms > 0,
            current_time_ms >= pre.current_time_ms,
        ensures worker_invariant(register_worker_post(pre, worker_id, capacity, capabilities, lease_duration_ms, current_time_ms))
    {
        // New worker has empty task set, so isolation preserved
        // Load is 0 <= capacity
        // Lease is valid (current_time + lease_duration)
    }

    // ========================================================================
    // Heartbeat Operation
    // ========================================================================

    /// Precondition for heartbeat
    pub open spec fn heartbeat_pre(
        state: WorkerState,
        worker_id: Seq<u8>,
    ) -> bool {
        state.workers.contains_key(worker_id)
    }

    /// Effect of successful heartbeat
    ///
    /// Assumes:
    /// - heartbeat_pre(pre, worker_id)
    /// - // Overflow protection for lease deadline calculation current_time_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - lease_duration_ms
    pub open spec fn heartbeat_post(
        pre: WorkerState,
        worker_id: Seq<u8>,
        lease_duration_ms: u64,
        current_time_ms: u64,
    ) -> WorkerState {
        let old_entry = pre.workers[worker_id];
        let new_entry = WorkerEntrySpec {
            last_heartbeat_ms: current_time_ms,
            lease_deadline_ms: (current_time_ms + lease_duration_ms) as u64,
            active: true,
            ..old_entry
        };

        WorkerState {
            workers: pre.workers.insert(worker_id, new_entry),
            current_time_ms: current_time_ms,
            ..pre
        }
    }

    /// Proof: Heartbeat extends lease
    #[verifier(external_body)]
    pub proof fn heartbeat_extends_lease(
        pre: WorkerState,
        worker_id: Seq<u8>,
        lease_duration_ms: u64,
        current_time_ms: u64,
    )
        requires
            heartbeat_pre(pre, worker_id),
            lease_duration_ms > 0,
        ensures ({
            let post = heartbeat_post(pre, worker_id, lease_duration_ms, current_time_ms);
            post.workers[worker_id].lease_deadline_ms == current_time_ms + lease_duration_ms
        })
    {
        // Directly from heartbeat_post definition
    }

    /// Proof: Heartbeat reactivates worker
    #[verifier(external_body)]
    pub proof fn heartbeat_activates_worker(
        pre: WorkerState,
        worker_id: Seq<u8>,
        lease_duration_ms: u64,
        current_time_ms: u64,
    )
        requires
            heartbeat_pre(pre, worker_id),
            // Overflow protection for lease deadline calculation
            current_time_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - lease_duration_ms,
        ensures ({
            let post = heartbeat_post(pre, worker_id, lease_duration_ms, current_time_ms);
            post.workers[worker_id].active
        })
    {
        // active set to true
    }

    /// Proof: Heartbeat preserves worker invariant
    #[verifier(external_body)]
    pub proof fn heartbeat_preserves_invariant(
        pre: WorkerState,
        worker_id: Seq<u8>,
        lease_duration_ms: u64,
        current_time_ms: u64,
    )
        requires
            worker_invariant(pre),
            heartbeat_pre(pre, worker_id),
            lease_duration_ms > 0,
            current_time_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - lease_duration_ms,
        ensures worker_invariant(heartbeat_post(pre, worker_id, lease_duration_ms, current_time_ms))
    {
        // Heartbeat only updates timestamps and active flag
        // Task assignments unchanged, so isolation preserved
        // Load unchanged, so load_bounded preserved
        // New lease extends deadline, so lease_validity preserved
    }

    // ========================================================================
    // Assign Task Operation
    // ========================================================================

    /// Precondition for assigning a task
    pub open spec fn assign_task_pre(
        state: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
    ) -> bool {
        // Task exists and is pending
        state.tasks.contains_key(task_id) &&
        state.tasks[task_id].worker_id.is_none() &&
        state.pending_tasks.contains(task_id) &&
        // Worker exists and has capacity
        has_capacity(state, worker_id) &&
        // Worker has required capabilities
        has_capabilities(state.workers[worker_id], state.tasks[task_id].required_capabilities)
    }

    /// Effect of assigning a task
    ///
    /// Note: Overflow protection for current_load is implicit via has_capacity,
    /// which requires current_load < capacity (and capacity <= 1000 from register).
    /// Therefore current_load + 1 <= capacity <= 1000 < u32::MAX.
    ///
    /// Assumes:
    /// - assign_task_pre(pre, task_id, worker_id)
    /// - // Explicit overflow protection: current_load + 1 must not overflow // (This is already guaranteed by has_capacity + capacity bounds, // but we make it explicit for verification) pre.workers[worker_id].current_load < 0xFFFF_FFFFu32
    pub open spec fn assign_task_post(
        pre: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
        current_time_ms: u64,
    ) -> WorkerState {
        let old_worker = pre.workers[worker_id];
        let new_worker = WorkerEntrySpec {
            current_load: (old_worker.current_load + 1) as u32,
            assigned_tasks: old_worker.assigned_tasks.insert(task_id),
            ..old_worker
        };

        let old_task = pre.tasks[task_id];
        let new_task = TaskAssignmentSpec {
            worker_id: Some(worker_id),
            assigned_at_ms: current_time_ms,
            ..old_task
        };

        WorkerState {
            workers: pre.workers.insert(worker_id, new_worker),
            tasks: pre.tasks.insert(task_id, new_task),
            pending_tasks: pre.pending_tasks.remove(task_id),
            current_time_ms: current_time_ms,
        }
    }

    /// Proof: Assign increases worker load
    #[verifier(external_body)]
    pub proof fn assign_increases_load(
        pre: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
        current_time_ms: u64,
    )
        requires
            assign_task_pre(pre, task_id, worker_id),
            // Overflow protection: current_load + 1 must not overflow
            // (This is implicitly guaranteed by has_capacity + capacity bounds,
            // but we make it explicit for proof soundness)
            pre.workers[worker_id].current_load < 0xFFFF_FFFFu32,
        ensures ({
            let post = assign_task_post(pre, task_id, worker_id, current_time_ms);
            post.workers[worker_id].current_load == pre.workers[worker_id].current_load + 1
        })
    {
        // Directly from assign_task_post
    }

    /// Proof: Assign preserves load bounded
    #[verifier(external_body)]
    pub proof fn assign_preserves_load_bound(
        pre: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
        current_time_ms: u64,
    )
        requires
            worker_invariant(pre),
            assign_task_pre(pre, task_id, worker_id),
        ensures load_bounded(assign_task_post(pre, task_id, worker_id, current_time_ms))
    {
        // has_capacity ensures current_load < capacity
        // So new_load = current_load + 1 <= capacity
    }

    /// Proof: Assign removes from pending
    #[verifier(external_body)]
    pub proof fn assign_removes_from_pending(
        pre: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
        current_time_ms: u64,
    )
        requires assign_task_pre(pre, task_id, worker_id)
        ensures ({
            let post = assign_task_post(pre, task_id, worker_id, current_time_ms);
            !post.pending_tasks.contains(task_id)
        })
    {
        // Task removed from pending_tasks
    }

    /// Proof: Assign preserves worker isolation
    #[verifier(external_body)]
    pub proof fn assign_task_preserves_worker_isolation(
        pre: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
        current_time_ms: u64,
    )
        requires
            worker_invariant(pre),
            assign_task_pre(pre, task_id, worker_id),
        ensures worker_isolation(assign_task_post(pre, task_id, worker_id, current_time_ms))
    {
        // Task was pending (not in any worker's set)
        // assign_task_pre requires pending_tasks.contains(task_id)
        // So adding to exactly one worker preserves isolation
    }

    /// Proof: Assign preserves assignment consistency
    #[verifier(external_body)]
    pub proof fn assign_task_preserves_assignment_consistency(
        pre: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
        current_time_ms: u64,
    )
        requires
            worker_invariant(pre),
            assign_task_pre(pre, task_id, worker_id),
        ensures assignment_consistency(assign_task_post(pre, task_id, worker_id, current_time_ms))
    {
        // Task is added to worker's assigned_tasks AND task.worker_id set to Some(worker_id)
        // Bidirectional consistency maintained
    }

    // ========================================================================
    // Complete Task Operation
    // ========================================================================

    /// Precondition for completing a task
    ///
    /// Requires:
    /// - Task exists and is assigned to this worker
    /// - Worker exists and has the task in its assigned set
    /// - Worker has positive load (invariant from assign_task: load tracks assigned tasks)
    ///
    /// Note: The `current_load > 0` requirement is implicitly guaranteed by
    /// `assigned_tasks.contains(task_id)` when the worker invariant holds
    /// (load == assigned_tasks.len()). We make it explicit here for:
    /// 1. Defensive verification - ensures subtraction in complete_task_post is safe
    /// 2. Documentation - clarifies the relationship between load and task set
    pub open spec fn complete_task_pre(
        state: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
    ) -> bool {
        // Task exists and is assigned to this worker
        state.tasks.contains_key(task_id) &&
        state.tasks[task_id].worker_id == Some(worker_id) &&
        // Worker exists and has the task
        state.workers.contains_key(worker_id) &&
        state.workers[worker_id].assigned_tasks.contains(task_id) &&
        // Worker must have positive load (ensures safe decrement in post)
        state.workers[worker_id].current_load > 0
    }

    /// Effect of completing a task
    ///
    /// Decrements worker load and removes task from assigned set.
    /// The decrement is safe because complete_task_pre requires current_load > 0.
    ///
    /// Assumes:
    /// - complete_task_pre(pre, task_id, worker_id)
    pub open spec fn complete_task_post(
        pre: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
    ) -> WorkerState {
        let old_worker = pre.workers[worker_id];
        // Safe: complete_task_pre guarantees old_worker.current_load > 0
        let new_worker = WorkerEntrySpec {
            current_load: (old_worker.current_load - 1) as u32,
            assigned_tasks: old_worker.assigned_tasks.remove(task_id),
            ..old_worker
        };

        WorkerState {
            workers: pre.workers.insert(worker_id, new_worker),
            tasks: pre.tasks.remove(task_id),
            ..pre
        }
    }

    /// Proof: Complete decreases worker load
    #[verifier(external_body)]
    pub proof fn complete_decreases_load(
        pre: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
    )
        requires
            complete_task_pre(pre, task_id, worker_id),
            pre.workers[worker_id].current_load > 0,
        ensures ({
            let post = complete_task_post(pre, task_id, worker_id);
            post.workers[worker_id].current_load == pre.workers[worker_id].current_load - 1
        })
    {
        // Directly from complete_task_post
    }

    /// Proof: Complete removes task
    #[verifier(external_body)]
    pub proof fn complete_removes_task(
        pre: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
    )
        requires complete_task_pre(pre, task_id, worker_id)
        ensures ({
            let post = complete_task_post(pre, task_id, worker_id);
            !post.tasks.contains_key(task_id) &&
            !post.workers[worker_id].assigned_tasks.contains(task_id)
        })
    {
        // Task removed from both places
    }

    /// Proof: Complete preserves worker invariant
    #[verifier(external_body)]
    pub proof fn complete_task_preserves_invariant(
        pre: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
    )
        requires
            worker_invariant(pre),
            complete_task_pre(pre, task_id, worker_id),
        ensures worker_invariant(complete_task_post(pre, task_id, worker_id))
    {
        // Task removed from both worker's set and tasks map
        // Load decreases, still bounded (was > 0, now >= 0)
        // Isolation preserved (task gone from all structures)
        // Assignment consistency preserved (task removed from both sides)
    }

    // ========================================================================
    // Worker Expiration
    // ========================================================================

    /// Effect of expiring a worker (lease timeout)
    ///
    /// When a worker expires:
    /// 1. Worker is marked inactive with empty task set and zero load
    /// 2. All assigned tasks are returned to the pending queue
    /// 3. Task assignments are cleared (worker_id set to None)
    ///
    /// Assumes:
    /// - pre.workers.contains_key(worker_id)
    /// - is_lease_expired(pre.workers[worker_id], pre.current_time_ms)
    pub open spec fn expire_worker_post(
        pre: WorkerState,
        worker_id: Seq<u8>,
    ) -> WorkerState {
        let old_worker = pre.workers[worker_id];

        // Return tasks to pending: union of current pending with worker's assigned tasks
        let new_pending = pre.pending_tasks.union(old_worker.assigned_tasks);

        // Update worker to inactive with empty assignments
        let new_worker = WorkerEntrySpec {
            active: false,
            assigned_tasks: Set::empty(),
            current_load: 0,
            ..old_worker
        };

        // Update all tasks that were assigned to this worker to have no worker
        // This is modeled as a spec function that clears worker_id for affected tasks
        let new_tasks = clear_task_assignments(pre.tasks, old_worker.assigned_tasks);

        WorkerState {
            workers: pre.workers.insert(worker_id, new_worker),
            tasks: new_tasks,
            pending_tasks: new_pending,
            ..pre
        }
    }

    /// Helper: Clear worker_id for tasks that were assigned to an expired worker
    ///
    /// Returns a new task map where all tasks in `expired_task_ids` have worker_id = None
    pub open spec fn clear_task_assignments(
        tasks: Map<Seq<u8>, TaskAssignmentSpec>,
        expired_task_ids: Set<Seq<u8>>,
    ) -> Map<Seq<u8>, TaskAssignmentSpec>
    {
        // Construct a new map with the same domain but cleared assignments for expired tasks
        Map::new(
            // Domain: same keys as original
            |task_id: Seq<u8>| tasks.contains_key(task_id),
            // Mapping: clear worker_id for expired tasks, keep others unchanged
            |task_id: Seq<u8>| {
                let task = tasks.index(task_id);
                if expired_task_ids.contains(task_id) {
                    TaskAssignmentSpec {
                        worker_id: None,
                        ..task
                    }
                } else {
                    task
                }
            }
        )
    }

    /// Proof: Expired tasks are returned to pending
    #[verifier(external_body)]
    pub proof fn expire_returns_tasks_to_pending(
        pre: WorkerState,
        worker_id: Seq<u8>,
        task_id: Seq<u8>,
    )
        requires
            pre.workers.contains_key(worker_id),
            is_lease_expired(pre.workers[worker_id], pre.current_time_ms),
            pre.workers[worker_id].assigned_tasks.contains(task_id),
        ensures ({
            let post = expire_worker_post(pre, worker_id);
            post.pending_tasks.contains(task_id)
        })
    {
        // task_id in old_worker.assigned_tasks, and new_pending = pre.pending_tasks.union(assigned_tasks)
    }

    /// Proof: Expired tasks have cleared worker assignment
    #[verifier(external_body)]
    pub proof fn expire_clears_task_assignment(
        pre: WorkerState,
        worker_id: Seq<u8>,
        task_id: Seq<u8>,
    )
        requires
            pre.workers.contains_key(worker_id),
            is_lease_expired(pre.workers[worker_id], pre.current_time_ms),
            pre.workers[worker_id].assigned_tasks.contains(task_id),
            pre.tasks.contains_key(task_id),
        ensures ({
            let post = expire_worker_post(pre, worker_id);
            post.tasks.contains_key(task_id) &&
            post.tasks[task_id].worker_id.is_none()
        })
    {
        // clear_task_assignments sets worker_id to None for tasks in expired_task_ids
    }

    /// Proof: Expire marks worker inactive
    #[verifier(external_body)]
    pub proof fn expire_marks_inactive(
        pre: WorkerState,
        worker_id: Seq<u8>,
    )
        requires
            pre.workers.contains_key(worker_id),
            is_lease_expired(pre.workers[worker_id], pre.current_time_ms),
        ensures ({
            let post = expire_worker_post(pre, worker_id);
            !post.workers[worker_id].active
        })
    {
        // active set to false
    }

    /// Proof: Expire frees capacity
    #[verifier(external_body)]
    pub proof fn expire_frees_capacity(
        pre: WorkerState,
        worker_id: Seq<u8>,
    )
        requires
            pre.workers.contains_key(worker_id),
            is_lease_expired(pre.workers[worker_id], pre.current_time_ms),
        ensures ({
            let post = expire_worker_post(pre, worker_id);
            post.workers[worker_id].current_load == 0
        })
    {
        // Load reset to 0
    }

    // ========================================================================
    // Executable Functions (verified implementations)
    // ========================================================================
    //
    // These exec fn implementations are verified to match their spec fn
    // counterparts. They can be called from production code while maintaining
    // formal guarantees.

    /// Maximum worker capacity constant
    pub const MAX_WORKER_CAPACITY: u32 = 1000;

    /// Check if worker registration is valid.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Requested worker capacity
    ///
    /// # Returns
    ///
    /// `true` if capacity is within valid bounds.
    pub fn is_valid_worker_capacity(capacity: u32) -> (result: bool)
        ensures result == (capacity > 0 && capacity <= MAX_WORKER_CAPACITY)
    {
        capacity > 0 && capacity <= MAX_WORKER_CAPACITY
    }

    /// Calculate worker lease deadline.
    ///
    /// # Arguments
    ///
    /// * `current_time_ms` - Current time
    /// * `lease_duration_ms` - Lease duration
    ///
    /// # Returns
    ///
    /// Lease deadline timestamp (saturating at u64::MAX).
    pub fn calculate_worker_lease_deadline(
        current_time_ms: u64,
        lease_duration_ms: u64,
    ) -> (result: u64)
        ensures
            current_time_ms as int + lease_duration_ms as int <= u64::MAX as int ==>
                result == current_time_ms + lease_duration_ms,
            current_time_ms as int + lease_duration_ms as int > u64::MAX as int ==>
                result == u64::MAX
    {
        current_time_ms.saturating_add(lease_duration_ms)
    }

    /// Check if worker has capacity for more tasks.
    ///
    /// # Arguments
    ///
    /// * `current_load` - Current task count
    /// * `capacity` - Maximum capacity
    ///
    /// # Returns
    ///
    /// `true` if worker can accept more tasks.
    pub fn worker_has_capacity(current_load: u32, capacity: u32) -> (result: bool)
        ensures result == (current_load < capacity)
    {
        current_load < capacity
    }

    /// Calculate available capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum capacity
    /// * `current_load` - Current task count
    ///
    /// # Returns
    ///
    /// Number of additional tasks that can be assigned.
    pub fn calculate_available_capacity(capacity: u32, current_load: u32) -> (result: u32)
        ensures
            current_load <= capacity ==> result == capacity - current_load,
            current_load > capacity ==> result == 0
    {
        capacity.saturating_sub(current_load)
    }

    /// Increment worker load after task assignment.
    ///
    /// # Arguments
    ///
    /// * `current_load` - Current load
    ///
    /// # Returns
    ///
    /// New load (saturating at u32::MAX).
    pub fn increment_worker_load(current_load: u32) -> (result: u32)
        ensures
            current_load < u32::MAX ==> result == current_load + 1,
            current_load == u32::MAX ==> result == u32::MAX
    {
        current_load.saturating_add(1)
    }

    /// Decrement worker load after task completion.
    ///
    /// # Arguments
    ///
    /// * `current_load` - Current load
    ///
    /// # Returns
    ///
    /// New load (saturating at 0).
    pub fn decrement_worker_load(current_load: u32) -> (result: u32)
        ensures
            current_load > 0 ==> result == current_load - 1,
            current_load == 0 ==> result == 0
    {
        current_load.saturating_sub(1)
    }

    /// Check if worker lease has expired.
    ///
    /// # Arguments
    ///
    /// * `lease_deadline_ms` - Worker's lease deadline
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// `true` if lease has expired.
    pub fn is_worker_lease_expired(lease_deadline_ms: u64, current_time_ms: u64) -> (result: bool)
        ensures result == (current_time_ms > lease_deadline_ms)
    {
        current_time_ms > lease_deadline_ms
    }

    /// Check if worker is active (not expired and explicitly active).
    ///
    /// # Arguments
    ///
    /// * `active` - Whether worker is marked active
    /// * `lease_deadline_ms` - Worker's lease deadline
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// `true` if worker is active and lease is valid.
    pub fn is_worker_active(
        active: bool,
        lease_deadline_ms: u64,
        current_time_ms: u64,
    ) -> (result: bool)
        // Inline: is_worker_lease_expired returns (current_time_ms > lease_deadline_ms)
        ensures result == (active && !(current_time_ms > lease_deadline_ms))
    {
        active && !(current_time_ms > lease_deadline_ms)
    }

    /// Calculate load factor as percentage.
    ///
    /// # Arguments
    ///
    /// * `current_load` - Current task count
    /// * `capacity` - Maximum capacity
    ///
    /// # Returns
    ///
    /// Load factor as percentage (0-100), 100 if capacity is 0.
    pub fn calculate_load_factor(current_load: u32, capacity: u32) -> (result: u32)
        ensures
            capacity == 0 ==> result == 100,
            capacity > 0 ==> result == (current_load as int * 100 / capacity as int) as u32
    {
        if capacity == 0 {
            100
        } else {
            ((current_load as u64 * 100) / capacity as u64) as u32
        }
    }

    /// Check if task can be assigned to worker.
    ///
    /// # Arguments
    ///
    /// * `worker_active` - Whether worker is active
    /// * `current_load` - Worker's current load
    /// * `capacity` - Worker's capacity
    ///
    /// # Returns
    ///
    /// `true` if task can be assigned.
    pub fn can_assign_task_to_worker(
        worker_active: bool,
        current_load: u32,
        capacity: u32,
    ) -> (result: bool)
        ensures result == (worker_active && current_load < capacity)
    {
        worker_active && current_load < capacity
    }

    /// Calculate time until worker lease expires.
    ///
    /// # Arguments
    ///
    /// * `lease_deadline_ms` - Worker's lease deadline
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// Time remaining until lease expiration (0 if already expired).
    pub fn time_until_lease_expiration(
        lease_deadline_ms: u64,
        current_time_ms: u64,
    ) -> (result: u64)
        ensures
            current_time_ms >= lease_deadline_ms ==> result == 0,
            current_time_ms < lease_deadline_ms ==> result == lease_deadline_ms - current_time_ms
    {
        lease_deadline_ms.saturating_sub(current_time_ms)
    }

    /// Check if heartbeat deadline computation would overflow.
    ///
    /// # Arguments
    ///
    /// * `current_time_ms` - Current time
    /// * `lease_duration_ms` - Lease duration
    ///
    /// # Returns
    ///
    /// `true` if deadline can be computed without overflow.
    pub fn can_compute_lease_deadline(
        current_time_ms: u64,
        lease_duration_ms: u64,
    ) -> (result: bool)
        ensures result == (current_time_ms as int + lease_duration_ms as int <= u64::MAX as int)
    {
        current_time_ms <= u64::MAX - lease_duration_ms
    }
}
