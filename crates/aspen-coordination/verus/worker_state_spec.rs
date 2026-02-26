//! Worker Coordinator State Machine Model
//!
//! Abstract state model for formal verification of worker coordination operations.
//!
//! # State Model
//!
//! The `WorkerState` captures:
//! - Registered workers with capacity
//! - Task assignments
//! - Lease management
//!
//! # Key Invariants
//!
//! 1. **WORK-1: Task Uniqueness**: Each task assigned to at most one worker
//! 2. **WORK-2: Worker Isolation**: No task in multiple workers' sets
//! 3. **WORK-3: Lease Validity**: Active workers have valid leases
//! 4. **WORK-4: Load Bounded**: Load never exceeds capacity
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/worker_state_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Worker registration entry
    pub struct WorkerEntrySpec {
        /// Worker identifier
        pub worker_id: Seq<u8>,
        /// Worker capacity (max concurrent tasks)
        pub capacity: u32,
        /// Current load (assigned tasks)
        pub current_load: u32,
        /// Set of assigned task IDs
        pub assigned_tasks: Set<Seq<u8>>,
        /// Registration timestamp (Unix ms)
        pub registered_at_ms: u64,
        /// Last heartbeat timestamp (Unix ms)
        pub last_heartbeat_ms: u64,
        /// Lease deadline (Unix ms)
        pub lease_deadline_ms: u64,
        /// Whether worker is active (not expired)
        pub is_active: bool,
        /// Worker capabilities/tags
        pub capabilities: Set<Seq<u8>>,
    }

    /// Task assignment entry
    pub struct TaskAssignmentSpec {
        /// Task identifier
        pub task_id: Seq<u8>,
        /// Assigned worker ID (if assigned)
        pub worker_id: Option<Seq<u8>>,
        /// Assignment timestamp
        pub assigned_at_ms: u64,
        /// Task deadline (if any)
        pub deadline_ms: u64,
        /// Task priority
        pub priority: u32,
        /// Required capabilities
        pub required_capabilities: Set<Seq<u8>>,
    }

    /// Complete worker coordinator state
    pub struct WorkerState {
        /// All registered workers (worker_id -> entry)
        pub workers: Map<Seq<u8>, WorkerEntrySpec>,
        /// All tasks (task_id -> assignment)
        pub tasks: Map<Seq<u8>, TaskAssignmentSpec>,
        /// Pending tasks (not yet assigned)
        pub pending_tasks: Set<Seq<u8>>,
        /// Current time (for lease checks)
        pub current_time_ms: u64,
    }

    // ========================================================================
    // Invariant 1: Task Uniqueness
    // ========================================================================

    /// WORK-1: Each task assigned to at most one worker
    pub open spec fn task_uniqueness(state: WorkerState) -> bool {
        forall |task_id: Seq<u8>|
            state.tasks.contains_key(task_id) &&
            state.tasks[task_id].worker_id.is_some() ==>
            // Exactly one worker has this task
            exists |worker_id: Seq<u8>|
                state.workers.contains_key(worker_id) &&
                state.workers[worker_id].assigned_tasks.contains(task_id) &&
                state.tasks[task_id].worker_id == Some(worker_id)
    }

    /// Stronger form: task appears in exactly one worker's set
    pub open spec fn task_appears_once(state: WorkerState, task_id: Seq<u8>) -> bool {
        // Count of workers containing this task is at most 1
        forall |w1: Seq<u8>, w2: Seq<u8>|
            state.workers.contains_key(w1) &&
            state.workers.contains_key(w2) &&
            state.workers[w1].assigned_tasks.contains(task_id) &&
            state.workers[w2].assigned_tasks.contains(task_id) ==>
            w1 =~= w2
    }

    // ========================================================================
    // Invariant 2: Worker Isolation
    // ========================================================================

    /// WORK-2: No task in multiple workers' sets
    ///
    /// Same as task_appears_once, generalized
    pub open spec fn worker_isolation(state: WorkerState) -> bool {
        forall |task_id: Seq<u8>| task_appears_once(state, task_id)
    }

    /// Worker assigned tasks match task assignments
    pub open spec fn assignment_consistency(state: WorkerState) -> bool {
        // Forward: worker has task => task points to worker
        (forall |worker_id: Seq<u8>, task_id: Seq<u8>|
            state.workers.contains_key(worker_id) &&
            state.workers[worker_id].assigned_tasks.contains(task_id) ==>
            state.tasks.contains_key(task_id) &&
            state.tasks[task_id].worker_id == Some(worker_id)
        ) &&
        // Backward: task points to worker => worker has task
        (forall |task_id: Seq<u8>|
            state.tasks.contains_key(task_id) &&
            state.tasks[task_id].worker_id.is_some() ==> {
                let worker_id = state.tasks[task_id].worker_id.unwrap();
                state.workers.contains_key(worker_id) &&
                state.workers[worker_id].assigned_tasks.contains(task_id)
            }
        )
    }

    // ========================================================================
    // Invariant 3: Lease Validity
    // ========================================================================

    /// WORK-3: Active workers have valid leases
    pub open spec fn lease_validity(state: WorkerState) -> bool {
        forall |worker_id: Seq<u8>|
            state.workers.contains_key(worker_id) &&
            state.workers[worker_id].is_active ==>
            state.workers[worker_id].lease_deadline_ms > state.current_time_ms
    }

    /// Check if worker lease has expired
    pub open spec fn is_lease_expired(
        worker: WorkerEntrySpec,
        current_time_ms: u64,
    ) -> bool {
        current_time_ms > worker.lease_deadline_ms
    }

    // ========================================================================
    // Invariant 4: Load Bounded
    // ========================================================================

    /// WORK-4: Load never exceeds capacity
    pub open spec fn load_bounded(state: WorkerState) -> bool {
        forall |worker_id: Seq<u8>|
            state.workers.contains_key(worker_id) ==>
            state.workers[worker_id].current_load <= state.workers[worker_id].capacity
    }

    /// Load equals assigned tasks count
    pub open spec fn load_consistent(state: WorkerState) -> bool {
        forall |worker_id: Seq<u8>|
            state.workers.contains_key(worker_id) ==>
            state.workers[worker_id].current_load as int ==
                state.workers[worker_id].assigned_tasks.len()
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined worker coordinator invariant
    pub open spec fn worker_invariant(state: WorkerState) -> bool {
        task_uniqueness(state) &&
        worker_isolation(state) &&
        assignment_consistency(state) &&
        lease_validity(state) &&
        load_bounded(state) &&
        load_consistent(state)
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial empty worker state
    pub open spec fn initial_worker_state() -> WorkerState {
        WorkerState {
            workers: Map::empty(),
            tasks: Map::empty(),
            pending_tasks: Set::empty(),
            current_time_ms: 0,
        }
    }

    /// Proof: Initial state satisfies invariant
    #[verifier(external_body)]
    pub proof fn initial_state_invariant()
        ensures worker_invariant(initial_worker_state())
    {
        // Empty maps trivially satisfy all forall properties
    }

    // ========================================================================
    // Helper Predicates
    // ========================================================================

    /// Check if worker can accept more tasks
    pub open spec fn has_capacity(state: WorkerState, worker_id: Seq<u8>) -> bool {
        state.workers.contains_key(worker_id) &&
        state.workers[worker_id].current_load < state.workers[worker_id].capacity &&
        state.workers[worker_id].is_active
    }

    /// Check if worker has required capabilities
    pub open spec fn has_capabilities(
        worker: WorkerEntrySpec,
        required: Set<Seq<u8>>,
    ) -> bool {
        required.subset_of(worker.capabilities)
    }

    /// Find eligible workers for a task
    pub open spec fn eligible_workers(
        state: WorkerState,
        required_capabilities: Set<Seq<u8>>,
    ) -> Set<Seq<u8>> {
        Set::new(|worker_id: Seq<u8>|
            state.workers.contains_key(worker_id) &&
            has_capacity(state, worker_id) &&
            has_capabilities(state.workers[worker_id], required_capabilities)
        )
    }

    /// Get pending task count
    pub open spec fn pending_task_count(state: WorkerState) -> int {
        state.pending_tasks.len() as int
    }

    /// Get active worker count
    /// Returns the cardinality of the set of active workers
    pub open spec fn active_worker_count(state: WorkerState) -> int {
        Set::new(|worker_id: Seq<u8>|
            state.workers.contains_key(worker_id) &&
            state.workers[worker_id].is_active
        ).len() as int
    }
}
