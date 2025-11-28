//! Shared cleanup configuration and utilities for preventing memory leaks
//!
//! This module provides configurable TTL and size-based cleanup for execution tracking
//! HashMaps across all adapter implementations.

use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Configuration for execution state cleanup to prevent unbounded memory growth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupConfig {
    /// Time-to-live for completed executions (default: 24 hours)
    pub completed_ttl: Duration,

    /// Time-to-live for failed executions (default: 24 hours)
    pub failed_ttl: Duration,

    /// Time-to-live for cancelled executions (default: 1 hour)
    pub cancelled_ttl: Duration,

    /// Maximum number of execution records to keep (default: 10,000)
    pub max_entries: usize,

    /// Interval between background cleanup runs (default: 1 hour)
    pub cleanup_interval: Duration,

    /// Whether to enable background cleanup task (default: true)
    pub enable_background_cleanup: bool,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            completed_ttl: Duration::from_secs(24 * 60 * 60), // 24 hours
            failed_ttl: Duration::from_secs(24 * 60 * 60),    // 24 hours
            cancelled_ttl: Duration::from_secs(60 * 60),      // 1 hour
            max_entries: 10_000,
            cleanup_interval: Duration::from_secs(60 * 60),   // 1 hour
            enable_background_cleanup: true,
        }
    }
}

impl CleanupConfig {
    /// Create a config optimized for high-throughput scenarios
    pub fn high_throughput() -> Self {
        Self {
            completed_ttl: Duration::from_secs(6 * 60 * 60), // 6 hours
            failed_ttl: Duration::from_secs(12 * 60 * 60),   // 12 hours
            cancelled_ttl: Duration::from_secs(30 * 60),     // 30 minutes
            max_entries: 50_000,
            cleanup_interval: Duration::from_secs(30 * 60),  // 30 minutes
            enable_background_cleanup: true,
        }
    }

    /// Create a config optimized for low-memory scenarios
    pub fn low_memory() -> Self {
        Self {
            completed_ttl: Duration::from_secs(60 * 60),     // 1 hour
            failed_ttl: Duration::from_secs(60 * 60),        // 1 hour
            cancelled_ttl: Duration::from_secs(15 * 60),     // 15 minutes
            max_entries: 1_000,
            cleanup_interval: Duration::from_secs(15 * 60),  // 15 minutes
            enable_background_cleanup: true,
        }
    }

    /// Create a config for testing (aggressive cleanup)
    pub fn testing() -> Self {
        Self {
            completed_ttl: Duration::from_secs(60),          // 1 minute
            failed_ttl: Duration::from_secs(60),             // 1 minute
            cancelled_ttl: Duration::from_secs(30),          // 30 seconds
            max_entries: 100,
            cleanup_interval: Duration::from_secs(10),       // 10 seconds
            enable_background_cleanup: true,
        }
    }
}

/// Metrics for cleanup operations
#[derive(Debug, Clone, Default)]
pub struct CleanupMetrics {
    /// Total number of entries cleaned up
    pub total_cleaned: usize,

    /// Number of entries cleaned due to TTL expiration
    pub ttl_expired: usize,

    /// Number of entries cleaned due to size limit (LRU eviction)
    pub lru_evicted: usize,

    /// Timestamp of last cleanup operation
    pub last_cleanup: Option<u64>,

    /// Duration of last cleanup operation
    pub last_cleanup_duration_ms: Option<u64>,
}

impl CleanupMetrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a cleanup operation
    pub fn record_cleanup(&mut self, ttl_count: usize, lru_count: usize, duration: Duration) {
        self.ttl_expired += ttl_count;
        self.lru_evicted += lru_count;
        self.total_cleaned += ttl_count + lru_count;
        self.last_cleanup = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        );
        self.last_cleanup_duration_ms = Some(duration.as_millis() as u64);
    }
}
