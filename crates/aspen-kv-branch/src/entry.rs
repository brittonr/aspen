//! Branch entry types for the dirty map.

use serde::Deserialize;
use serde::Serialize;

/// A buffered mutation in a branch overlay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BranchEntry {
    /// A buffered write with the new value.
    Write { value: String },
    /// A tombstone marking the key as deleted in this branch.
    Tombstone,
}

impl BranchEntry {
    /// Returns the byte size of this entry's value, or 0 for tombstones.
    pub fn value_bytes(&self) -> u64 {
        match self {
            BranchEntry::Write { value } => value.len() as u64,
            BranchEntry::Tombstone => 0,
        }
    }

    /// Returns true if this entry is a tombstone.
    pub fn is_tombstone(&self) -> bool {
        matches!(self, BranchEntry::Tombstone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_entry_reports_value_bytes() {
        let entry = BranchEntry::Write { value: "hello".into() };
        assert_eq!(entry.value_bytes(), 5);
        assert!(!entry.is_tombstone());
    }

    #[test]
    fn tombstone_reports_zero_bytes() {
        let entry = BranchEntry::Tombstone;
        assert_eq!(entry.value_bytes(), 0);
        assert!(entry.is_tombstone());
    }
}
