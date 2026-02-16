//! Job dependency tracking and resolution.

use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::error::JobError;
use crate::error::Result;
use crate::job::JobId;

/// Maximum depth allowed for dependency chains (Tiger Style bound).
/// Prevents unbounded DAG depth which could cause stack overflow during traversal.
const MAX_DEPENDENCY_DEPTH: u32 = 100;

/// Maximum number of nodes allowed in the dependency graph (Tiger Style bound).
const MAX_GRAPH_NODES: usize = 100_000;

/// State of a job's dependencies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DependencyState {
    /// Waiting on specified dependencies.
    Waiting(Vec<JobId>),
    /// All dependencies satisfied, ready to run.
    Ready,
    /// Currently executing.
    Running,
    /// Successfully completed.
    Completed,
    /// Failed with error.
    Failed(String),
    /// Blocked by circular dependency.
    Blocked(String),
}

impl DependencyState {
    /// Check if job can execute.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Check if in terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed(_) | Self::Blocked(_))
    }
}

/// Policy for handling dependency failures.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum DependencyFailurePolicy {
    /// Fail all dependent jobs immediately.
    #[default]
    FailCascade,
    /// Continue with partial dependencies.
    ContinuePartial,
    /// Wait for dependency retry.
    WaitForRetry,
    /// Skip failed dependency.
    SkipFailed,
}

/// Job dependency information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDependencyInfo {
    /// Job ID.
    pub job_id: JobId,
    /// Jobs this job depends on.
    pub dependencies: HashSet<JobId>,
    /// Jobs that depend on this job.
    pub dependents: HashSet<JobId>,
    /// Current dependency state.
    pub state: DependencyState,
    /// Failure policy.
    pub failure_policy: DependencyFailurePolicy,
    /// Last dependency check time.
    pub last_check: Option<DateTime<Utc>>,
    /// Depth in dependency tree.
    pub depth: u32,
}

