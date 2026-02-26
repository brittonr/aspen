//! Cluster-level configuration types.
//!
//! Contains Raft timing profiles and control plane backend selection.

use std::str::FromStr;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use super::NodeConfig;

/// Raft timing profile for different deployment scenarios.
///
/// These profiles adjust heartbeat interval and election timeouts for
/// different operational requirements. Faster profiles enable quicker
/// leader failover but may cause false elections in high-latency networks.
///
/// # Constraint
///
/// All profiles maintain: `heartbeat_interval < election_timeout_min / 3`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RaftTimingProfile {
    /// Default conservative settings for production deployments.
    ///
    /// - heartbeat: 500ms
    /// - election_min: 1500ms
    /// - election_max: 3000ms
    ///
    /// Good for: Cloud deployments, cross-region clusters, high-latency networks.
    #[default]
    Conservative,

    /// Balanced settings for most local/LAN deployments.
    ///
    /// - heartbeat: 100ms
    /// - election_min: 500ms
    /// - election_max: 1000ms
    ///
    /// Good for: LAN clusters, single-datacenter deployments.
    Balanced,

    /// Aggressive settings for fast failover in low-latency environments.
    ///
    /// - heartbeat: 30ms
    /// - election_min: 100ms
    /// - election_max: 200ms
    ///
    /// Good for: Testing, single-machine development, co-located nodes.
    /// Warning: May cause unnecessary elections in high-latency networks.
    Fast,
}

impl std::str::FromStr for RaftTimingProfile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "conservative" => Ok(Self::Conservative),
            "balanced" => Ok(Self::Balanced),
            "fast" => Ok(Self::Fast),
            _ => Err(format!("invalid raft timing profile: '{}' (expected: conservative, balanced, fast)", s)),
        }
    }
}

impl RaftTimingProfile {
    /// Get the heartbeat interval for this profile in milliseconds.
    pub fn heartbeat_interval_ms(&self) -> u64 {
        match self {
            Self::Conservative => 500,
            Self::Balanced => 100,
            Self::Fast => 30,
        }
    }

    /// Get the minimum election timeout for this profile in milliseconds.
    pub fn election_timeout_min_ms(&self) -> u64 {
        match self {
            Self::Conservative => 1500,
            Self::Balanced => 500,
            Self::Fast => 100,
        }
    }

    /// Get the maximum election timeout for this profile in milliseconds.
    pub fn election_timeout_max_ms(&self) -> u64 {
        match self {
            Self::Conservative => 3000,
            Self::Balanced => 1000,
            Self::Fast => 200,
        }
    }

    /// Apply this profile's timing values to a NodeConfig.
    pub fn apply_to(&self, config: &mut NodeConfig) {
        config.heartbeat_interval_ms = self.heartbeat_interval_ms();
        config.election_timeout_min_ms = self.election_timeout_min_ms();
        config.election_timeout_max_ms = self.election_timeout_max_ms();
    }
}

/// Control-plane backend implementation.
///
/// Selects which implementation handles cluster consensus and coordination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ControlBackend {
    /// Deterministic in-memory implementation for testing.
    ///
    /// Uses an in-memory store without network communication, allowing
    /// for fast and reproducible tests.
    Deterministic,
    /// Production Raft-backed implementation.
    ///
    /// Uses openraft consensus with persistent storage and network
    /// communication via Iroh.
    #[default]
    Raft,
}

impl FromStr for ControlBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "deterministic" => Ok(ControlBackend::Deterministic),
            "raft" | "raft_actor" | "raftactor" => Ok(ControlBackend::Raft),
            _ => Err(format!("invalid control backend: {}", s)),
        }
    }
}
