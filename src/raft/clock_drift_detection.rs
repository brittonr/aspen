//! Clock drift detection between Raft cluster nodes.
//!
//! Uses NTP-style 4-timestamp approach to estimate clock offset while
//! accounting for network latency. This is purely observational and does
//! NOT affect Raft consensus correctness.
//!
//! # Why Clock Drift Detection?
//!
//! Raft consensus does NOT require synchronized clocks - it uses:
//! - Logical ordering (term numbers + log indices) instead of wall-clock time
//! - Monotonic clocks (`std::time::Instant`) for election/heartbeat timeouts
//!
//! However, detecting clock drift is useful for operational health:
//! - TLS certificate validation (iroh QUIC) depends on wall-clock time
//! - Log timestamps become confusing when debugging cross-node issues
//! - Metrics dashboards show skewed data
//!
//! # NTP-Style Offset Calculation
//!
//! During each RPC exchange, we capture 4 timestamps:
//! - `t1`: Client sends request (wall-clock time)
//! - `t2`: Server receives request (wall-clock time)
//! - `t3`: Server sends response (wall-clock time)
//! - `t4`: Client receives response (wall-clock time)
//!
//! The clock offset is estimated as: `offset = ((t2 - t1) + (t3 - t4)) / 2`
//!
//! This formula accounts for network round-trip time by averaging the
//! apparent offset from both directions.
//!
//! # Tiger Style
//!
//! - Bounded storage: MAX_DRIFT_OBSERVATIONS limits tracked nodes
//! - Explicit thresholds: CLOCK_DRIFT_WARNING_THRESHOLD_MS, CLOCK_DRIFT_ALERT_THRESHOLD_MS
//! - Fail-safe: Never blocks consensus, purely observational
//! - EWMA smoothing: Reduces noise from individual measurements

use std::collections::HashMap;
use std::time::Instant;

use tracing::{error, info, warn};

use crate::raft::constants::{
    CLOCK_DRIFT_ALERT_THRESHOLD_MS, CLOCK_DRIFT_WARNING_THRESHOLD_MS, DRIFT_EWMA_ALPHA,
    MAX_DRIFT_OBSERVATIONS, MIN_DRIFT_OBSERVATIONS,
};
use crate::raft::types::NodeId;

/// Severity level of detected clock drift.
///
/// Indicates how far the estimated clock offset is from ideal (0ms).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftSeverity {
    /// Drift within acceptable range (< warning threshold).
    Normal,
    /// Drift exceeds warning threshold but below alert.
    /// Indicates NTP may need attention.
    Warning,
    /// Drift exceeds alert threshold.
    /// Indicates significant NTP misconfiguration.
    Alert,
}

/// Observation state for a single peer's clock drift.
struct DriftObservation {
    /// Most recent raw offset measurement (signed milliseconds).
    /// Positive = peer clock is ahead of local clock.
    /// Negative = peer clock is behind local clock.
    last_offset_ms: i64,
    /// Exponentially weighted moving average of offset.
    /// Smooths out measurement noise.
    ewma_offset_ms: f64,
    /// Number of observations recorded.
    observation_count: u64,
    /// Timestamp of last observation (for staleness detection).
    last_observed_at: Instant,
    /// Current severity classification.
    severity: DriftSeverity,
}

impl DriftObservation {
    fn new(offset_ms: i64, warning_threshold_ms: u64, alert_threshold_ms: u64) -> Self {
        use crate::raft::pure::classify_drift_severity;

        let severity =
            classify_drift_severity(offset_ms as f64, warning_threshold_ms, alert_threshold_ms);
        Self {
            last_offset_ms: offset_ms,
            ewma_offset_ms: offset_ms as f64,
            observation_count: 1,
            last_observed_at: Instant::now(),
            severity,
        }
    }

