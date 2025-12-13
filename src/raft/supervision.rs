//! Supervision configuration (legacy stub).
//!
//! This module provides a minimal `SupervisionConfig` for backward compatibility
//! with existing configuration files and tests. The actual actor-based supervision
//! has been replaced by `Supervisor` in `supervisor.rs`.
//!
//! # Migration
//!
//! New code should use `bootstrap::bootstrap_node()` directly.

use serde::{Deserialize, Serialize};

/// Configuration for Raft actor supervision (deprecated).
///
/// This struct is kept for backward compatibility with existing configuration
/// files. The actual supervision is now handled by `Supervisor`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisionConfig {
    /// Maximum number of restarts before giving up.
    #[serde(default = "default_max_restart_count")]
    pub max_restart_count: u32,

    /// Health check interval in milliseconds.
    #[serde(default = "default_health_check_interval_ms")]
    pub health_check_interval_ms: u64,

    /// Restart backoff base in milliseconds.
    #[serde(default = "default_restart_backoff_base_ms")]
    pub restart_backoff_base_ms: u64,

    /// Maximum restart backoff in milliseconds.
    #[serde(default = "default_restart_backoff_max_ms")]
    pub restart_backoff_max_ms: u64,
}

impl Default for SupervisionConfig {
    fn default() -> Self {
        Self {
            max_restart_count: default_max_restart_count(),
            health_check_interval_ms: default_health_check_interval_ms(),
            restart_backoff_base_ms: default_restart_backoff_base_ms(),
            restart_backoff_max_ms: default_restart_backoff_max_ms(),
        }
    }
}

fn default_max_restart_count() -> u32 {
    3
}

fn default_health_check_interval_ms() -> u64 {
    5000
}

fn default_restart_backoff_base_ms() -> u64 {
    1000
}

fn default_restart_backoff_max_ms() -> u64 {
    30000
}
