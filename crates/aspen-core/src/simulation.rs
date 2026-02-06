//! Simulation artifact capture and persistence.
//!
//! This module provides utilities for capturing deterministic simulation data
//! (seeds, event traces, metrics) and persisting them to disk for debugging and
//! CI reporting.

use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

/// Complete snapshot of a simulation run, including seed, events, and metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationArtifact {
    /// Unique identifier for this simulation run.
    pub run_id: String,
    /// Timestamp when the simulation started.
    pub timestamp: DateTime<Utc>,
    /// Deterministic seed used for this simulation.
    pub seed: u64,
    /// Name of the test that produced this artifact.
    pub test_name: String,
    /// Event trace captured during the simulation.
    pub events: Vec<String>,
    /// Transport metrics snapshot.
    pub metrics: String,
    /// Whether the simulation passed or failed.
    pub status: SimulationStatus,
    /// Optional error message if the simulation failed.
    pub error: Option<String>,
    /// Simulation duration in milliseconds.
    pub duration_ms: u64,
}

/// Status of a simulation run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulationStatus {
    /// Simulation completed successfully without errors.
    Passed,
    /// Simulation failed with an error.
    Failed,
}

impl fmt::Display for SimulationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Passed => write!(f, "passed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl SimulationArtifact {
    /// Create a new simulation artifact with the given parameters.
    pub fn new(test_name: impl Into<String>, seed: u64, events: Vec<String>, metrics: String) -> Self {
        let test_name_str = test_name.into();
        let timestamp = Utc::now();
        let run_id = format!("{}-seed{}-{}", test_name_str, seed, timestamp.format("%Y%m%d-%H%M%S"));
        Self {
            run_id,
            timestamp,
            seed,
            test_name: test_name_str,
            events,
            metrics,
            status: SimulationStatus::Passed,
            error: None,
            duration_ms: 0,
        }
    }

    /// Mark the simulation as failed with an error message.
    pub fn with_failure(mut self, error: impl Into<String>) -> Self {
        self.status = SimulationStatus::Failed;
        self.error = Some(error.into());
        self
    }

    /// Set the simulation duration in milliseconds.
    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Persist this artifact to disk as JSON.
    ///
    /// The artifact will be written to `{base_dir}/{run_id}.json`.
    pub fn persist(&self, base_dir: impl AsRef<Path>) -> Result<PathBuf> {
        let base_dir = base_dir.as_ref();
        fs::create_dir_all(base_dir).context("failed to create simulation artifacts directory")?;

        let file_path = base_dir.join(format!("{}.json", self.run_id));
        let json = serde_json::to_string_pretty(self).context("failed to serialize simulation artifact")?;

        fs::write(&file_path, json).context("failed to write simulation artifact")?;

        Ok(file_path)
    }

    /// Load a simulation artifact from a JSON file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let contents = fs::read_to_string(path.as_ref()).context("failed to read simulation artifact")?;
        serde_json::from_str(&contents).context("failed to deserialize simulation artifact")
    }
}

/// Builder for constructing simulation artifacts with a fluent API.
pub struct SimulationArtifactBuilder {
    test_name: String,
    seed: u64,
    events: Vec<String>,
    metrics: String,
    status: SimulationStatus,
    error: Option<String>,
    start_time: Option<DateTime<Utc>>,
}

impl SimulationArtifactBuilder {
    /// Create a new builder with the test name and seed.
    pub fn new(test_name: impl Into<String>, seed: u64) -> Self {
        Self {
            test_name: test_name.into(),
            seed,
            events: Vec::new(),
            metrics: String::new(),
            status: SimulationStatus::Passed,
            error: None,
            start_time: None,
        }
    }

