//! Cursor type for resumable pub/sub subscriptions.
//!
//! A cursor wraps the Raft log index, providing a global ordering
//! across all topics and enabling resume from any point in history.

use serde::Deserialize;
use serde::Serialize;

/// A cursor representing a position in the event stream.
///
/// Cursors are Raft log indices, providing:
/// - **Global ordering**: All events across all topics have a single total order
/// - **Resume capability**: Subscribe from any historical point
/// - **Correlation**: Same log index as KV operations
///
/// # Examples
///
/// ```
/// use aspen_pubsub::Cursor;
///
/// // Start from the beginning of all events
/// let cursor = Cursor::BEGINNING;
///
/// // Start from only new events
/// let cursor = Cursor::LATEST;
///
/// // Resume from a specific position
/// let cursor = Cursor::from_index(12345);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Cursor(pub u64);

impl Cursor {
    /// Cursor representing the beginning of all events.
    ///
    /// Subscribing from this cursor will replay all available
    /// historical events before streaming new ones.
    pub const BEGINNING: Cursor = Cursor(0);

    /// Cursor representing only new events.
    ///
    /// Subscribing from this cursor will skip all historical
    /// events and only receive new ones as they are published.
    pub const LATEST: Cursor = Cursor(u64::MAX);

    /// Create a cursor from a specific Raft log index.
    ///
    /// Use this to resume from a previously saved cursor position.
    #[inline]
    pub const fn from_index(index: u64) -> Self {
        Cursor(index)
    }

    /// Get the underlying log index.
    #[inline]
    pub const fn index(&self) -> u64 {
        self.0
    }

    /// Check if this cursor represents the beginning.
    #[inline]
    pub const fn is_beginning(&self) -> bool {
        self.0 == 0
    }

    /// Check if this cursor represents the latest position.
    #[inline]
    pub const fn is_latest(&self) -> bool {
        self.0 == u64::MAX
    }

    /// Get the next cursor position (current + 1).
    ///
    /// Useful for checkpointing after processing an event.
    #[inline]
    pub const fn next(&self) -> Self {
        Cursor(self.0.saturating_add(1))
    }

    /// Get the previous cursor position (current - 1).
    ///
    /// Saturates at 0.
    #[inline]
    pub const fn prev(&self) -> Self {
        Cursor(self.0.saturating_sub(1))
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::BEGINNING
    }
}

impl std::fmt::Display for Cursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_latest() {
            write!(f, "LATEST")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

impl From<u64> for Cursor {
    fn from(index: u64) -> Self {
        Cursor(index)
    }
}

impl From<Cursor> for u64 {
    fn from(cursor: Cursor) -> Self {
        cursor.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_constants() {
        assert_eq!(Cursor::BEGINNING.index(), 0);
        assert_eq!(Cursor::LATEST.index(), u64::MAX);
        assert!(Cursor::BEGINNING.is_beginning());
        assert!(Cursor::LATEST.is_latest());
    }

    #[test]
    fn test_cursor_from_index() {
        let cursor = Cursor::from_index(12345);
        assert_eq!(cursor.index(), 12345);
        assert!(!cursor.is_beginning());
        assert!(!cursor.is_latest());
    }

    #[test]
    fn test_cursor_next_prev() {
        let cursor = Cursor::from_index(100);
        assert_eq!(cursor.next().index(), 101);
        assert_eq!(cursor.prev().index(), 99);

        // Saturation at boundaries
        assert_eq!(Cursor::BEGINNING.prev().index(), 0);
        assert_eq!(Cursor::LATEST.next().index(), u64::MAX);
    }

    #[test]
    fn test_cursor_ordering() {
        assert!(Cursor::from_index(10) < Cursor::from_index(20));
        assert!(Cursor::BEGINNING < Cursor::LATEST);
    }

    #[test]
    fn test_cursor_display() {
        assert_eq!(Cursor::from_index(12345).to_string(), "12345");
        assert_eq!(Cursor::LATEST.to_string(), "LATEST");
    }

    #[test]
    fn test_cursor_conversions() {
        let cursor: Cursor = 100u64.into();
        assert_eq!(cursor.index(), 100);

        let index: u64 = cursor.into();
        assert_eq!(index, 100);
    }

    #[test]
    fn test_cursor_serialization() {
        let cursor = Cursor::from_index(12345);
        let json = serde_json::to_string(&cursor).unwrap();
        let decoded: Cursor = serde_json::from_str(&json).unwrap();
        assert_eq!(cursor, decoded);
    }
}
