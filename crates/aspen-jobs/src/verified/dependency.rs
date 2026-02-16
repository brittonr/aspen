//! Pure dependency graph validation functions.
//!
//! These functions validate job dependencies without accessing the
//! actual dependency graph or state. They encapsulate pure validation
//! logic that can be tested independently.
//!
//! # Tiger Style
//!
//! - Pure validation without side effects
//! - Explicit error conditions
//! - Deterministic behavior

/// Check if a job would have a self-dependency.
///
/// # Arguments
///
/// * `job_id` - The job being added
/// * `dependency_id` - A potential dependency
///
/// # Returns
///
/// `true` if the job would depend on itself.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::is_self_dependency;
///
/// assert!(is_self_dependency("job-1", "job-1"));
/// assert!(!is_self_dependency("job-1", "job-2"));
/// ```
#[inline]
pub fn is_self_dependency(job_id: &str, dependency_id: &str) -> bool {
    job_id == dependency_id
}

/// Check if any of the dependencies would create a self-dependency.
///
/// # Arguments
///
/// * `job_id` - The job being added
/// * `dependencies` - List of job IDs this job would depend on
///
/// # Returns
///
/// `true` if any dependency equals the job ID.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::has_self_dependency;
///
/// assert!(has_self_dependency("job-1", &["job-2", "job-1", "job-3"]));
/// assert!(!has_self_dependency("job-1", &["job-2", "job-3"]));
/// assert!(!has_self_dependency("job-1", &[]));
/// ```
#[inline]
pub fn has_self_dependency(job_id: &str, dependencies: &[&str]) -> bool {
    dependencies.contains(&job_id)
}

/// Compute the depth of a job in the dependency tree.
///
/// Depth is 1 + maximum depth of all dependencies.
///
/// # Arguments
///
/// * `dependency_depths` - Depths of all direct dependencies
///
/// # Returns
///
/// The computed depth for this job.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::compute_dependency_depth;
///
/// // No dependencies -> depth 1
/// assert_eq!(compute_dependency_depth(&[]), 1);
///
/// // Max dependency depth is 3 -> this job is depth 4
/// assert_eq!(compute_dependency_depth(&[1, 3, 2]), 4);
/// ```
#[inline]
pub fn compute_dependency_depth(dependency_depths: &[u32]) -> u32 {
    let max_depth = dependency_depths.iter().copied().max().unwrap_or(0);
    let result = max_depth.saturating_add(1);

    // Tiger Style: postcondition - result is always >= 1
    debug_assert!(result >= 1, "DEP_DEPTH: result must be >= 1");
    // Tiger Style: postcondition - result is always > max input
    debug_assert!(
        dependency_depths.is_empty() || result > max_depth || max_depth == u32::MAX,
        "DEP_DEPTH: result {} must be > max_depth {}",
        result,
        max_depth
    );
    result
}

/// Determine the initial dependency state for a job.
///
/// A job is ready if it has no dependencies or all dependencies are complete.
///
/// # Arguments
///
/// * `total_dependencies` - Total number of declared dependencies
/// * `completed_dependencies` - Number of dependencies already completed
///
/// # Returns
///
/// `true` if the job is ready to execute (all dependencies satisfied).
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::is_initially_ready;
///
/// // No dependencies
/// assert!(is_initially_ready(0, 0));
///
/// // All dependencies complete
/// assert!(is_initially_ready(3, 3));
///
/// // Some dependencies pending
/// assert!(!is_initially_ready(3, 2));
/// ```
#[inline]
pub const fn is_initially_ready(total_dependencies: u32, completed_dependencies: u32) -> bool {
    total_dependencies == completed_dependencies
}

/// Determine if all dependencies in a waiting list are satisfied.
///
/// # Arguments
///
/// * `waiting_count` - Number of dependencies still being waited on
///
/// # Returns
///
/// `true` if no dependencies are being waited on.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::are_all_dependencies_satisfied;
///
/// assert!(are_all_dependencies_satisfied(0));
/// assert!(!are_all_dependencies_satisfied(1));
/// assert!(!are_all_dependencies_satisfied(5));
/// ```
#[inline]
pub const fn are_all_dependencies_satisfied(waiting_count: u32) -> bool {
    waiting_count == 0
}

/// Dependency state category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyStateCategory {
    /// Job is waiting on dependencies.
    Waiting,
    /// Job is ready to execute.
    Ready,
    /// Job is currently running.
    Running,
    /// Job is in a terminal state (completed, failed, or blocked).
    Terminal,
}