    /// Update observation with new measurement using EWMA.
    fn update(&mut self, offset_ms: i64, warning_threshold_ms: u64, alert_threshold_ms: u64) {
        use crate::raft::pure::{classify_drift_severity, compute_ewma};

        self.last_offset_ms = offset_ms;
        // EWMA: new_avg = alpha * new_value + (1 - alpha) * old_avg
        self.ewma_offset_ms = compute_ewma(offset_ms as f64, self.ewma_offset_ms, DRIFT_EWMA_ALPHA);
        self.observation_count = self.observation_count.saturating_add(1);
        self.last_observed_at = Instant::now();
        self.severity = classify_drift_severity(
            self.ewma_offset_ms,
            warning_threshold_ms,
            alert_threshold_ms,
        );
    }
}

/// Clock drift detector for monitoring peer clock synchronization.
///
/// Tracks estimated clock offsets for each peer node based on RPC timestamps.
/// Emits warnings/alerts when drift exceeds configured thresholds.
///
/// This detector is purely observational and does NOT affect Raft consensus.
pub struct ClockDriftDetector {
    /// Map of NodeId -> drift observation state.
    observations: HashMap<NodeId, DriftObservation>,
    /// Warning threshold in milliseconds.
    warning_threshold_ms: u64,
    /// Alert threshold in milliseconds.
    alert_threshold_ms: u64,
}

impl Default for ClockDriftDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ClockDriftDetector {
    /// Create a new clock drift detector with default thresholds.
    pub fn new() -> Self {
        Self {
            observations: HashMap::new(),
            warning_threshold_ms: CLOCK_DRIFT_WARNING_THRESHOLD_MS,
            alert_threshold_ms: CLOCK_DRIFT_ALERT_THRESHOLD_MS,
        }
    }

    /// Create a clock drift detector with custom thresholds.
    pub fn with_thresholds(warning_threshold_ms: u64, alert_threshold_ms: u64) -> Self {
        Self {
            observations: HashMap::new(),
            warning_threshold_ms,
            alert_threshold_ms,
        }
    }

    /// Record a new drift observation from an RPC timestamp exchange.
    ///
    /// # NTP-Style Calculation
    ///
    /// Given 4 timestamps (all in milliseconds since UNIX epoch):
    /// - `client_send_ms` (t1): When client sent the request
    /// - `server_recv_ms` (t2): When server received the request
    /// - `server_send_ms` (t3): When server sent the response
    /// - `client_recv_ms` (t4): When client received the response
    ///
    /// The estimated clock offset is: `((t2 - t1) + (t3 - t4)) / 2`
    ///
    /// This accounts for network round-trip time (RTT) by averaging the
    /// apparent offset from both request and response directions.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The peer node whose clock offset is being measured
    /// * `client_send_ms` - t1: Client request send time (wall-clock ms)
    /// * `server_recv_ms` - t2: Server request receive time (wall-clock ms)
    /// * `server_send_ms` - t3: Server response send time (wall-clock ms)
    /// * `client_recv_ms` - t4: Client response receive time (wall-clock ms)
    pub fn record_observation(
        &mut self,
        node_id: NodeId,
        client_send_ms: u64,
        server_recv_ms: u64,
        server_send_ms: u64,
        client_recv_ms: u64,
    ) {
        use crate::raft::pure::calculate_ntp_clock_offset;

        // Calculate clock offset and RTT using NTP formula (extracted pure function)
        let (offset_ms, rtt_ms) = calculate_ntp_clock_offset(
            client_send_ms,
            server_recv_ms,
            server_send_ms,
            client_recv_ms,
        );

        // Tiger Style: Bounded storage
        if self.observations.len() >= MAX_DRIFT_OBSERVATIONS as usize
            && !self.observations.contains_key(&node_id)
        {
            // Remove oldest observation to make room
            if let Some(oldest_id) = self.find_oldest_observation() {
                self.observations.remove(&oldest_id);
            }
        }

        let prev_severity = self.observations.get(&node_id).map(|o| o.severity);

        // Update or insert observation, capturing new severity for logging
        let (new_severity, obs_count, ewma_offset) =
            if let Some(obs) = self.observations.get_mut(&node_id) {
                obs.update(
                    offset_ms,
                    self.warning_threshold_ms,
                    self.alert_threshold_ms,
                );
                (obs.severity, obs.observation_count, obs.ewma_offset_ms)
            } else {
                let obs = DriftObservation::new(
                    offset_ms,
                    self.warning_threshold_ms,
                    self.alert_threshold_ms,
                );
                let severity = obs.severity;
                let count = obs.observation_count;
                let ewma = obs.ewma_offset_ms;
                self.observations.insert(node_id, obs);
                (severity, count, ewma)
            };

        // Only log on severity transitions (to avoid log spam)
        // Also require minimum observations to reduce false positives
        if obs_count >= MIN_DRIFT_OBSERVATIONS as u64 && prev_severity != Some(new_severity) {
            match new_severity {
                DriftSeverity::Alert => {
                    error!(
                        node_id = %node_id,
                        offset_ms = ewma_offset as i64,
                        rtt_ms = rtt_ms,
                        observation_count = obs_count,
                        "ALERT: significant clock drift detected - check NTP synchronization"
                    );
                }
                DriftSeverity::Warning => {
                    warn!(
                        node_id = %node_id,
                        offset_ms = ewma_offset as i64,
                        rtt_ms = rtt_ms,
                        observation_count = obs_count,
                        "clock drift detected: offset exceeds {}ms warning threshold",
                        self.warning_threshold_ms
                    );
                }
                DriftSeverity::Normal => {
                    if prev_severity.is_some() {
                        info!(
                            node_id = %node_id,
                            offset_ms = ewma_offset as i64,
                            "clock drift returned to normal"
                        );
                    }
                }
            }
        }
    }

