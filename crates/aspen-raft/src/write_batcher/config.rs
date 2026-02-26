use std::time::Duration;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Configuration for write batching behavior.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BatchConfig {
    /// Maximum number of operations per batch.
    /// Tiger Style: bounded to prevent unbounded memory use.
    pub max_entries: u32,
    /// Maximum total size of values in bytes per batch.
    /// Tiger Style: bounded to prevent memory exhaustion.
    pub max_bytes: u64,
    /// Maximum time to wait before flushing a batch in milliseconds.
    /// Trade-off: higher = more throughput, lower = less latency.
    #[serde(default = "default_max_wait_ms")]
    pub max_wait_ms: u64,
    /// Computed max_wait Duration (not serialized).
    #[serde(skip)]
    pub max_wait: Duration,
}

fn default_max_wait_ms() -> u64 {
    2
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_entries: 100,
            max_bytes: 1024 * 1024, // 1 MB (u64)
            max_wait_ms: 2,
            max_wait: Duration::from_millis(2),
        }
    }
}

impl BatchConfig {
    /// Create a config optimized for high throughput (more batching).
    ///
    /// Note: max_entries is capped at 100 due to underlying storage limits
    /// (MAX_SETMULTI_KEYS). The throughput gain comes from the longer max_wait
    /// which allows more concurrent writes to batch together.
    pub fn high_throughput() -> Self {
        Self {
            max_entries: 100,           // Capped at MAX_SETMULTI_KEYS
            max_bytes: 4 * 1024 * 1024, // 4 MB (u64)
            max_wait_ms: 5,
            max_wait: Duration::from_millis(5),
        }
    }

    /// Create a config optimized for low latency (less batching).
    pub fn low_latency() -> Self {
        Self {
            max_entries: 20,
            max_bytes: 256 * 1024, // 256 KB (u64)
            max_wait_ms: 1,
            max_wait: Duration::from_millis(1),
        }
    }

    /// Disable batching entirely (every write is immediate).
    pub fn disabled() -> Self {
        Self {
            max_entries: 1,
            max_bytes: u64::MAX,
            max_wait_ms: 0,
            max_wait: Duration::ZERO,
        }
    }

    /// Finalize config by computing max_wait from max_wait_ms.
    /// Call this after deserializing from config.
    pub fn finalize(mut self) -> Self {
        // Tiger Style: max_entries must be positive and bounded
        assert!(self.max_entries > 0, "BATCH_CONFIG: max_entries must be > 0");
        assert!(
            self.max_entries <= 10_000,
            "BATCH_CONFIG: max_entries {} exceeds hard limit 10000",
            self.max_entries
        );
        // Tiger Style: max_bytes must be positive
        assert!(self.max_bytes > 0, "BATCH_CONFIG: max_bytes must be > 0");

        self.max_wait = Duration::from_millis(self.max_wait_ms);
        self
    }
}
