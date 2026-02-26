//! Pure pipeline DAG validation functions.
//!
//! These functions validate pipeline stage dependencies and compute
//! execution order without side effects.
//!
//! # Tiger Style
//!
//! - Pure validation functions
//! - Explicit error conditions
//! - No I/O or system calls

/// Result of stage dependency validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageValidationError {
    /// Stage depends on itself.
    SelfDependency {
        /// Name of the stage that depends on itself
        stage_name: String,
    },
    /// Stage depends on non-existent stage.
    MissingDependency {
        /// Name of the stage with the missing dependency
        stage_name: String,
        /// Name of the missing dependency
        missing_dep: String,
    },
    /// Circular dependency detected.
    CyclicDependency {
        /// Stages forming the circular dependency
        cycle: Vec<String>,
    },
    /// Too many stages in pipeline.
    TooManyStages {
        /// Current number of stages
        count: usize,
        /// Maximum allowed stages
        max: usize,
    },
    /// Too many jobs in pipeline.
    TooManyJobs {
        /// Current number of jobs
        count: usize,
        /// Maximum allowed jobs
        max: usize,
    },
}

/// Check if a stage has a self-dependency.
///
/// # Arguments
///
/// * `stage_name` - Name of the stage being validated
/// * `dependencies` - List of stage dependencies
///
/// # Returns
///
/// `true` if the stage depends on itself.
///
/// # Example
///
/// ```
/// use aspen_ci_core::verified::has_self_dependency;
///
/// assert!(has_self_dependency("build", &["build"]));
/// assert!(!has_self_dependency("build", &["test"]));
/// assert!(!has_self_dependency("build", &[]));
/// ```
#[inline]
pub fn has_self_dependency(stage_name: &str, dependencies: &[&str]) -> bool {
    dependencies.contains(&stage_name)
}

/// Check if all stage dependencies exist.
///
/// # Arguments
///
/// * `dependencies` - Dependencies to check
/// * `all_stage_names` - All valid stage names
///
/// # Returns
///
/// `None` if all dependencies exist, `Some(missing)` otherwise.
///
/// # Example
///
/// ```
/// use aspen_ci_core::verified::find_missing_dependency;
///
/// let stages = &["build", "test", "deploy"];
///
/// assert_eq!(find_missing_dependency(&["build"], stages), None);
/// assert_eq!(find_missing_dependency(&["unknown"], stages), Some("unknown"));
/// ```
#[inline]
pub fn find_missing_dependency<'a>(dependencies: &[&'a str], all_stage_names: &[&str]) -> Option<&'a str> {
    dependencies.iter().find(|dep| !all_stage_names.contains(dep)).copied()
}

/// Check if all dependencies for a stage are satisfied.
///
/// A dependency is satisfied if it is in the completed set.
///
/// # Arguments
///
/// * `dependencies` - List of dependencies for the stage
/// * `completed_stages` - Set of completed stage names
///
/// # Returns
///
/// `true` if all dependencies are satisfied.
///
/// # Example
///
/// ```
/// use aspen_ci_core::verified::are_dependencies_met;
///
/// let completed = &["build", "lint"];
///
/// assert!(are_dependencies_met(&["build"], completed));
/// assert!(are_dependencies_met(&["build", "lint"], completed));
/// assert!(!are_dependencies_met(&["build", "test"], completed));
/// assert!(are_dependencies_met(&[], completed));
/// ```
#[inline]
pub fn are_dependencies_met(dependencies: &[&str], completed_stages: &[&str]) -> bool {
    dependencies.iter().all(|dep| completed_stages.contains(dep))
}

/// Find stages that are ready to execute.
///
/// A stage is ready if all its dependencies are in the completed set
/// and it hasn't been started yet.
///
/// # Arguments
///
/// * `stages_with_deps` - List of (stage_name, dependencies) pairs
/// * `completed_stages` - Stages that have completed
/// * `started_stages` - Stages that have started (running or completed)
///
/// # Returns
///
/// List of stage names that are ready to start.
///
/// # Example
///
/// ```
/// use aspen_ci_core::verified::find_ready_stages;
///
/// let stages = &[
///     ("build", vec![]),
///     ("test", vec!["build"]),
///     ("deploy", vec!["test"]),
/// ];
///
/// // Initially only build is ready
/// assert_eq!(
///     find_ready_stages(stages, &[], &[]),
///     vec!["build"]
/// );
///
/// // After build completes, test is ready
/// assert_eq!(
///     find_ready_stages(stages, &["build"], &["build"]),
///     vec!["test"]
/// );
/// ```
pub fn find_ready_stages<'a>(
    stages_with_deps: &[(&'a str, Vec<&str>)],
    completed_stages: &[&str],
    started_stages: &[&str],
) -> Vec<&'a str> {
    stages_with_deps
        .iter()
        .filter(|(name, deps)| {
            // Not yet started
            !started_stages.contains(name) &&
            // All dependencies completed
            are_dependencies_met(deps, completed_stages)
        })
        .map(|(name, _)| *name)
        .collect()
}