    /// Create a new builder with automatic seed selection.
    ///
    /// Seed selection priority:
    /// 1. `MADSIM_TEST_SEED` environment variable
    /// 2. `ASPEN_TEST_SEED` environment variable
    /// 3. Deterministic hash of test name
    ///
    /// This enables:
    /// - CI matrix testing with diverse seeds via environment variable
    /// - Reproducible failures: "MADSIM_TEST_SEED=42 cargo nextest run test_name"
    /// - Consistent default behavior based on test name
    pub fn new_with_auto_seed(test_name: impl Into<String>) -> (Self, u64) {
        use std::hash::Hash;
        use std::hash::Hasher;

        let test_name = test_name.into();

        // Priority order for seed selection
        let seed = std::env::var("MADSIM_TEST_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .or_else(|| std::env::var("ASPEN_TEST_SEED").ok().and_then(|s| s.parse().ok()))
            .unwrap_or_else(|| {
                // Deterministic seed from test name
                let mut hasher = std::hash::DefaultHasher::new();
                test_name.hash(&mut hasher);
                hasher.finish()
            });

        eprintln!("Test '{}' using seed: {}", test_name, seed);
        eprintln!("To reproduce: MADSIM_TEST_SEED={} cargo nextest run {}", seed, test_name);

        (Self::new(test_name, seed), seed)
    }

    /// Record the start time (call this at the beginning of the simulation).
    pub fn start(mut self) -> Self {
        self.start_time = Some(Utc::now());
        self
    }

    /// Add an event to the trace.
    pub fn add_event(mut self, event: impl Into<String>) -> Self {
        self.events.push(event.into());
        self
    }

    /// Add multiple events to the trace.
    pub fn add_events(mut self, events: impl IntoIterator<Item = String>) -> Self {
        self.events.extend(events);
        self
    }

    /// Set the metrics snapshot.
    pub fn with_metrics(mut self, metrics: impl Into<String>) -> Self {
        self.metrics = metrics.into();
        self
    }

    /// Mark the simulation as failed.
    pub fn fail(mut self, error: impl Into<String>) -> Self {
        self.status = SimulationStatus::Failed;
        self.error = Some(error.into());
        self
    }

    /// Build the final artifact.
    pub fn build(self) -> SimulationArtifact {
        let timestamp = self.start_time.unwrap_or_else(Utc::now);
        let run_id = format!("{}-seed{}-{}", self.test_name, self.seed, timestamp.format("%Y%m%d-%H%M%S"));

        let duration_ms = if let Some(start) = self.start_time {
            let elapsed = Utc::now().signed_duration_since(start);
            elapsed.num_milliseconds().max(0) as u64
        } else {
            0
        };

        SimulationArtifact {
            run_id,
            timestamp,
            seed: self.seed,
            test_name: self.test_name,
            events: self.events,
            metrics: self.metrics,
            status: self.status,
            error: self.error,
            duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_builder_creates_valid_artifact() {
        let artifact = SimulationArtifactBuilder::new("test_simulation", 42)
            .start()
            .add_event("event1")
            .add_event("event2")
            .with_metrics("counter{} 42")
            .build();

        assert_eq!(artifact.test_name, "test_simulation");
        assert_eq!(artifact.seed, 42);
        assert_eq!(artifact.events.len(), 2);
        assert_eq!(artifact.status, SimulationStatus::Passed);
        assert!(artifact.error.is_none());
    }

    #[test]
    fn artifact_builder_handles_failure() {
        let artifact = SimulationArtifactBuilder::new("test_simulation", 42).fail("something went wrong").build();

        assert_eq!(artifact.status, SimulationStatus::Failed);
        assert_eq!(artifact.error, Some("something went wrong".to_string()));
    }

    #[test]
    fn artifact_roundtrip_json() {
        let original =
            SimulationArtifact::new("test_simulation", 42, vec!["event1".into(), "event2".into()], "metrics".into());

        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: SimulationArtifact = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(original.test_name, deserialized.test_name);
        assert_eq!(original.seed, deserialized.seed);
        assert_eq!(original.events, deserialized.events);
        assert_eq!(original.metrics, deserialized.metrics);
    }

    // =========================================================================
    // SimulationStatus Tests
    // =========================================================================

    #[test]
    fn simulation_status_display_passed() {
        assert_eq!(format!("{}", SimulationStatus::Passed), "passed");
    }

    #[test]
    fn simulation_status_display_failed() {
        assert_eq!(format!("{}", SimulationStatus::Failed), "failed");
    }

    #[test]
    fn simulation_status_equality() {
        assert_eq!(SimulationStatus::Passed, SimulationStatus::Passed);
        assert_eq!(SimulationStatus::Failed, SimulationStatus::Failed);
        assert_ne!(SimulationStatus::Passed, SimulationStatus::Failed);
    }

    #[test]
    fn simulation_status_clone() {
        let status = SimulationStatus::Passed;
        let cloned = status;
        assert_eq!(status, cloned);
    }

    #[test]
    fn simulation_status_debug() {
        assert_eq!(format!("{:?}", SimulationStatus::Passed), "Passed");
        assert_eq!(format!("{:?}", SimulationStatus::Failed), "Failed");
    }

    // =========================================================================
    // SimulationArtifact Tests
    // =========================================================================

    #[test]
    fn artifact_new_creates_run_id() {
        let artifact = SimulationArtifact::new("my_test", 12345, vec![], String::new());

        assert!(artifact.run_id.contains("my_test"));
        assert!(artifact.run_id.contains("seed12345"));
    }

    #[test]
    fn artifact_new_sets_default_status() {
        let artifact = SimulationArtifact::new("test", 0, vec![], String::new());

        assert_eq!(artifact.status, SimulationStatus::Passed);
        assert!(artifact.error.is_none());
        assert_eq!(artifact.duration_ms, 0);
    }

    #[test]
    fn artifact_with_failure() {
        let artifact = SimulationArtifact::new("test", 0, vec![], String::new()).with_failure("test error");

        assert_eq!(artifact.status, SimulationStatus::Failed);
        assert_eq!(artifact.error, Some("test error".to_string()));
    }

    #[test]
    fn artifact_with_duration_ms() {
        let artifact = SimulationArtifact::new("test", 0, vec![], String::new()).with_duration_ms(5000);

        assert_eq!(artifact.duration_ms, 5000);
    }

    #[test]
    fn artifact_with_events() {
        let events = vec!["event1".to_string(), "event2".to_string(), "event3".to_string()];
        let artifact = SimulationArtifact::new("test", 0, events.clone(), String::new());

        assert_eq!(artifact.events, events);
    }

    #[test]
    fn artifact_with_metrics() {
        let metrics = "counter{label=\"value\"} 42\nhistogram_bucket{le=\"0.1\"} 10".to_string();
        let artifact = SimulationArtifact::new("test", 0, vec![], metrics.clone());

        assert_eq!(artifact.metrics, metrics);
    }

    #[test]
    fn artifact_persist_and_load() {
        let temp_dir = std::env::temp_dir().join(format!("aspen-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir); // Clean up any previous run

        let original =
            SimulationArtifact::new("persist_test", 999, vec!["persist_event".into()], "persist_metrics".into())
                .with_duration_ms(1234);

        let path = original.persist(&temp_dir).expect("persist should succeed");
        assert!(path.exists());

        let loaded = SimulationArtifact::load(&path).expect("load should succeed");

        assert_eq!(original.run_id, loaded.run_id);
        assert_eq!(original.seed, loaded.seed);
        assert_eq!(original.test_name, loaded.test_name);
        assert_eq!(original.events, loaded.events);
        assert_eq!(original.metrics, loaded.metrics);
        assert_eq!(original.status, loaded.status);
        assert_eq!(original.duration_ms, loaded.duration_ms);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn artifact_persist_creates_directory() {
        let temp_dir = std::env::temp_dir()
            .join(format!("aspen-test-nested-{}", std::process::id()))
            .join("nested")
            .join("path");

        let _ = fs::remove_dir_all(temp_dir.parent().unwrap().parent().unwrap());

        let artifact = SimulationArtifact::new("nested_test", 0, vec![], String::new());
        let path = artifact.persist(&temp_dir).expect("persist should create nested dirs");

        assert!(path.exists());

        // Cleanup
        let _ = fs::remove_dir_all(temp_dir.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn artifact_load_nonexistent_fails() {
        let result = SimulationArtifact::load("/nonexistent/path/artifact.json");
        assert!(result.is_err());
    }

    #[test]
    fn artifact_debug() {
        let artifact = SimulationArtifact::new("debug_test", 42, vec![], String::new());
        let debug = format!("{:?}", artifact);

        assert!(debug.contains("SimulationArtifact"));
        assert!(debug.contains("debug_test"));
        assert!(debug.contains("42"));
    }

    #[test]
    fn artifact_clone() {
        let original = SimulationArtifact::new("clone_test", 777, vec!["event".into()], "metrics".into())
            .with_failure("error")
            .with_duration_ms(100);

        let cloned = original.clone();

        assert_eq!(original.run_id, cloned.run_id);
        assert_eq!(original.seed, cloned.seed);
        assert_eq!(original.events, cloned.events);
        assert_eq!(original.status, cloned.status);
        assert_eq!(original.error, cloned.error);
    }

    // =========================================================================
    // SimulationArtifactBuilder Tests
    // =========================================================================

    #[test]
    fn builder_new() {
        let builder = SimulationArtifactBuilder::new("builder_test", 123);
        let artifact = builder.build();

        assert_eq!(artifact.test_name, "builder_test");
        assert_eq!(artifact.seed, 123);
        assert_eq!(artifact.status, SimulationStatus::Passed);
    }

    #[test]
    fn builder_add_events() {
        let artifact = SimulationArtifactBuilder::new("test", 0)
            .add_events(vec!["e1".into(), "e2".into(), "e3".into()])
            .build();

        assert_eq!(artifact.events.len(), 3);
        assert_eq!(artifact.events[0], "e1");
        assert_eq!(artifact.events[2], "e3");
    }

    #[test]
    fn builder_chained_add_event() {
        let artifact = SimulationArtifactBuilder::new("test", 0)
            .add_event("first")
            .add_event("second")
            .add_event("third")
            .build();

        assert_eq!(artifact.events, vec!["first", "second", "third"]);
    }

    #[test]
    fn builder_with_metrics() {
        let artifact = SimulationArtifactBuilder::new("test", 0).with_metrics("custom_metric 100").build();

        assert_eq!(artifact.metrics, "custom_metric 100");
    }

    #[test]
    fn builder_fail() {
        let artifact = SimulationArtifactBuilder::new("test", 0).fail("builder failure").build();

        assert_eq!(artifact.status, SimulationStatus::Failed);
        assert_eq!(artifact.error, Some("builder failure".to_string()));
    }

    #[test]
    fn builder_start_records_time() {
        let artifact = SimulationArtifactBuilder::new("test", 0).start().build();

        // Duration should be very small (just the time to build)
        assert!(artifact.duration_ms < 1000);
    }

    #[test]
    fn builder_without_start_has_zero_duration() {
        let artifact = SimulationArtifactBuilder::new("test", 0).build();

        assert_eq!(artifact.duration_ms, 0);
    }

    #[test]
    fn builder_full_chain() {
        let artifact = SimulationArtifactBuilder::new("full_chain_test", 9999)
            .start()
            .add_event("init")
            .add_events(vec!["process".into(), "complete".into()])
            .with_metrics("duration_ms 500")
            .build();

        assert_eq!(artifact.test_name, "full_chain_test");
        assert_eq!(artifact.seed, 9999);
        assert_eq!(artifact.events.len(), 3);
        assert_eq!(artifact.metrics, "duration_ms 500");
        assert_eq!(artifact.status, SimulationStatus::Passed);
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn artifact_empty_events() {
        let artifact = SimulationArtifact::new("empty", 0, vec![], String::new());
        assert!(artifact.events.is_empty());
    }

    #[test]
    fn artifact_large_event_trace() {
        let events: Vec<String> = (0..10000).map(|i| format!("event_{}", i)).collect();
        let artifact = SimulationArtifact::new("large_trace", 0, events.clone(), String::new());

        assert_eq!(artifact.events.len(), 10000);
        assert_eq!(artifact.events[0], "event_0");
        assert_eq!(artifact.events[9999], "event_9999");
    }

    #[test]
    fn artifact_special_characters_in_name() {
        let artifact = SimulationArtifact::new("test::nested::module", 0, vec![], String::new());

        assert!(artifact.run_id.contains("test::nested::module"));
    }

    #[test]
    fn artifact_max_seed() {
        let artifact = SimulationArtifact::new("max_seed", u64::MAX, vec![], String::new());

        assert_eq!(artifact.seed, u64::MAX);
        assert!(artifact.run_id.contains(&format!("seed{}", u64::MAX)));
    }

    #[test]
    fn artifact_zero_seed() {
        let artifact = SimulationArtifact::new("zero_seed", 0, vec![], String::new());

        assert_eq!(artifact.seed, 0);
        assert!(artifact.run_id.contains("seed0"));
    }

    #[test]
    fn artifact_serialization_preserves_failed_status() {
        let original = SimulationArtifact::new("test", 0, vec![], String::new()).with_failure("test failure");

        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: SimulationArtifact = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.status, SimulationStatus::Failed);
        assert_eq!(deserialized.error, Some("test failure".to_string()));
    }

    #[test]
    fn artifact_postcard_serialization() {
        let original = SimulationArtifact::new("postcard_test", 42, vec!["event".into()], "metrics".into());

        let bytes = postcard::to_allocvec(&original).expect("postcard serialize");
        let deserialized: SimulationArtifact = postcard::from_bytes(&bytes).expect("postcard deserialize");

        assert_eq!(original.seed, deserialized.seed);
        assert_eq!(original.test_name, deserialized.test_name);
        assert_eq!(original.events, deserialized.events);
    }
}