    /// Find the node ID with the oldest observation timestamp.
    fn find_oldest_observation(&self) -> Option<NodeId> {
        self.observations
            .iter()
            .min_by_key(|(_, obs)| obs.last_observed_at)
            .map(|(id, _)| *id)
    }

    /// Get nodes currently showing concerning drift (Warning or Alert level).
    ///
    /// Returns a list of (NodeId, DriftSeverity, estimated_offset_ms).
    /// Only includes nodes with sufficient observations to be reliable.
    pub fn get_nodes_with_drift(&self) -> Vec<(NodeId, DriftSeverity, i64)> {
        self.observations
            .iter()
            .filter(|(_, obs)| {
                obs.observation_count >= MIN_DRIFT_OBSERVATIONS as u64
                    && obs.severity != DriftSeverity::Normal
            })
            .map(|(id, obs)| (*id, obs.severity, obs.ewma_offset_ms as i64))
            .collect()
    }

    /// Get current drift estimate for a specific node.
    ///
    /// Returns `Some((severity, offset_ms))` if the node has been observed,
    /// `None` otherwise.
    pub fn get_drift(&self, node_id: NodeId) -> Option<(DriftSeverity, i64)> {
        self.observations
            .get(&node_id)
            .map(|obs| (obs.severity, obs.ewma_offset_ms as i64))
    }

    /// Get summary statistics for monitoring.
    pub fn get_summary(&self) -> ClockDriftSummary {
        let mut warnings = 0;
        let mut alerts = 0;
        let mut max_drift_ms: i64 = 0;

        for obs in self.observations.values() {
            if obs.observation_count < MIN_DRIFT_OBSERVATIONS as u64 {
                continue;
            }
            match obs.severity {
                DriftSeverity::Warning => warnings += 1,
                DriftSeverity::Alert => alerts += 1,
                DriftSeverity::Normal => {}
            }
            let abs_drift = obs.ewma_offset_ms.abs() as i64;
            if abs_drift > max_drift_ms.abs() {
                max_drift_ms = obs.ewma_offset_ms as i64;
            }
        }

        ClockDriftSummary {
            nodes_with_warning: warnings,
            nodes_with_alert: alerts,
            max_drift_ms,
            total_nodes_observed: self.observations.len() as u32,
        }
    }
}

