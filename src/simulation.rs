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
}
