//! Persistent file metadata for timestamps.
//!
//! Stores mtime/ctime as a compact binary blob alongside file data in
//! the KV store using a `.meta` key suffix.
//!
//! # Format
//!
//! 32 bytes, little-endian:
//!
//! | Offset | Size | Field        |
//! |--------|------|--------------|
//! | 0      | 8    | mtime_secs   |
//! | 8      | 8    | mtime_nsecs  |
//! | 16     | 8    | ctime_secs   |
//! | 24     | 8    | ctime_nsecs  |

use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

/// Size of the serialized metadata blob in bytes.
const META_SIZE: usize = 32;

/// Persistent file metadata (timestamps).
///
/// Stored as a companion `.meta` key alongside file data.
/// Kept compact (32 bytes) to minimize KV overhead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMetadata {
    /// Last modification time (seconds since Unix epoch).
    pub mtime_secs: i64,
    /// Last modification time (nanoseconds component).
    pub mtime_nsecs: i64,
    /// Last status change time (seconds since Unix epoch).
    pub ctime_secs: i64,
    /// Last status change time (nanoseconds component).
    pub ctime_nsecs: i64,
}

impl FileMetadata {
    /// Create metadata with the current wall-clock time for both mtime and ctime.
    pub fn now() -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        let secs = now.as_secs() as i64;
        let nsecs = now.subsec_nanos() as i64;
        Self {
            mtime_secs: secs,
            mtime_nsecs: nsecs,
            ctime_secs: secs,
            ctime_nsecs: nsecs,
        }
    }

    /// Create metadata with a specific mtime and current ctime.
    pub fn with_mtime(mtime_secs: i64, mtime_nsecs: i64) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        Self {
            mtime_secs,
            mtime_nsecs,
            ctime_secs: now.as_secs() as i64,
            ctime_nsecs: now.subsec_nanos() as i64,
        }
    }

    /// Update ctime to current time (e.g., on setattr).
    pub fn touch_ctime(mut self) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        self.ctime_secs = now.as_secs() as i64;
        self.ctime_nsecs = now.subsec_nanos() as i64;
        self
    }

    /// Update both mtime and ctime to current time (e.g., on write).
    pub fn touch(mut self) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        let secs = now.as_secs() as i64;
        let nsecs = now.subsec_nanos() as i64;
        self.mtime_secs = secs;
        self.mtime_nsecs = nsecs;
        self.ctime_secs = secs;
        self.ctime_nsecs = nsecs;
        self
    }

    /// Serialize to a 32-byte little-endian blob.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(META_SIZE);
        buf.extend_from_slice(&self.mtime_secs.to_le_bytes());
        buf.extend_from_slice(&self.mtime_nsecs.to_le_bytes());
        buf.extend_from_slice(&self.ctime_secs.to_le_bytes());
        buf.extend_from_slice(&self.ctime_nsecs.to_le_bytes());
        buf
    }

    /// Deserialize from a byte slice.
    ///
    /// Returns `None` if the slice is too short.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < META_SIZE {
            return None;
        }
        Some(Self {
            mtime_secs: i64::from_le_bytes(bytes[0..8].try_into().ok()?),
            mtime_nsecs: i64::from_le_bytes(bytes[8..16].try_into().ok()?),
            ctime_secs: i64::from_le_bytes(bytes[16..24].try_into().ok()?),
            ctime_nsecs: i64::from_le_bytes(bytes[24..32].try_into().ok()?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_serialization() {
        let meta = FileMetadata {
            mtime_secs: 1700000000,
            mtime_nsecs: 123456789,
            ctime_secs: 1700000001,
            ctime_nsecs: 987654321,
        };
        let bytes = meta.to_bytes();
        assert_eq!(bytes.len(), 32);

        let restored = FileMetadata::from_bytes(&bytes).unwrap();
        assert_eq!(meta, restored);
    }

    #[test]
    fn now_creates_valid_metadata() {
        let meta = FileMetadata::now();
        assert!(meta.mtime_secs > 0);
        assert!(meta.ctime_secs > 0);
        assert_eq!(meta.mtime_secs, meta.ctime_secs);
    }

    #[test]
    fn touch_updates_both_timestamps() {
        let old = FileMetadata {
            mtime_secs: 1000,
            mtime_nsecs: 0,
            ctime_secs: 1000,
            ctime_nsecs: 0,
        };
        let updated = old.touch();
        assert!(updated.mtime_secs > 1000);
        assert!(updated.ctime_secs > 1000);
    }

    #[test]
    fn touch_ctime_only_updates_ctime() {
        let old = FileMetadata {
            mtime_secs: 1000,
            mtime_nsecs: 0,
            ctime_secs: 1000,
            ctime_nsecs: 0,
        };
        let updated = old.touch_ctime();
        assert_eq!(updated.mtime_secs, 1000);
        assert!(updated.ctime_secs > 1000);
    }

    #[test]
    fn from_bytes_rejects_short_input() {
        assert!(FileMetadata::from_bytes(&[0u8; 16]).is_none());
        assert!(FileMetadata::from_bytes(&[]).is_none());
    }
}