/// Summary statistics for clock drift monitoring.
#[derive(Debug, Clone, Copy)]
pub struct ClockDriftSummary {
    /// Number of nodes with Warning-level drift.
    pub nodes_with_warning: u32,
    /// Number of nodes with Alert-level drift.
    pub nodes_with_alert: u32,
    /// Maximum observed drift (signed, in milliseconds).
    pub max_drift_ms: i64,
    /// Total number of nodes being tracked.
    pub total_nodes_observed: u32,
}

/// Get current wall-clock time as milliseconds since UNIX epoch.
///
/// Used for timestamping RPC requests/responses for drift detection.
pub fn current_time_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_drift_calculation() {
        let mut detector = ClockDriftDetector::new();

        // Simulate an RPC with known timestamps
        // Server clock is 50ms ahead of client
        // RTT is 100ms
        let t1 = 1000; // Client send
        let t2 = 1100; // Server recv (50ms server offset + 50ms network delay)
        let t3 = 1150; // Server send (50ms processing)
        let t4 = 1200; // Client recv (50ms network delay back)

        detector.record_observation(NodeId(1), t1, t2, t3, t4);

        let (severity, offset) = detector.get_drift(NodeId(1)).unwrap();
        // Expected offset: ((1100-1000) + (1150-1200)) / 2 = (100 + (-50)) / 2 = 25ms
        // This is close to the actual 50ms offset, reduced by RTT asymmetry estimation
        assert_eq!(offset, 25);
        assert_eq!(severity, DriftSeverity::Normal);
    }

    #[test]
    fn test_severity_classification() {
        let mut detector = ClockDriftDetector::new();

        // Create observation with 150ms drift (above warning, below alert)
        // Need multiple observations to trigger logging
        for _ in 0..MIN_DRIFT_OBSERVATIONS {
            let t1 = 1000;
            let t2 = 1300; // 300ms apparent offset
            let t3 = 1350;
            let t4 = 1200;
            detector.record_observation(NodeId(1), t1, t2, t3, t4);
        }

        let (severity, _) = detector.get_drift(NodeId(1)).unwrap();
        // Offset = ((300) + (150)) / 2 = 225ms -> Warning
        assert_eq!(severity, DriftSeverity::Warning);
    }

    #[test]
    fn test_ewma_smoothing() {
        let mut detector = ClockDriftDetector::new();

        // First observation: 0ms drift
        detector.record_observation(NodeId(1), 1000, 1000, 1000, 1000);
        let (_, offset1) = detector.get_drift(NodeId(1)).unwrap();
        assert_eq!(offset1, 0);

        // Second observation: 100ms drift
        // EWMA = 0.1 * 100 + 0.9 * 0 = 10
        detector.record_observation(NodeId(1), 1000, 1100, 1100, 1000);
        let (_, offset2) = detector.get_drift(NodeId(1)).unwrap();
        assert!(offset2 > 0 && offset2 < 100, "EWMA should smooth the value");
    }

    #[test]
    fn test_bounded_storage() {
        let mut detector = ClockDriftDetector::new();

        // Add MAX_DRIFT_OBSERVATIONS + 1 nodes
        for node_id in 0..=(MAX_DRIFT_OBSERVATIONS as u64) {
            detector.record_observation(NodeId(node_id), 1000, 1000, 1000, 1000);
        }

        // Should not exceed the limit
        assert!(detector.observations.len() <= MAX_DRIFT_OBSERVATIONS as usize);
    }

    #[test]
    fn test_get_nodes_with_drift() {
        let mut detector = ClockDriftDetector::new();

        // Add normal node
        for _ in 0..MIN_DRIFT_OBSERVATIONS {
            detector.record_observation(NodeId(1), 1000, 1000, 1000, 1000);
        }

        // Add warning node (150ms drift)
        for _ in 0..MIN_DRIFT_OBSERVATIONS {
            detector.record_observation(NodeId(2), 1000, 1300, 1350, 1200);
        }

        let drifting = detector.get_nodes_with_drift();
        assert_eq!(drifting.len(), 1);
        assert_eq!(drifting[0].0, NodeId(2));
        assert_eq!(drifting[0].1, DriftSeverity::Warning);
    }
}
