//! Liveness tracking methods for AspenRaftTester.

use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use anyhow::Result;

use super::AspenRaftTester;
use super::liveness::LivenessMetrics;
use super::liveness::LivenessMode;
use super::liveness::LivenessReport;
use super::liveness::LivenessViolation;
use super::liveness::ViolationType;

fn duration_ms_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "madsim liveness tracking needs monotonic instants to measure elections and blocked time"
)]
fn current_instant() -> Instant {
    Instant::now()
}

impl AspenRaftTester {
    /// Enable liveness tracking at runtime.
    ///
    /// This allows enabling liveness mode after tester creation.
    /// Useful for two-phase testing where you first run safety tests,
    /// then re-run with liveness checking.
    pub fn enable_liveness(&mut self, mode: LivenessMode) {
        self.liveness_config.mode = mode;
        self.liveness_state.active = mode != LivenessMode::Disabled;
        self.add_event(format!("liveness: enabled mode {:?}", mode));
    }

    /// Disable liveness tracking.
    pub fn disable_liveness(&mut self) {
        self.liveness_config.mode = LivenessMode::Disabled;
        self.liveness_state.active = false;
        self.add_event("liveness: disabled");
    }

    /// Check if the cluster currently has a leader (quick check, no retries).
    ///
    /// Unlike `check_one_leader()` which waits with retries, this is a
    /// point-in-time snapshot used for liveness tracking.
    pub fn has_leader_now(&self) -> bool {
        for node in self.nodes.iter() {
            if node.connected().load(Ordering::Relaxed) {
                let metrics = node.raft().metrics().borrow().clone();
                if metrics.current_leader.is_some() {
                    return true;
                }
            }
        }
        false
    }

    /// Perform a liveness check and update internal state.
    ///
    /// This should be called periodically during tests to track liveness.
    /// Returns whether the cluster currently satisfies liveness requirements.
    pub fn check_liveness_tick(&mut self) -> bool {
        if !self.liveness_state.active {
            return true; // Not tracking, always passes
        }

        let now = current_instant();
        let elapsed_from_start = duration_ms_u64(now.duration_since(self.start_time));
        let has_leader = self.has_leader_now();

        self.metrics.liveness.liveness_checks += 1;

        if has_leader {
            self.record_leader_present(now, elapsed_from_start);
            true
        } else {
            self.record_leader_absent(now, elapsed_from_start)
        }
    }

    /// Record that a leader is present during a liveness tick.
    fn record_leader_present(&mut self, now: Instant, elapsed_from_start: u64) {
        // Record first election time
        if self.liveness_state.first_election_time.is_none() {
            self.liveness_state.first_election_time = Some(now);
            self.metrics.liveness.first_election_ms = elapsed_from_start;
            self.add_event(format!("liveness: first leader elected at {}ms", elapsed_from_start));
        }

        // If we were leaderless, record recovery time
        if let Some(leaderless_start) = self.liveness_state.leaderless_since {
            let recovery_time_ms = duration_ms_u64(now.duration_since(leaderless_start));
            self.metrics.liveness.leaderless_duration_ms += recovery_time_ms;
            self.metrics.liveness.max_leader_recovery_ms =
                self.metrics.liveness.max_leader_recovery_ms.max(recovery_time_ms);
            self.liveness_state.leaderless_since = None;
            self.add_event(format!("liveness: leader recovered after {}ms", recovery_time_ms));
        }

        self.liveness_state.last_leader_time = Some(now);
        self.metrics.liveness.liveness_checks_passed += 1;
    }

    /// Record that no leader is present during a liveness tick.
    ///
    /// Returns whether the liveness check still passes (no violation).
    fn record_leader_absent(&mut self, now: Instant, elapsed_from_start: u64) -> bool {
        if self.liveness_state.leaderless_since.is_none() {
            self.liveness_state.leaderless_since = Some(now);
            self.metrics.liveness.leaderless_periods += 1;
        }

        // Check for violations based on mode
        let leaderless_duration_ms = self
            .liveness_state
            .leaderless_since
            .map(|leaderless_start| duration_ms_u64(now.duration_since(leaderless_start)))
            .unwrap_or(0);

        let violation_threshold = match self.liveness_config.mode {
            LivenessMode::Disabled => u64::MAX, // Never violate
            LivenessMode::Strict => self.liveness_config.check_interval_ms.saturating_mul(2),
            LivenessMode::Eventual => self.liveness_config.recovery_timeout_ms,
            LivenessMode::CustomTimeout(ms) => ms,
        };

        if leaderless_duration_ms > violation_threshold {
            self.liveness_state.violations.push(LivenessViolation {
                started_at_ms: elapsed_from_start.saturating_sub(leaderless_duration_ms),
                duration_ms: leaderless_duration_ms,
                violation_type: ViolationType::LeaderlessTimeout,
                context: format!(
                    "Cluster leaderless for {}ms (threshold: {}ms)",
                    leaderless_duration_ms, violation_threshold
                ),
            });
            self.add_event(format!("liveness: VIOLATION - leaderless for {}ms", leaderless_duration_ms));
            false
        } else {
            self.metrics.liveness.liveness_checks_passed += 1;
            true
        }
    }

