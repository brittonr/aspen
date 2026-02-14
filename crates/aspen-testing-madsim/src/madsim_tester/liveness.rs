//! Liveness mode testing types (TigerBeetle-style two-phase testing).

use std::time::Instant;

use serde::Deserialize;
use serde::Serialize;

/// Metrics for liveness testing.
///
/// These metrics track the system's ability to make progress under
/// various failure conditions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LivenessMetrics {
    /// Total time (ms) the cluster had no leader.
    pub leaderless_duration_ms: u64,
    /// Number of times the cluster became leaderless.
    pub leaderless_periods: u32,
    /// Time (ms) for first leader election after cluster start.
    pub first_election_ms: u64,
    /// Maximum time (ms) to recover a leader after crash/partition.
    pub max_leader_recovery_ms: u64,
    /// Number of successful writes during liveness testing.
    pub writes_completed: u64,
    /// Number of failed writes due to no leader.
    pub writes_blocked: u64,
    /// Total time (ms) spent with operations blocked.
    pub blocked_duration_ms: u64,
    /// Number of liveness checks performed.
    pub liveness_checks: u64,
    /// Number of liveness checks that passed.
    pub liveness_checks_passed: u64,
}

/// Liveness testing mode.
///
/// TigerBeetle uses a two-phase testing approach:
/// 1. **Safety mode**: Run tests with strict invariant checking (no progress required)
/// 2. **Liveness mode**: Re-run the same tests, requiring progress to be made
///
/// This allows testing both correctness (safety) and progress (liveness) guarantees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub enum LivenessMode {
    /// Disabled (default): Only check safety invariants, don't require progress.
    #[default]
    Disabled,
    /// Strict: Cluster must always have a leader within timeout.
    /// This is the strongest liveness guarantee.
    Strict,
    /// Eventual: Cluster may temporarily lack a leader, but must recover.
    /// Allows transient unavailability during failures.
    Eventual,
    /// Custom timeout: Like Eventual, but with a custom recovery timeout.
    /// The value is the maximum allowed leaderless duration in milliseconds.
    CustomTimeout(u64),
}

/// Configuration for liveness testing.
#[derive(Debug, Clone)]
pub struct LivenessConfig {
    /// Liveness mode to use.
    pub mode: LivenessMode,
    /// How often to check liveness (ms).
    pub check_interval_ms: u64,
    /// Maximum time to wait for leader recovery (ms).
    /// Used when mode is Eventual.
    pub recovery_timeout_ms: u64,
    /// Whether to track detailed timing metrics.
    pub track_timing: bool,
}

impl Default for LivenessConfig {
    fn default() -> Self {
        Self {
            mode: LivenessMode::Disabled,
            check_interval_ms: 100,      // Check every 100ms
            recovery_timeout_ms: 30_000, // 30 second recovery timeout
            track_timing: true,
        }
    }
}

impl LivenessConfig {
    /// Create a strict liveness configuration.
    pub fn strict() -> Self {
        Self {
            mode: LivenessMode::Strict,
            check_interval_ms: 100,
            recovery_timeout_ms: 10_000, // Strict = shorter timeout
            track_timing: true,
        }
    }

    /// Create an eventual liveness configuration.
    pub fn eventual() -> Self {
        Self {
            mode: LivenessMode::Eventual,
            ..Default::default()
        }
    }

    /// Create a custom liveness configuration with specific timeout.
    pub fn with_timeout(max_leaderless_ms: u64) -> Self {
        Self {
            mode: LivenessMode::CustomTimeout(max_leaderless_ms),
            recovery_timeout_ms: max_leaderless_ms,
            ..Default::default()
        }
    }
}

/// Report from liveness testing.
#[derive(Debug, Clone, Serialize)]
pub struct LivenessReport {
    /// Overall liveness status.
    pub passed: bool,
    /// Liveness mode that was used.
    pub mode: LivenessMode,
    /// Detailed metrics from the test.
    pub metrics: LivenessMetrics,
    /// Any liveness violations detected.
    pub violations: Vec<LivenessViolation>,
    /// Summary message.
    pub summary: String,
}

/// A liveness violation event.
#[derive(Debug, Clone, Serialize)]
pub struct LivenessViolation {
    /// When the violation started (relative to test start, in ms).
    pub started_at_ms: u64,
    /// Duration of the violation (ms).
    pub duration_ms: u64,
    /// Type of violation.
    pub violation_type: ViolationType,
    /// Additional context.
    pub context: String,
}

/// Types of liveness violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ViolationType {
    /// Cluster had no leader for too long.
    LeaderlessTimeout,
    /// Operation was blocked for too long.
    OperationBlocked,
    /// Cluster failed to recover after failure injection.
    RecoveryTimeout,
}

/// Internal state for liveness tracking.
#[derive(Debug, Clone, Default)]
pub(crate) struct LivenessState {
    /// Whether liveness tracking is active.
    pub(crate) active: bool,
    /// When the cluster last had a leader.
    pub(crate) last_leader_time: Option<Instant>,
    /// When the cluster became leaderless (for tracking duration).
    pub(crate) leaderless_since: Option<Instant>,
    /// Accumulated violations.
    pub(crate) violations: Vec<LivenessViolation>,
    /// First election time (set once on first leader).
    pub(crate) first_election_time: Option<Instant>,
}