/// Count total jobs across all stages.
///
/// # Arguments
///
/// * `jobs_per_stage` - Number of jobs in each stage
///
/// # Returns
///
/// Total job count (saturating at usize::MAX).
///
/// # Example
///
/// ```
/// use aspen_ci_core::verified::count_total_jobs;
///
/// assert_eq!(count_total_jobs(&[3, 2, 5]), 10);
/// assert_eq!(count_total_jobs(&[]), 0);
/// ```
#[inline]
pub fn count_total_jobs(jobs_per_stage: &[usize]) -> usize {
    jobs_per_stage.iter().fold(0usize, |acc, &count| acc.saturating_add(count))
}

/// Check if pipeline size is within limits.
///
/// # Arguments
///
/// * `stage_count` - Number of stages
/// * `job_count` - Total number of jobs
/// * `max_stages` - Maximum allowed stages
/// * `max_jobs` - Maximum allowed jobs
///
/// # Returns
///
/// `Ok(())` if within limits, `Err` with the limit exceeded.
///
/// # Example
///
/// ```
/// use aspen_ci_core::verified::{check_pipeline_limits, StageValidationError};
///
/// assert!(check_pipeline_limits(5, 20, 10, 100).is_ok());
///
/// assert!(matches!(
///     check_pipeline_limits(15, 20, 10, 100),
///     Err(StageValidationError::TooManyStages { count: 15, max: 10 })
/// ));
/// ```
#[inline]
pub fn check_pipeline_limits(
    stage_count: usize,
    job_count: usize,
    max_stages: usize,
    max_jobs: usize,
) -> Result<(), StageValidationError> {
    if stage_count > max_stages {
        return Err(StageValidationError::TooManyStages {
            count: stage_count,
            max: max_stages,
        });
    }

    if job_count > max_jobs {
        return Err(StageValidationError::TooManyJobs {
            count: job_count,
            max: max_jobs,
        });
    }

    Ok(())
}

