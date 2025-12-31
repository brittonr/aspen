//! Constants for iroh-docs integration.
//!
//! Tiger Style: All constants are explicitly typed with fixed limits.

use std::time::Duration;

/// ALPN identifier for iroh-docs sync protocol.
///
/// Re-exported from iroh_docs for convenience.
/// Value: `b"/iroh-sync/1"`
pub use iroh_docs::net::ALPN as DOCS_SYNC_ALPN;

/// Maximum number of concurrent docs sync connections.
/// Tiger Style: Bounded to prevent connection exhaustion.
pub const MAX_DOCS_CONNECTIONS: u32 = 100;

/// Batch size for exporting KV entries to docs.
/// Tiger Style: Bounded to prevent unbounded memory usage.
pub const EXPORT_BATCH_SIZE: u32 = 100;

/// Interval for background full-sync (drift correction).
pub const BACKGROUND_SYNC_INTERVAL: Duration = Duration::from_secs(60);

/// Maximum key size in docs entries.
/// Tiger Style: Bounded to prevent oversized entries.
pub const MAX_DOC_KEY_SIZE: usize = 4096;

/// Maximum value size in docs entries.
/// Tiger Style: Bounded to prevent oversized entries.
pub const MAX_DOC_VALUE_SIZE: usize = 1_048_576; // 1MB

/// Timeout for sync operations.
pub const DOCS_SYNC_TIMEOUT: Duration = Duration::from_secs(30);