/// Dependency graph for job execution ordering.
pub struct DependencyGraph {
    /// Job dependency information.
    nodes: Arc<RwLock<HashMap<JobId, JobDependencyInfo>>>,
    /// Adjacency list for forward dependencies (job -> dependencies).
    edges_forward: Arc<RwLock<HashMap<JobId, HashSet<JobId>>>>,
    /// Adjacency list for reverse dependencies (job -> dependents).
    edges_reverse: Arc<RwLock<HashMap<JobId, HashSet<JobId>>>>,
    /// Jobs ready for execution.
    ready_queue: Arc<RwLock<VecDeque<JobId>>>,
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyGraph {
    /// Create a new dependency graph.
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            edges_forward: Arc::new(RwLock::new(HashMap::new())),
            edges_reverse: Arc::new(RwLock::new(HashMap::new())),
            ready_queue: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Calculate dependency depth and validate it doesn't exceed limit.
    fn add_job_calculate_depth(
        nodes: &HashMap<JobId, JobDependencyInfo>,
        dependencies: &[JobId],
        job_id: &JobId,
    ) -> u32 {
        let mut max_dep_depth = 0u32;
        for dep_id in dependencies {
            if let Some(dep_info) = nodes.get(dep_id) {
                max_dep_depth = max_dep_depth.max(dep_info.depth);
            }
        }

        let computed_depth = max_dep_depth.saturating_add(1);
        assert!(
            computed_depth <= MAX_DEPENDENCY_DEPTH,
            "dependency chain depth {computed_depth} exceeds limit {MAX_DEPENDENCY_DEPTH} for job {job_id}"
        );
        computed_depth
    }

    /// Compute initial dependency state based on which dependencies are completed.
    fn add_job_compute_state(nodes: &HashMap<JobId, JobDependencyInfo>, dependencies: &[JobId]) -> DependencyState {
        if dependencies.is_empty() {
            return DependencyState::Ready;
        }

        // Check which dependencies are already completed
        let mut waiting_on = Vec::new();
        for dep_id in dependencies {
            if let Some(dep_info) = nodes.get(dep_id) {
                if !matches!(dep_info.state, DependencyState::Completed) {
                    waiting_on.push(dep_id.clone());
                }
            } else {
                // Dependency doesn't exist yet, need to wait
                waiting_on.push(dep_id.clone());
            }
        }

        if waiting_on.is_empty() {
            DependencyState::Ready
        } else {
            DependencyState::Waiting(waiting_on)
        }
    }

    /// Add a job with its dependencies.
    pub async fn add_job(
        &self,
        job_id: JobId,
        dependencies: Vec<JobId>,
        failure_policy: DependencyFailurePolicy,
    ) -> Result<()> {
        // Tiger Style: job ID must not be empty
        assert!(!job_id.as_str().is_empty(), "job ID must not be empty");
        // Tiger Style: no self-dependencies in the input list
        assert!(!dependencies.iter().any(|dep| dep == &job_id), "job cannot depend on itself: {job_id}");

        // Check for self-dependency
        if dependencies.contains(&job_id) {
            return Err(JobError::InvalidJobSpec {
                reason: "Job cannot depend on itself".to_string(),
            });
        }

        // Check if adding would create a cycle
        for dep_id in &dependencies {
            if self.would_create_cycle(&job_id, dep_id).await? {
                return Err(JobError::InvalidJobSpec {
                    reason: format!("Adding dependency would create cycle: {} -> {}", job_id, dep_id),
                });
            }
        }

        let mut nodes = self.nodes.write().await;
        let mut edges_forward = self.edges_forward.write().await;
        let mut edges_reverse = self.edges_reverse.write().await;

        // Tiger Style: graph size must not exceed bound
        assert!(
            nodes.len() < MAX_GRAPH_NODES,
            "dependency graph has {} nodes, exceeding limit {}",
            nodes.len(),
            MAX_GRAPH_NODES
        );

        // Calculate depth and validate
        let computed_depth = Self::add_job_calculate_depth(&nodes, &dependencies, &job_id);

        // Compute initial state
        let state = Self::add_job_compute_state(&nodes, &dependencies);

        let job_info = JobDependencyInfo {
            job_id: job_id.clone(),
            dependencies: dependencies.iter().cloned().collect(),
            dependents: HashSet::new(),
            state: state.clone(),
            failure_policy,
            last_check: None,
            depth: computed_depth,
        };

        // Add to graph
        nodes.insert(job_id.clone(), job_info);

        // Update edges
        edges_forward.insert(job_id.clone(), dependencies.iter().cloned().collect());

        for dep_id in &dependencies {
            edges_reverse.entry(dep_id.clone()).or_insert_with(HashSet::new).insert(job_id.clone());

            // Update dependent's info
            if let Some(dep_info) = nodes.get_mut(dep_id) {
                dep_info.dependents.insert(job_id.clone());
            }
        }

        // If ready, add to ready queue
        if matches!(state, DependencyState::Ready) {
            let mut ready_queue = self.ready_queue.write().await;
            ready_queue.push_back(job_id.clone());
        }

        info!(
            job_id = %job_id,
            dependencies = dependencies.len(),
            state = ?state,
            "job added to dependency graph"
        );

        Ok(())
    }

    /// Check if all dependencies are satisfied.
    pub async fn is_ready(&self, job_id: &JobId) -> Result<bool> {
        let nodes = self.nodes.read().await;

        if let Some(info) = nodes.get(job_id) {
            Ok(matches!(info.state, DependencyState::Ready))
        } else {
            Err(JobError::JobNotFound { id: job_id.to_string() })
        }
    }

    /// Check dependencies and update state if ready.
    pub async fn check_dependencies(&self, job_id: &JobId) -> Result<bool> {
        let mut nodes = self.nodes.write().await;

        // Get job info and clone what we need to avoid borrow issues
        let (dependencies, failure_policy) = {
            let info = nodes.get(job_id).ok_or_else(|| JobError::JobNotFound { id: job_id.to_string() })?;

            // Only check if currently waiting
            if !matches!(info.state, DependencyState::Waiting(_)) {
                return Ok(matches!(info.state, DependencyState::Ready));
            }

            (info.dependencies.clone(), info.failure_policy.clone())
        };

        // Check each dependency
        let mut all_satisfied = true;
        let mut failed_deps = Vec::new();
        let mut waiting_on = Vec::new();

        for dep_id in &dependencies {
            if let Some(dep_info) = nodes.get(dep_id) {
                match &dep_info.state {
                    DependencyState::Completed => {
                        // Dependency satisfied
                    }
                    DependencyState::Failed(reason) => {
                        failed_deps.push((dep_id.clone(), reason.clone()));
                        match failure_policy {
                            DependencyFailurePolicy::FailCascade => {
                                all_satisfied = false;
                                break;
                            }
                            DependencyFailurePolicy::SkipFailed => {
                                // Continue checking other deps
                            }
                            _ => {
                                waiting_on.push(dep_id.clone());
                                all_satisfied = false;
                            }
                        }
                    }
                    _ => {
                        waiting_on.push(dep_id.clone());
                        all_satisfied = false;
                    }
                }
            } else {
                // Dependency doesn't exist, keep waiting
                waiting_on.push(dep_id.clone());
                all_satisfied = false;
            }
        }

        // Update state based on results
        let info = nodes.get_mut(job_id).ok_or_else(|| JobError::JobNotFound { id: job_id.to_string() })?;
        info.last_check = Some(Utc::now());

        // Decomposed: check if there are failed deps, then check policy
        let has_failed_deps = !failed_deps.is_empty();
        let should_cascade = matches!(info.failure_policy, DependencyFailurePolicy::FailCascade);
        if has_failed_deps && should_cascade {
            info.state = DependencyState::Failed(format!("Dependency failed: {:?}", failed_deps));
            Ok(false)
        } else if all_satisfied {
            info.state = DependencyState::Ready;

            // Add to ready queue
            drop(nodes);
            let mut ready_queue = self.ready_queue.write().await;
            ready_queue.push_back(job_id.clone());

            info!(job_id = %job_id, "job dependencies satisfied, marked ready");
            Ok(true)
        } else {
            info.state = DependencyState::Waiting(waiting_on);
            Ok(false)
        }
    }

    /// Mark a job as completed and unblock dependents.
    pub async fn mark_completed(&self, job_id: &JobId) -> Result<Vec<JobId>> {
        // Tiger Style: job ID must not be empty
        assert!(!job_id.as_str().is_empty(), "job ID must not be empty for mark_completed");

        let mut nodes = self.nodes.write().await;
        let edges_reverse = self.edges_reverse.read().await;

        // Update job state
        if let Some(info) = nodes.get_mut(job_id) {
            info.state = DependencyState::Completed;
        } else {
            return Ok(Vec::new());
        }

        // Find dependents
        let dependents = edges_reverse.get(job_id).cloned().unwrap_or_default();

        // Check each dependent
        let mut newly_ready = Vec::new();
        for dep_job_id in dependents {
            if let Some(dep_info) = nodes.get_mut(&dep_job_id) {
                // Only process if waiting
                if let DependencyState::Waiting(ref mut waiting_on) = dep_info.state {
                    // Remove completed job from waiting list
                    waiting_on.retain(|id| id != job_id);

                    if waiting_on.is_empty() {
                        // All dependencies satisfied
                        dep_info.state = DependencyState::Ready;
                        newly_ready.push(dep_job_id.clone());
                    }
                }
            }
        }

        // Add newly ready jobs to queue
        if !newly_ready.is_empty() {
            drop(nodes);
            let mut ready_queue = self.ready_queue.write().await;
            for job_id in &newly_ready {
                ready_queue.push_back(job_id.clone());
            }

            info!(
                job_id = %job_id,
                unblocked = newly_ready.len(),
                "job completed, unblocked dependents"
            );
        }

        Ok(newly_ready)
    }

    /// Handle cascade failure policy: mark dependent as failed and queue transitive dependents.
    fn mark_failed_handle_cascade(
        dep_info: &mut JobDependencyInfo,
        dep_job_id: &JobId,
        parent_reason: &str,
        edges_reverse: &HashMap<JobId, HashSet<JobId>>,
        processed: &HashSet<JobId>,
        worklist: &mut VecDeque<(JobId, String)>,
        affected: &mut Vec<JobId>,
    ) {
        let cascade_reason = format!("Cascade failure: {}", parent_reason);
        dep_info.state = DependencyState::Failed(cascade_reason.clone());
        affected.push(dep_job_id.clone());

        // Add this job's dependents to the worklist for recursive processing
        if let Some(transitive_deps) = edges_reverse.get(dep_job_id) {
            for trans_id in transitive_deps {
                if !processed.contains(trans_id) {
                    worklist.push_back((trans_id.clone(), cascade_reason.clone()));
                }
            }
        }
    }

    /// Handle skip-failed policy: remove failed dependency and check if job becomes ready.
    fn mark_failed_handle_skip(
        dep_info: &mut JobDependencyInfo,
        dep_job_id: &JobId,
        failed_job_id: &JobId,
        affected: &mut Vec<JobId>,
    ) {
        if let DependencyState::Waiting(ref mut waiting_on) = dep_info.state {
            waiting_on.retain(|id| id != failed_job_id);
            if waiting_on.is_empty() {
                dep_info.state = DependencyState::Ready;
                affected.push(dep_job_id.clone());
            }
        }
    }

    /// Mark a job as failed and handle dependent failures recursively.
    ///
    /// This uses a worklist algorithm to propagate failures through the entire
    /// dependency graph, handling transitive dependents correctly. All jobs
    /// that are affected by the failure cascade are returned.
    pub async fn mark_failed(&self, job_id: &JobId, reason: String) -> Result<Vec<JobId>> {
        // Tiger Style: job ID must not be empty
        assert!(!job_id.as_str().is_empty(), "job ID must not be empty for mark_failed");
        // Tiger Style: failure reason must not be empty
        assert!(!reason.is_empty(), "failure reason must not be empty for job {job_id}");

        let mut nodes = self.nodes.write().await;
        let edges_reverse = self.edges_reverse.read().await;

        // Update initial job state
        if let Some(info) = nodes.get_mut(job_id) {
            info.state = DependencyState::Failed(reason.clone());
        } else {
            return Ok(Vec::new());
        }

        // Use worklist algorithm to process all affected jobs transitively
        let mut worklist: VecDeque<(JobId, String)> = VecDeque::new();
        let mut processed: HashSet<JobId> = HashSet::new();
        let mut affected = Vec::new();

        // Seed the worklist with direct dependents of the failed job
        if let Some(dependents) = edges_reverse.get(job_id) {
            for dep_id in dependents {
                worklist.push_back((dep_id.clone(), reason.clone()));
            }
        }
        processed.insert(job_id.clone());

        // Process the worklist until empty
        while let Some((dep_job_id, parent_reason)) = worklist.pop_front() {
            if processed.contains(&dep_job_id) {
                continue;
            }
            processed.insert(dep_job_id.clone());

            if let Some(dep_info) = nodes.get_mut(&dep_job_id) {
                if dep_info.state.is_terminal() {
                    continue;
                }

                match dep_info.failure_policy {
                    DependencyFailurePolicy::FailCascade => {
                        Self::mark_failed_handle_cascade(
                            dep_info,
                            &dep_job_id,
                            &parent_reason,
                            &edges_reverse,
                            &processed,
                            &mut worklist,
                            &mut affected,
                        );
                    }
                    DependencyFailurePolicy::SkipFailed => {
                        Self::mark_failed_handle_skip(dep_info, &dep_job_id, job_id, &mut affected);
                    }
                    _ => {
                        // Keep waiting (WaitForRetry, ContinuePartial)
                    }
                }
            }
        }

        warn!(
            job_id = %job_id,
            affected = affected.len(),
            "job failed, affected dependents (recursive cascade)"
        );

        Ok(affected)
    }

    /// Mark a job as running.
    pub async fn mark_running(&self, job_id: &JobId) -> Result<()> {
        // Tiger Style: job ID must not be empty
        assert!(!job_id.as_str().is_empty(), "job ID must not be empty for mark_running");

        let mut nodes = self.nodes.write().await;

        if let Some(info) = nodes.get_mut(job_id) {
            if !matches!(info.state, DependencyState::Ready) {
                return Err(JobError::InvalidJobState {
                    state: format!("{:?}", info.state),
                    operation: "mark_running".to_string(),
                });
            }
            info.state = DependencyState::Running;
            Ok(())
        } else {
            Err(JobError::JobNotFound { id: job_id.to_string() })
        }
    }

    /// Get next ready job.
    pub async fn get_next_ready(&self) -> Option<JobId> {
        let mut ready_queue = self.ready_queue.write().await;
        ready_queue.pop_front()
    }

    /// Get all ready jobs.
    pub async fn get_all_ready(&self) -> Vec<JobId> {
        let ready_queue = self.ready_queue.read().await;
        ready_queue.iter().cloned().collect()
    }

    /// Check if adding an edge would create a cycle.
    async fn would_create_cycle(&self, from: &JobId, to: &JobId) -> Result<bool> {
        // Tiger Style: from and to must not be the same node (self-loop)
        assert!(from != to, "self-loop detected: {from} -> {to}");

        let edges_forward = self.edges_forward.read().await;

        // Use DFS to check if we can reach 'from' starting from 'to'
        let mut visited = HashSet::new();
        let mut stack = vec![to.clone()];

        while let Some(current) = stack.pop() {
            if current == *from {
                return Ok(true); // Found cycle
            }

            if visited.insert(current.clone()) {
                if let Some(neighbors) = edges_forward.get(&current) {
                    for neighbor in neighbors {
                        if !visited.contains(neighbor) {
                            stack.push(neighbor.clone());
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// Detect all cycles in the graph.
    pub async fn detect_cycles(&self) -> Vec<Vec<JobId>> {
        let edges_forward = self.edges_forward.read().await;
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut cycles = Vec::new();

        for node in edges_forward.keys() {
            if !visited.contains(node) {
                let mut path = Vec::new();
                self.dfs_cycles(node, &edges_forward, &mut visited, &mut rec_stack, &mut path, &mut cycles);
            }
        }

        cycles
    }

    /// DFS helper for cycle detection.
    fn dfs_cycles(
        &self,
        node: &JobId,
        edges: &HashMap<JobId, HashSet<JobId>>,
        visited: &mut HashSet<JobId>,
        rec_stack: &mut HashSet<JobId>,
        path: &mut Vec<JobId>,
        cycles: &mut Vec<Vec<JobId>>,
    ) {
        visited.insert(node.clone());
        rec_stack.insert(node.clone());
        path.push(node.clone());

        if let Some(neighbors) = edges.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    self.dfs_cycles(neighbor, edges, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle
                    if let Some(pos) = path.iter().position(|id| id == neighbor) {
                        cycles.push(path[pos..].to_vec());
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
    }

    /// Get topological ordering of jobs.
    pub async fn topological_sort(&self) -> Result<Vec<JobId>> {
        let edges_forward = self.edges_forward.read().await;
        let nodes = self.nodes.read().await;

        // Calculate in-degrees
        let mut in_degree: HashMap<JobId, usize> = HashMap::new();
        for node in nodes.keys() {
            in_degree.insert(node.clone(), 0);
        }

        for neighbors in edges_forward.values() {
            for neighbor in neighbors {
                *in_degree.entry(neighbor.clone()).or_insert(0) += 1;
            }
        }

        // Find nodes with no dependencies
        let mut queue = VecDeque::new();
        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node.clone());
            }
        }

        // Process nodes
        let mut result = Vec::new();
        while let Some(node) = queue.pop_front() {
            result.push(node.clone());

            if let Some(neighbors) = edges_forward.get(&node) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }

        // Check if all nodes were processed (no cycles)
        if result.len() != nodes.len() {
            return Err(JobError::InvalidJobSpec {
                reason: "Graph contains cycles".to_string(),
            });
        }

        // Tiger Style: topological sort result must include all nodes
        debug_assert_eq!(
            result.len(),
            nodes.len(),
            "topological sort produced {} nodes but graph has {}",
            result.len(),
            nodes.len()
        );

        Ok(result)
    }

    /// Get dependency chain for a job.
    pub async fn get_dependency_chain(&self, job_id: &JobId) -> Result<Vec<JobId>> {
        // Tiger Style: job ID must not be empty
        assert!(!job_id.as_str().is_empty(), "job ID must not be empty for get_dependency_chain");

        let edges_forward = self.edges_forward.read().await;
        let mut chain = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = vec![job_id.clone()];

        while let Some(current) = stack.pop() {
            if visited.insert(current.clone()) {
                chain.push(current.clone());

                // Tiger Style: bound traversal to graph size to prevent runaway
                debug_assert!(
                    chain.len() <= MAX_GRAPH_NODES,
                    "dependency chain traversal exceeded {MAX_GRAPH_NODES} nodes"
                );

                if let Some(deps) = edges_forward.get(&current) {
                    for dep in deps {
                        if !visited.contains(dep) {
                            stack.push(dep.clone());
                        }
                    }
                }
            }
        }

        Ok(chain)
    }

    /// Get jobs blocked by a given job.
    pub async fn get_blocked_by(&self, job_id: &JobId) -> Vec<JobId> {
        let edges_reverse = self.edges_reverse.read().await;
        edges_reverse.get(job_id).map(|deps| deps.iter().cloned().collect()).unwrap_or_default()
    }

    /// Get job dependency info.
    pub async fn get_job_info(&self, job_id: &JobId) -> Option<JobDependencyInfo> {
        let nodes = self.nodes.read().await;
        nodes.get(job_id).cloned()
    }

    /// Clear completed jobs from the graph.
    pub async fn cleanup_completed(&self) -> usize {
        let mut nodes = self.nodes.write().await;
        let mut edges_forward = self.edges_forward.write().await;
        let mut edges_reverse = self.edges_reverse.write().await;

        let completed: Vec<JobId> = nodes
            .iter()
            .filter(|(_, info)| matches!(info.state, DependencyState::Completed))
            .map(|(id, _)| id.clone())
            .collect();

        let count = completed.len();
        let initial_node_count = nodes.len();

        for job_id in completed {
            nodes.remove(&job_id);
            edges_forward.remove(&job_id);
            edges_reverse.remove(&job_id);
        }

        // Tiger Style: nodes removed must equal completed count
        debug_assert_eq!(
            initial_node_count - nodes.len(),
            count,
            "cleanup removed {} nodes but expected to remove {}",
            initial_node_count - nodes.len(),
            count
        );

        debug!(removed = count, "cleaned up completed jobs from dependency graph");
        count
    }
}
