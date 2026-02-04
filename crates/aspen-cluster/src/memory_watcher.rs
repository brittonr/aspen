//! Memory pressure detection for CI job execution.
//!
//! This module monitors system memory usage and provides pressure levels
//! that can be used to make scheduling decisions. When memory pressure
//! is high, workers can pause job dequeuing and the orchestrator can
//! cancel non-essential pipelines.
//!
//! # Architecture
//!
//! The `MemoryWatcher` polls `/proc/meminfo` at configurable intervals
//! and maintains a current pressure level. Consumers can check the level
//! before starting resource-intensive operations.
//!
//! # Pressure Levels
//!
//! - `Normal`: Memory usage below warning threshold (80%)
//! - `Warning`: Memory usage between 80-90%
//! - `Critical`: Memory usage above 90%
//!
//! # Tiger Style
//!
//! - Fixed thresholds from `aspen_core`
//! - Bounded polling interval
//! - Non-blocking pressure checks

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;

use aspen_core::MEMORY_PRESSURE_CRITICAL_PERCENT;
use aspen_core::MEMORY_PRESSURE_WARNING_PERCENT;
use aspen_core::MEMORY_WATCHER_INTERVAL_MS;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::watch;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Memory pressure level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPressureLevel {
    /// Normal memory usage - below warning threshold.
    Normal,
    /// Warning level - memory usage is elevated.
    Warning,
    /// Critical level - memory usage is dangerously high.
    Critical,
}

impl MemoryPressureLevel {
    /// Check if jobs should be paused at this level.
    pub fn should_pause_jobs(&self) -> bool {
        matches!(self, Self::Critical)
    }

    /// Check if non-essential operations should be cancelled.
    pub fn should_cancel_non_essential(&self) -> bool {
        matches!(self, Self::Critical)
    }

    /// Get the pressure level as a percentage threshold.
    pub fn as_percent(&self) -> u64 {
        match self {
            Self::Normal => 0,
            Self::Warning => MEMORY_PRESSURE_WARNING_PERCENT,
            Self::Critical => MEMORY_PRESSURE_CRITICAL_PERCENT,
        }
    }
}

/// Memory statistics from the system.
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Total memory in bytes.
    pub total_bytes: u64,
    /// Available memory in bytes.
    pub available_bytes: u64,
    /// Used memory in bytes.
    pub used_bytes: u64,
    /// Memory usage percentage (0-100).
    pub usage_percent: u64,
}

impl MemoryStats {
    /// Calculate pressure level based on usage.
    pub fn pressure_level(&self) -> MemoryPressureLevel {
        if self.usage_percent >= MEMORY_PRESSURE_CRITICAL_PERCENT {
            MemoryPressureLevel::Critical
        } else if self.usage_percent >= MEMORY_PRESSURE_WARNING_PERCENT {
            MemoryPressureLevel::Warning
        } else {
            MemoryPressureLevel::Normal
        }
    }
}

/// Watches system memory and provides pressure levels.
pub struct MemoryWatcher {
    /// Current memory usage percent (0-100).
    usage_percent: Arc<AtomicU64>,
    /// Sender for shutdown signal.
    shutdown_tx: watch::Sender<bool>,
    /// Receiver for shutdown signal.
    shutdown_rx: watch::Receiver<bool>,
    /// Polling interval.
    interval: Duration,
}

impl MemoryWatcher {
    /// Create a new memory watcher with default interval.
    pub fn new() -> Self {
        Self::with_interval(Duration::from_millis(MEMORY_WATCHER_INTERVAL_MS))
    }