/// Compute stage execution order using topological sort.
///
/// Returns stages in an order where dependencies come before dependents.
/// Uses Kahn's algorithm for simplicity.
///
/// # Arguments
///
/// * `stages_with_deps` - List of (stage_index, dependency_indices) pairs
/// * `stage_count` - Total number of stages
///
/// # Returns
///
/// `Some(order)` with indices in execution order, `None` if cycle detected.
///
/// # Example
///
/// ```
/// use aspen_ci_core::verified::compute_stage_order;
///
/// // build (0) -> test (1) -> deploy (2)
/// let deps = &[
///     (0, vec![]),      // build has no deps
///     (1, vec![0]),     // test depends on build
///     (2, vec![1]),     // deploy depends on test
/// ];
///
/// assert_eq!(compute_stage_order(deps, 3), Some(vec![0, 1, 2]));
///
/// // Cycle: a -> b -> a
/// let cycle_deps = &[
///     (0, vec![1]),
///     (1, vec![0]),
/// ];
/// assert_eq!(compute_stage_order(cycle_deps, 2), None);
/// ```
pub fn compute_stage_order(stages_with_deps: &[(usize, Vec<usize>)], stage_count: usize) -> Option<Vec<usize>> {
    if stage_count == 0 {
        return Some(Vec::new());
    }

    // Compute in-degree for each stage (number of dependencies)
    let mut in_degree = vec![0usize; stage_count];
    for (idx, deps) in stages_with_deps {
        if *idx < stage_count {
            in_degree[*idx] = deps.len();
        }
    }

    // Start with stages that have no dependencies
    let mut queue: Vec<usize> = (0..stage_count).filter(|&i| in_degree[i] == 0).collect();

    let mut result = Vec::with_capacity(stage_count);
    let mut processed = vec![false; stage_count];

    while let Some(stage) = queue.pop() {
        if processed[stage] {
            continue;
        }
        processed[stage] = true;
        result.push(stage);

        // Find stages that depend on this one and decrement their in-degree
        for (idx, deps) in stages_with_deps {
            if !processed[*idx] && deps.contains(&stage) {
                in_degree[*idx] = in_degree[*idx].saturating_sub(1);
                if in_degree[*idx] == 0 {
                    queue.push(*idx);
                }
            }
        }
    }

    if result.len() == stage_count {
        Some(result)
    } else {
        None // Cycle detected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // has_self_dependency tests
    // ========================================================================

    #[test]
    fn test_self_dependency_true() {
        assert!(has_self_dependency("build", &["build"]));
    }

    #[test]
    fn test_self_dependency_false() {
        assert!(!has_self_dependency("build", &["test"]));
    }

    #[test]
    fn test_self_dependency_empty() {
        assert!(!has_self_dependency("build", &[]));
    }

    // ========================================================================
    // find_missing_dependency tests
    // ========================================================================

    #[test]
    fn test_no_missing() {
        let stages = &["build", "test", "deploy"];
        assert_eq!(find_missing_dependency(&["build"], stages), None);
    }

    #[test]
    fn test_has_missing() {
        let stages = &["build", "test"];
        assert_eq!(find_missing_dependency(&["deploy"], stages), Some("deploy"));
    }

    // ========================================================================
    // are_dependencies_met tests
    // ========================================================================

    #[test]
    fn test_deps_met() {
        assert!(are_dependencies_met(&["build"], &["build", "lint"]));
    }

    #[test]
    fn test_deps_not_met() {
        assert!(!are_dependencies_met(&["build", "test"], &["build"]));
    }

    #[test]
    fn test_no_deps() {
        assert!(are_dependencies_met(&[], &["build"]));
    }

    // ========================================================================
    // find_ready_stages tests
    // ========================================================================

    #[test]
    fn test_initial_ready() {
        let stages = &[("build", vec![]), ("test", vec!["build"])];
        assert_eq!(find_ready_stages(stages, &[], &[]), vec!["build"]);
    }

    #[test]
    fn test_after_build() {
        let stages = &[("build", vec![]), ("test", vec!["build"])];
        assert_eq!(find_ready_stages(stages, &["build"], &["build"]), vec!["test"]);
    }

    #[test]
    fn test_parallel_stages() {
        let stages = &[("build", vec![]), ("lint", vec![]), ("test", vec!["build"])];
        let ready = find_ready_stages(stages, &[], &[]);
        assert!(ready.contains(&"build"));
        assert!(ready.contains(&"lint"));
        assert!(!ready.contains(&"test"));
    }

    // ========================================================================
    // count_total_jobs tests
    // ========================================================================

    #[test]
    fn test_count_jobs() {
        assert_eq!(count_total_jobs(&[3, 2, 5]), 10);
    }

    #[test]
    fn test_count_empty() {
        assert_eq!(count_total_jobs(&[]), 0);
    }

    // ========================================================================
    // check_pipeline_limits tests
    // ========================================================================

    #[test]
    fn test_within_limits() {
        assert!(check_pipeline_limits(5, 20, 10, 100).is_ok());
    }

    #[test]
    fn test_too_many_stages() {
        match check_pipeline_limits(15, 20, 10, 100) {
            Err(StageValidationError::TooManyStages { count, max }) => {
                assert_eq!(count, 15);
                assert_eq!(max, 10);
            }
            other => panic!("Expected TooManyStages, got {:?}", other),
        }
    }

    #[test]
    fn test_too_many_jobs() {
        match check_pipeline_limits(5, 150, 10, 100) {
            Err(StageValidationError::TooManyJobs { count, max }) => {
                assert_eq!(count, 150);
                assert_eq!(max, 100);
            }
            other => panic!("Expected TooManyJobs, got {:?}", other),
        }
    }

    // ========================================================================
    // compute_stage_order tests
    // ========================================================================

    #[test]
    fn test_linear_order() {
        let deps = &[(0, vec![]), (1, vec![0]), (2, vec![1])];
        assert_eq!(compute_stage_order(deps, 3), Some(vec![0, 1, 2]));
    }

    #[test]
    fn test_cycle_detection() {
        let deps = &[(0, vec![1]), (1, vec![0])];
        assert_eq!(compute_stage_order(deps, 2), None);
    }

    #[test]
    fn test_empty_pipeline() {
        let deps: &[(usize, Vec<usize>)] = &[];
        assert_eq!(compute_stage_order(deps, 0), Some(vec![]));
    }

    #[test]
    fn test_stage_order_parallel() {
        // Both stage 0 and 1 have no deps, can run in any order
        let deps = &[(0, vec![]), (1, vec![]), (2, vec![0, 1])];
        let order = compute_stage_order(deps, 3);
        assert!(order.is_some());
        let order = order.unwrap();
        // Stage 2 must come after 0 and 1
        let pos_0 = order.iter().position(|&x| x == 0).unwrap();
        let pos_1 = order.iter().position(|&x| x == 1).unwrap();
        let pos_2 = order.iter().position(|&x| x == 2).unwrap();
        assert!(pos_2 > pos_0);
        assert!(pos_2 > pos_1);
    }
}