/// Categorize a dependency state for routing logic.
///
/// # Arguments
///
/// * `is_waiting` - Whether the job is waiting on dependencies
/// * `is_ready` - Whether the job is ready to execute
/// * `is_running` - Whether the job is currently running
/// * `is_terminal` - Whether the job is in a terminal state
///
/// # Returns
///
/// The category of the dependency state.
///
/// # Example
///
/// ```
/// use aspen_jobs::verified::{DependencyStateCategory, categorize_dependency_state};
///
/// assert_eq!(
///     categorize_dependency_state(true, false, false, false),
///     DependencyStateCategory::Waiting
/// );
/// assert_eq!(
///     categorize_dependency_state(false, true, false, false),
///     DependencyStateCategory::Ready
/// );
/// ```
#[inline]
pub const fn categorize_dependency_state(
    is_waiting: bool,
    is_ready: bool,
    is_running: bool,
    is_terminal: bool,
) -> DependencyStateCategory {
    if is_terminal {
        DependencyStateCategory::Terminal
    } else if is_running {
        DependencyStateCategory::Running
    } else if is_ready {
        DependencyStateCategory::Ready
    } else if is_waiting {
        DependencyStateCategory::Waiting
    } else {
        // Default to waiting if no flags set (shouldn't happen)
        DependencyStateCategory::Waiting
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // is_self_dependency tests
    // ========================================================================

    #[test]
    fn test_self_dependency_true() {
        assert!(is_self_dependency("job-1", "job-1"));
    }

    #[test]
    fn test_self_dependency_false() {
        assert!(!is_self_dependency("job-1", "job-2"));
    }

    #[test]
    fn test_self_dependency_empty() {
        assert!(is_self_dependency("", ""));
    }

    // ========================================================================
    // has_self_dependency tests
    // ========================================================================

    #[test]
    fn test_has_self_dep_found() {
        assert!(has_self_dependency("job-1", &["job-2", "job-1", "job-3"]));
    }

    #[test]
    fn test_has_self_dep_not_found() {
        assert!(!has_self_dependency("job-1", &["job-2", "job-3"]));
    }

    #[test]
    fn test_has_self_dep_empty() {
        assert!(!has_self_dependency("job-1", &[]));
    }

    // ========================================================================
    // compute_dependency_depth tests
    // ========================================================================

    #[test]
    fn test_depth_no_deps() {
        assert_eq!(compute_dependency_depth(&[]), 1);
    }

    #[test]
    fn test_depth_with_deps() {
        assert_eq!(compute_dependency_depth(&[1, 3, 2]), 4);
    }

    #[test]
    fn test_depth_single_dep() {
        assert_eq!(compute_dependency_depth(&[5]), 6);
    }

    #[test]
    fn test_depth_saturation() {
        assert_eq!(compute_dependency_depth(&[u32::MAX]), u32::MAX);
    }

    // ========================================================================
    // is_initially_ready tests
    // ========================================================================

    #[test]
    fn test_initially_ready_no_deps() {
        assert!(is_initially_ready(0, 0));
    }

    #[test]
    fn test_initially_ready_all_complete() {
        assert!(is_initially_ready(3, 3));
    }

    #[test]
    fn test_initially_ready_some_pending() {
        assert!(!is_initially_ready(3, 2));
    }

    // ========================================================================
    // are_all_dependencies_satisfied tests
    // ========================================================================

    #[test]
    fn test_all_satisfied() {
        assert!(are_all_dependencies_satisfied(0));
    }

    #[test]
    fn test_not_all_satisfied() {
        assert!(!are_all_dependencies_satisfied(1));
        assert!(!are_all_dependencies_satisfied(5));
    }

    // ========================================================================
    // categorize_dependency_state tests
    // ========================================================================

    #[test]
    fn test_categorize_terminal() {
        assert_eq!(categorize_dependency_state(false, false, false, true), DependencyStateCategory::Terminal);
        // Terminal takes precedence
        assert_eq!(categorize_dependency_state(true, true, true, true), DependencyStateCategory::Terminal);
    }

    #[test]
    fn test_categorize_running() {
        assert_eq!(categorize_dependency_state(false, false, true, false), DependencyStateCategory::Running);
    }

    #[test]
    fn test_categorize_ready() {
        assert_eq!(categorize_dependency_state(false, true, false, false), DependencyStateCategory::Ready);
    }

    #[test]
    fn test_categorize_waiting() {
        assert_eq!(categorize_dependency_state(true, false, false, false), DependencyStateCategory::Waiting);
    }
}
