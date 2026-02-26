//! Key range type for representing contiguous segments of the key space.

use serde::Deserialize;
use serde::Serialize;

/// A key range representing a contiguous segment of the key space.
///
/// Ranges are half-open intervals: `[start_key, end_key)`.
/// An empty `end_key` indicates the range extends to the end of the keyspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct KeyRange {
    /// Start key (inclusive). Empty string means start of keyspace.
    pub start_key: String,
    /// End key (exclusive). Empty string means end of keyspace.
    pub end_key: String,
}

impl KeyRange {
    /// Create a new key range.
    pub fn new(start_key: impl Into<String>, end_key: impl Into<String>) -> Self {
        Self {
            start_key: start_key.into(),
            end_key: end_key.into(),
        }
    }

    /// Create a range covering the entire keyspace.
    pub fn full() -> Self {
        Self::new("", "")
    }

    /// Check if a key falls within this range.
    ///
    /// Returns true if `start_key <= key < end_key` (or key >= start_key if end_key is empty).
    pub fn contains(&self, key: &str) -> bool {
        key >= self.start_key.as_str() && (self.end_key.is_empty() || key < self.end_key.as_str())
    }

    /// Check if this range is empty (start >= end for non-unbounded ranges).
    pub fn is_empty(&self) -> bool {
        !self.end_key.is_empty() && self.start_key >= self.end_key
    }

    /// Split this range at the given key, returning (left, right) ranges.
    ///
    /// Returns None if split_key is not within the range.
    pub fn split_at(&self, split_key: &str) -> Option<(KeyRange, KeyRange)> {
        if !self.contains(split_key) || split_key == self.start_key.as_str() {
            return None;
        }

        let left = KeyRange::new(&self.start_key, split_key);
        let right = KeyRange::new(split_key, &self.end_key);

        Some((left, right))
    }

    /// Check if this range is adjacent to another (can be merged).
    ///
    /// Two ranges are adjacent if one ends where the other begins.
    /// Note: We compare non-empty boundaries, treating empty "" as infinity.
    pub fn is_adjacent_to(&self, other: &KeyRange) -> bool {
        // self comes before other: self.end == other.start (both non-empty)
        (!self.end_key.is_empty() && self.end_key == other.start_key)
            // other comes before self: other.end == self.start (both non-empty)
            || (!other.end_key.is_empty() && other.end_key == self.start_key)
    }

    /// Merge this range with an adjacent range.
    ///
    /// Returns None if ranges are not adjacent.
    pub fn merge_with(&self, other: &KeyRange) -> Option<KeyRange> {
        // self comes before other: self ends where other starts
        if !self.end_key.is_empty() && self.end_key == other.start_key {
            // Merged: [self.start, other.end)
            Some(KeyRange::new(&self.start_key, &other.end_key))
        // other comes before self: other ends where self starts
        } else if !other.end_key.is_empty() && other.end_key == self.start_key {
            // Merged: [other.start, self.end)
            Some(KeyRange::new(&other.start_key, &self.end_key))
        } else {
            None
        }
    }
}

impl Default for KeyRange {
    fn default() -> Self {
        Self::full()
    }
}
