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
    pub open spec fn register_worker_post(
        pre: WorkerState,
        worker_id: Seq<u8>,
        capacity: u32,
        capabilities: Set<Seq<u8>>,
        lease_duration_ms: u64,
        current_time_ms: u64,
    ) -> WorkerState
        requires
            register_worker_pre(pre, worker_id, capacity),
            // Overflow protection for lease deadline calculation
            current_time_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - lease_duration_ms,
    {
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
        ensures {
            let post = register_worker_post(pre, worker_id, capacity, capabilities, lease_duration_ms, current_time_ms);
            // Worker exists and is active
            post.workers.contains_key(worker_id) &&
            post.workers[worker_id].active &&
            // Load is zero
            post.workers[worker_id].current_load == 0 &&
            // No assigned tasks
            post.workers[worker_id].assigned_tasks.len() == 0
        }
    {
        // Follows from register_worker_post definition
    }

    /// Proof: Register preserves invariant
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
    pub open spec fn heartbeat_post(
        pre: WorkerState,
        worker_id: Seq<u8>,
        lease_duration_ms: u64,
        current_time_ms: u64,
    ) -> WorkerState
        requires
            heartbeat_pre(pre, worker_id),
            // Overflow protection for lease deadline calculation
            current_time_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - lease_duration_ms,
    {
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
    pub proof fn heartbeat_extends_lease(
        pre: WorkerState,
        worker_id: Seq<u8>,
        lease_duration_ms: u64,
        current_time_ms: u64,
    )
        requires
            heartbeat_pre(pre, worker_id),
            lease_duration_ms > 0,
        ensures {
            let post = heartbeat_post(pre, worker_id, lease_duration_ms, current_time_ms);
            post.workers[worker_id].lease_deadline_ms == current_time_ms + lease_duration_ms
        }
    {
        // Directly from heartbeat_post definition
    }

    /// Proof: Heartbeat reactivates worker
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
        ensures {
            let post = heartbeat_post(pre, worker_id, lease_duration_ms, current_time_ms);
            post.workers[worker_id].active
        }
    {
        // active set to true
    }

    /// Proof: Heartbeat preserves worker invariant
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
    pub open spec fn assign_task_post(
        pre: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
        current_time_ms: u64,
    ) -> WorkerState
        requires
            assign_task_pre(pre, task_id, worker_id),
            // Explicit overflow protection: current_load + 1 must not overflow
            // (This is already guaranteed by has_capacity + capacity bounds,
            // but we make it explicit for verification)
            pre.workers[worker_id].current_load < 0xFFFF_FFFFu32,
    {
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
        ensures {
            let post = assign_task_post(pre, task_id, worker_id, current_time_ms);
            post.workers[worker_id].current_load == pre.workers[worker_id].current_load + 1
        }
    {
        // Directly from assign_task_post
    }

    /// Proof: Assign preserves load bounded
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
    pub proof fn assign_removes_from_pending(
        pre: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
        current_time_ms: u64,
    )
        requires assign_task_pre(pre, task_id, worker_id)
        ensures {
            let post = assign_task_post(pre, task_id, worker_id, current_time_ms);
            !post.pending_tasks.contains(task_id)
        }
    {
        // Task removed from pending_tasks
    }

    /// Proof: Assign preserves worker isolation
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
        state.workers[worker_id].assigned_tasks.contains(task_id)
    }

    /// Effect of completing a task
    pub open spec fn complete_task_post(
        pre: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
    ) -> WorkerState
        requires complete_task_pre(pre, task_id, worker_id)
    {
        let old_worker = pre.workers[worker_id];
        let new_worker = WorkerEntrySpec {
            current_load: if old_worker.current_load > 0 {
                (old_worker.current_load - 1) as u32
            } else {
                0
            },
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
    pub proof fn complete_decreases_load(
        pre: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
    )
        requires
            complete_task_pre(pre, task_id, worker_id),
            pre.workers[worker_id].current_load > 0,
        ensures {
            let post = complete_task_post(pre, task_id, worker_id);
            post.workers[worker_id].current_load == pre.workers[worker_id].current_load - 1
        }
    {
        // Directly from complete_task_post
    }

    /// Proof: Complete removes task
    pub proof fn complete_removes_task(
        pre: WorkerState,
        task_id: Seq<u8>,
        worker_id: Seq<u8>,
    )
        requires complete_task_pre(pre, task_id, worker_id)
        ensures {
            let post = complete_task_post(pre, task_id, worker_id);
            !post.tasks.contains_key(task_id) &&
            !post.workers[worker_id].assigned_tasks.contains(task_id)
        }
    {
        // Task removed from both places
    }

    /// Proof: Complete preserves worker invariant
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
    pub open spec fn expire_worker_post(
        pre: WorkerState,
        worker_id: Seq<u8>,
    ) -> WorkerState
        requires
            pre.workers.contains_key(worker_id),
            is_lease_expired(pre.workers[worker_id], pre.current_time_ms),
    {
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
    pub proof fn expire_returns_tasks_to_pending(
        pre: WorkerState,
        worker_id: Seq<u8>,
        task_id: Seq<u8>,
    )
        requires
            pre.workers.contains_key(worker_id),
            is_lease_expired(pre.workers[worker_id], pre.current_time_ms),
            pre.workers[worker_id].assigned_tasks.contains(task_id),
        ensures {
            let post = expire_worker_post(pre, worker_id);
            post.pending_tasks.contains(task_id)
        }
    {
        // task_id in old_worker.assigned_tasks, and new_pending = pre.pending_tasks.union(assigned_tasks)
    }

    /// Proof: Expired tasks have cleared worker assignment
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
        ensures {
            let post = expire_worker_post(pre, worker_id);
            post.tasks.contains_key(task_id) &&
            post.tasks[task_id].worker_id.is_none()
        }
    {
        // clear_task_assignments sets worker_id to None for tasks in expired_task_ids
    }

    /// Proof: Expire marks worker inactive
    pub proof fn expire_marks_inactive(
        pre: WorkerState,
        worker_id: Seq<u8>,
    )
        requires
            pre.workers.contains_key(worker_id),
            is_lease_expired(pre.workers[worker_id], pre.current_time_ms),
        ensures {
            let post = expire_worker_post(pre, worker_id);
            !post.workers[worker_id].active
        }
    {
        // active set to false
    }

    /// Proof: Expire frees capacity
    pub proof fn expire_frees_capacity(
        pre: WorkerState,
        worker_id: Seq<u8>,
    )
        requires
            pre.workers.contains_key(worker_id),
            is_lease_expired(pre.workers[worker_id], pre.current_time_ms),
        ensures {
            let post = expire_worker_post(pre, worker_id);
            post.workers[worker_id].current_load == 0
        }
    {
        // Load reset to 0
    }
}