    /// Create a memory watcher with custom interval.
    pub fn with_interval(interval: Duration) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Self {
            usage_percent: Arc::new(AtomicU64::new(0)),
            shutdown_tx,
            shutdown_rx,
            interval,
        }
    }

    /// Start the memory watcher background task.
    ///
    /// This spawns a task that polls memory usage at the configured interval.
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let usage_percent = self.usage_percent.clone();
        let interval = self.interval;
        let mut shutdown_rx = self.shutdown_rx.clone();

        tokio::spawn(async move {
            info!(interval_ms = interval.as_millis(), "memory watcher started");

            loop {
                // Check for shutdown
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            info!("memory watcher shutting down");
                            break;
                        }
                    }
                    _ = tokio::time::sleep(interval) => {
                        // Read memory stats
                        match read_memory_stats() {
                            Ok(stats) => {
                                let prev = usage_percent.swap(stats.usage_percent, Ordering::SeqCst);
                                let level = stats.pressure_level();

                                // Log level changes
                                let prev_level = if prev >= MEMORY_PRESSURE_CRITICAL_PERCENT {
                                    MemoryPressureLevel::Critical
                                } else if prev >= MEMORY_PRESSURE_WARNING_PERCENT {
                                    MemoryPressureLevel::Warning
                                } else {
                                    MemoryPressureLevel::Normal
                                };

                                if level != prev_level {
                                    match level {
                                        MemoryPressureLevel::Critical => {
                                            warn!(
                                                usage_percent = stats.usage_percent,
                                                available_mb = stats.available_bytes / (1024 * 1024),
                                                "memory pressure CRITICAL"
                                            );
                                        }
                                        MemoryPressureLevel::Warning => {
                                            warn!(
                                                usage_percent = stats.usage_percent,
                                                available_mb = stats.available_bytes / (1024 * 1024),
                                                "memory pressure warning"
                                            );
                                        }
                                        MemoryPressureLevel::Normal => {
                                            info!(
                                                usage_percent = stats.usage_percent,
                                                "memory pressure returned to normal"
                                            );
                                        }
                                    }
                                }

                                debug!(
                                    usage_percent = stats.usage_percent,
                                    level = ?level,
                                    "memory stats updated"
                                );
                            }
                            Err(e) => {
                                warn!(error = %e, "failed to read memory stats");
                            }
                        }
                    }
                }
            }
        })
    }

    /// Get the current memory pressure level.
    pub fn get_pressure_level(&self) -> MemoryPressureLevel {
        let percent = self.usage_percent.load(Ordering::SeqCst);
        if percent >= MEMORY_PRESSURE_CRITICAL_PERCENT {
            MemoryPressureLevel::Critical
        } else if percent >= MEMORY_PRESSURE_WARNING_PERCENT {
            MemoryPressureLevel::Warning
        } else {
            MemoryPressureLevel::Normal
        }
    }

    /// Get the current memory usage percentage.
    pub fn get_usage_percent(&self) -> u64 {
        self.usage_percent.load(Ordering::SeqCst)
    }

    /// Get current memory statistics.
    pub fn get_stats(&self) -> Result<MemoryStats, std::io::Error> {
        read_memory_stats()
    }

    /// Stop the memory watcher.
    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    /// Check if memory pressure allows job execution.
    ///
    /// Returns true if jobs should be allowed to run, false if they should wait.
    pub fn allow_jobs(&self) -> bool {
        !self.get_pressure_level().should_pause_jobs()
    }
}

impl Default for MemoryWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MemoryWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Read memory statistics from /proc/meminfo.
fn read_memory_stats() -> Result<MemoryStats, std::io::Error> {
    let content = std::fs::read_to_string("/proc/meminfo")?;

    let mut total_kb: u64 = 0;
    let mut available_kb: u64 = 0;

    for line in content.lines() {
        if let Some(value) = line.strip_prefix("MemTotal:") {
            total_kb = parse_meminfo_value(value);
        } else if let Some(value) = line.strip_prefix("MemAvailable:") {
            available_kb = parse_meminfo_value(value);
        }
    }

    let total_bytes = total_kb * 1024;
    let available_bytes = available_kb * 1024;
    let used_bytes = total_bytes.saturating_sub(available_bytes);
    let usage_percent = if total_bytes > 0 {
        (used_bytes * 100) / total_bytes
    } else {
        0
    };

    Ok(MemoryStats {
        total_bytes,
        available_bytes,
        used_bytes,
        usage_percent,
    })
}

/// Parse a meminfo value line (e.g., "  16384 kB").
fn parse_meminfo_value(value: &str) -> u64 {
    value.split_whitespace().next().and_then(|v| v.parse().ok()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pressure_level_from_percent() {
        let mut stats = MemoryStats {
            total_bytes: 100 * 1024 * 1024 * 1024,    // 100 GB
            available_bytes: 50 * 1024 * 1024 * 1024, // 50 GB
            used_bytes: 50 * 1024 * 1024 * 1024,
            usage_percent: 50,
        };

        assert_eq!(stats.pressure_level(), MemoryPressureLevel::Normal);

        stats.usage_percent = 85;
        assert_eq!(stats.pressure_level(), MemoryPressureLevel::Warning);

        stats.usage_percent = 95;
        assert_eq!(stats.pressure_level(), MemoryPressureLevel::Critical);
    }

    #[test]
    fn test_should_pause_jobs() {
        assert!(!MemoryPressureLevel::Normal.should_pause_jobs());
        assert!(!MemoryPressureLevel::Warning.should_pause_jobs());
        assert!(MemoryPressureLevel::Critical.should_pause_jobs());
    }

    #[test]
    fn test_parse_meminfo_value() {
        assert_eq!(parse_meminfo_value("  16384 kB"), 16384);
        assert_eq!(parse_meminfo_value("16384"), 16384);
        assert_eq!(parse_meminfo_value(""), 0);
        assert_eq!(parse_meminfo_value("invalid"), 0);
    }

    #[test]
    fn test_memory_watcher_default() {
        let watcher = MemoryWatcher::new();
        // Initially should be normal (0% usage before first poll)
        assert_eq!(watcher.get_pressure_level(), MemoryPressureLevel::Normal);
        assert!(watcher.allow_jobs());
    }
}
