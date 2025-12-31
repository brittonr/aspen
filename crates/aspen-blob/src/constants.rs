//! Constants for iroh-blobs integration.
//!
//! Tiger Style: All constants are explicitly typed with fixed limits
//! to prevent unbounded resource usage.

use std::time::Duration;

/// Threshold for automatic large value offloading (1 MB).
/// KV values larger than this are stored as blobs with a reference in KV.
pub const BLOB_THRESHOLD: u32 = 1_048_576;

/// Maximum blob size (1 GB).
/// Tiger Style: Bounded to prevent unbounded memory/disk usage.
pub const MAX_BLOB_SIZE: u64 = 1_073_741_824;

/// Garbage collection interval.
pub const BLOB_GC_INTERVAL: Duration = Duration::from_secs(60);

/// Grace period before collecting unreferenced blobs.
/// Gives time for in-flight operations to complete.
pub const BLOB_GC_GRACE_PERIOD: Duration = Duration::from_secs(300);

/// Prefix for blob references stored in KV.
/// Values starting with this prefix are blob references, not actual data.
pub const BLOB_REF_PREFIX: &str = "__blob:";

/// Maximum concurrent blob downloads.
/// Tiger Style: Bounded to prevent connection exhaustion.
pub const MAX_CONCURRENT_BLOB_DOWNLOADS: u32 = 10;

/// Maximum concurrent blob uploads.
pub const MAX_CONCURRENT_BLOB_UPLOADS: u32 = 10;

/// Blob download timeout.
pub const BLOB_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

/// Blob upload timeout.
pub const BLOB_UPLOAD_TIMEOUT: Duration = Duration::from_secs(300);

/// Maximum number of blobs to list in a single request.
/// Tiger Style: Bounded pagination.
pub const MAX_BLOB_LIST_SIZE: u32 = 1000;

/// Tag prefix for KV-referenced blobs.
/// Format: "kv:{key}" for blobs referenced by KV entries.
pub const KV_TAG_PREFIX: &str = "kv:";

/// Tag prefix for user-created blob protections.
/// Format: "user:{name}" for manually protected blobs.
pub const USER_TAG_PREFIX: &str = "user:";