    /// Run a test with continuous liveness checking.
    ///
    /// This is the main entry point for TigerBeetle-style liveness testing.
    /// Runs the provided test function while periodically checking liveness.
    pub async fn run_with_liveness<F, Fut>(&mut self, duration: Duration, test_fn: F) -> LivenessReport
    where
        F: FnOnce(&mut Self) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let start = current_instant();
        let check_sleep = Duration::from_millis(self.liveness_config.check_interval_ms);

        // Initial liveness check
        self.check_liveness_tick();

        // Run the test function
        let test_result = test_fn(self).await;

        // Continue checking until duration expires
        while start.elapsed() < duration {
            self.check_liveness_tick();
            madsim::time::sleep(check_sleep).await;
        }

        // Final check
        self.check_liveness_tick();

        // Generate report
        self.generate_liveness_report(test_result)
    }

    /// Run a liveness test loop with BUGGIFY fault injection.
    ///
    /// Combines BUGGIFY fault injection with liveness checking.
    /// This is the most comprehensive test mode.
    pub async fn run_with_liveness_and_buggify(&mut self, duration: Duration) -> LivenessReport {
        let start = current_instant();
        let check_sleep = Duration::from_millis(self.liveness_config.check_interval_ms);
        let fault_sleep = Duration::from_secs(1);
        let mut last_fault_instant = current_instant();

        self.add_event("liveness: starting combined BUGGIFY + liveness test");

        while start.elapsed() < duration {
            // Check liveness
            self.check_liveness_tick();

            // Apply BUGGIFY faults periodically
            if last_fault_instant.elapsed() >= fault_sleep {
                self.apply_buggify_faults().await;
                last_fault_instant = current_instant();
            }

            madsim::time::sleep(check_sleep).await;
        }

        // Final check
        self.check_liveness_tick();

        self.add_event("liveness: completed BUGGIFY + liveness test");

        self.generate_liveness_report(Ok(()))
    }

    /// Generate a liveness report from current state.
    fn generate_liveness_report(&mut self, test_result: Result<()>) -> LivenessReport {
        let is_passed = self.liveness_state.violations.is_empty() && test_result.is_ok();

        let summary = if is_passed {
            format!(
                "Liveness test PASSED: {} checks, {}ms leaderless total, {}ms max recovery",
                self.metrics.liveness.liveness_checks,
                self.metrics.liveness.leaderless_duration_ms,
                self.metrics.liveness.max_leader_recovery_ms
            )
        } else {
            let violation_count = self.liveness_state.violations.len();
            let test_error = test_result.as_ref().err().map(|e| e.to_string());
            format!("Liveness test FAILED: {} violations, test error: {:?}", violation_count, test_error)
        };

        self.add_event(format!("liveness: {}", summary));

        // Copy metrics for report
        let metrics_snapshot = self.metrics.liveness.clone();
        self.metrics.liveness = metrics_snapshot.clone();

        LivenessReport {
            passed: is_passed,
            mode: self.liveness_config.mode,
            metrics: metrics_snapshot,
            violations: self.liveness_state.violations.clone(),
            summary,
        }
    }

    /// Get current liveness metrics.
    pub fn liveness_metrics(&self) -> &LivenessMetrics {
        &self.metrics.liveness
    }

    /// Check if any liveness violations have occurred.
    pub fn has_liveness_violations(&self) -> bool {
        !self.liveness_state.violations.is_empty()
    }

    /// Get all liveness violations.
    pub fn liveness_violations(&self) -> &[LivenessViolation] {
        &self.liveness_state.violations
    }

    /// Perform a write with liveness tracking.
    ///
    /// If the write fails due to no leader, this is tracked as blocked time.
    pub async fn write_with_liveness(&mut self, key: String, value: String) -> Result<()> {
        let start = current_instant();
        let result = self.write(key.clone(), value).await;

        if result.is_ok() {
            self.metrics.liveness.writes_completed += 1;
        } else {
            let blocked_time_ms = duration_ms_u64(start.elapsed());
            self.metrics.liveness.writes_blocked += 1;
            self.metrics.liveness.blocked_duration_ms += blocked_time_ms;

            if self.liveness_state.active {
                let elapsed_ms = duration_ms_u64(current_instant().duration_since(self.start_time));
                self.liveness_state.violations.push(LivenessViolation {
                    started_at_ms: elapsed_ms.saturating_sub(blocked_time_ms),
                    duration_ms: blocked_time_ms,
                    violation_type: ViolationType::OperationBlocked,
                    context: format!("Write blocked for {}ms on key '{}'", blocked_time_ms, key),
                });
            }
        }

        result
    }
}
