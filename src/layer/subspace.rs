//! Subspace-based namespace isolation.
//!
//! Subspaces provide a pattern for organizing keys into isolated namespaces
//! within a flat key-value store. Each subspace acts as a "directory" with
//! its own key prefix, enabling:
//!
//! - **Multi-tenant isolation**: Each tenant gets their own subspace
//! - **Index organization**: Different indexes live in separate subspaces
//! - **Range queries**: Efficient scans within a subspace
//!
//! # FoundationDB Pattern
//!
//! This follows the [FoundationDB Subspace](
//! https://apple.github.io/foundationdb/developer-guide.html#subspaces)
//! pattern where a subspace is defined by a prefix tuple.
//!
//! # Example
//!
//! ```ignore
//! use aspen::layer::{Subspace, Tuple};
//!
//! // Create top-level namespaces
//! let users = Subspace::new(Tuple::new().push("users"));
//! let orders = Subspace::new(Tuple::new().push("orders"));
//!
//! // Create keys within namespaces
//! let alice_key = users.pack(&Tuple::new().push("alice").push("profile"));
//! let bob_key = users.pack(&Tuple::new().push("bob").push("profile"));
//!
//! // Create nested subspaces
//! let alice = users.subspace(&Tuple::new().push("alice"));
//! let (start, end) = alice.range(); // All of alice's data
//! ```
//!
//! # Design
//!
//! Subspaces are lightweight wrappers around a prefix tuple. They do not
//! allocate or store data themselves - they just provide methods for
//! constructing and deconstructing keys within their namespace.

use super::tuple::{Element, Tuple, TupleError};

/// A namespace within the key-value store.
///
/// Subspaces provide isolation and organization for keys. All keys within
/// a subspace share a common prefix, making range scans efficient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subspace {
    /// The prefix tuple that defines this subspace.
    prefix: Tuple,
    /// Cached packed prefix for efficiency.
    raw_prefix: Vec<u8>,
}

impl Subspace {
    /// Create a new subspace with the given prefix.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users = Subspace::new(Tuple::new().push("users"));
    /// ```
    pub fn new(prefix: Tuple) -> Self {
        let raw_prefix = prefix.pack();
        Self { prefix, raw_prefix }
    }

    /// Create a subspace from a raw byte prefix.
    ///
    /// This is useful when you have a pre-computed prefix that you want
    /// to use as a subspace without the overhead of tuple encoding.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let raw = Subspace::from_bytes(vec![0x02, b'u', b's', b'e', b'r', b's', 0x00]);
    /// ```
    pub fn from_bytes(raw_prefix: Vec<u8>) -> Self {
        // Try to unpack the prefix as a tuple for consistency
        let prefix = Tuple::unpack(&raw_prefix).unwrap_or_default();
        Self { prefix, raw_prefix }
    }

    /// Get the prefix tuple.
    pub fn prefix(&self) -> &Tuple {
        &self.prefix
    }

    /// Get the raw byte prefix.
    pub fn raw_prefix(&self) -> &[u8] {
        &self.raw_prefix
    }

    /// Create a nested subspace by appending a tuple to this subspace's prefix.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users = Subspace::new(Tuple::new().push("users"));
    /// let alice = users.subspace(&Tuple::new().push("alice"));
    /// // alice's prefix is ("users", "alice")
    /// ```
    pub fn subspace(&self, suffix: &Tuple) -> Self {
        let mut new_prefix = self.prefix.clone();
        for elem in suffix.iter() {
            new_prefix.push_mut(elem.clone());
        }
        Self::new(new_prefix)
    }

    /// Pack a key tuple within this subspace.
    ///
    /// The resulting bytes are the concatenation of this subspace's prefix
    /// and the packed key tuple.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users = Subspace::new(Tuple::new().push("users"));
    /// let key = users.pack(&Tuple::new().push("alice").push("profile"));
    /// // key is packed ("users", "alice", "profile")
    /// ```
    pub fn pack(&self, key: &Tuple) -> Vec<u8> {
        let mut result = self.raw_prefix.clone();
        key.pack_into(&mut result);
        result
    }

    /// Pack a single element within this subspace.
    ///
    /// Convenience method for single-element keys.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users = Subspace::new(Tuple::new().push("users"));
    /// let key = users.pack_element("alice");
    /// // key is packed ("users", "alice")
    /// ```
    pub fn pack_element<E: Into<Element>>(&self, element: E) -> Vec<u8> {
        self.pack(&Tuple::new().push(element))
    }

    /// Unpack a key from this subspace.
    ///
    /// Returns the key tuple without this subspace's prefix.
    /// Returns an error if the key doesn't start with this subspace's prefix.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users = Subspace::new(Tuple::new().push("users"));
    /// let key = users.pack(&Tuple::new().push("alice").push("profile"));
    /// let unpacked = users.unpack(&key)?;
    /// // unpacked is ("alice", "profile")
    /// ```
    pub fn unpack(&self, key: &[u8]) -> Result<Tuple, SubspaceError> {
        if !self.contains(key) {
            return Err(SubspaceError::PrefixMismatch {
                expected_len: self.raw_prefix.len(),
                actual_len: key.len(),
            });
        }

        let suffix = &key[self.raw_prefix.len()..];
        Tuple::unpack(suffix).map_err(|source| SubspaceError::TupleError { source })
    }

    /// Check if a key belongs to this subspace.
    ///
    /// Returns true if the key starts with this subspace's prefix.
    pub fn contains(&self, key: &[u8]) -> bool {
        key.starts_with(&self.raw_prefix)
    }

    /// Get the range of all keys in this subspace.
    ///
    /// Returns `(start_key, end_key)` suitable for range scans.
    /// The start is inclusive and the end is exclusive.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users = Subspace::new(Tuple::new().push("users"));
    /// let (start, end) = users.range();
    /// // Scan all keys in the users namespace
    /// storage.scan_range(start, end);
    /// ```
    pub fn range(&self) -> (Vec<u8>, Vec<u8>) {
        let start = self.raw_prefix.clone();
        let mut end = self.raw_prefix.clone();
        end.push(0xFF);
        (start, end)
    }

    /// Get the range of keys that match a specific prefix within this subspace.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users = Subspace::new(Tuple::new().push("users"));
    /// let (start, end) = users.range_of(&Tuple::new().push("alice"));
    /// // Scan all of alice's keys within users
    /// ```
    pub fn range_of(&self, key_prefix: &Tuple) -> (Vec<u8>, Vec<u8>) {
        let mut start = self.raw_prefix.clone();
        key_prefix.pack_into(&mut start);
        let mut end = start.clone();
        end.push(0xFF);
        (start, end)
    }

    /// Get the strict upper bound for this subspace (FDB strinc).
    ///
    /// Returns None if the prefix is all 0xFF bytes.
    pub fn strinc(&self) -> Option<Vec<u8>> {
        let mut result = self.raw_prefix.clone();
        strinc_inplace(&mut result).then_some(result)
    }
}

impl Default for Subspace {
    /// The default subspace is the root (empty prefix).
    fn default() -> Self {
        Self::new(Tuple::new())
    }
}

/// Errors that can occur during subspace operations.
#[derive(Debug, snafu::Snafu)]
pub enum SubspaceError {
    /// Key doesn't match subspace prefix.
    #[snafu(display(
        "key prefix mismatch: expected {} bytes, got {} bytes",
        expected_len,
        actual_len
    ))]
    PrefixMismatch {
        /// Expected prefix length.
        expected_len: usize,
        /// Actual key length.
        actual_len: usize,
    },

    /// Error decoding the tuple portion.
    #[snafu(display("tuple decode error: {source}"))]
    TupleError {
        /// The underlying tuple error.
        source: TupleError,
    },
}

/// Increment a byte vector in place (FDB strinc).
///
/// Returns true if successful, false if all bytes are 0xFF.
fn strinc_inplace(data: &mut Vec<u8>) -> bool {
    while let Some(&last) = data.last() {
        if last < 0xFF {
            *data.last_mut().unwrap() = last + 1;
            return true;
        }
        data.pop();
    }
    false
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subspace_creation() {
        let users = Subspace::new(Tuple::new().push("users"));
        assert!(!users.raw_prefix().is_empty());
        assert_eq!(users.prefix().len(), 1);
    }

    #[test]
    fn test_subspace_pack() {
        let users = Subspace::new(Tuple::new().push("users"));
        let key = users.pack(&Tuple::new().push("alice").push("profile"));

        // Should start with the users prefix
        assert!(key.starts_with(users.raw_prefix()));

        // Should be longer than just the prefix
        assert!(key.len() > users.raw_prefix().len());
    }

    #[test]
    fn test_subspace_unpack() {
        let users = Subspace::new(Tuple::new().push("users"));
        let original = Tuple::new().push("alice").push("profile");
        let packed = users.pack(&original);

        let unpacked = users.unpack(&packed).unwrap();
        assert_eq!(unpacked, original);
    }

    #[test]
    fn test_subspace_contains() {
        let users = Subspace::new(Tuple::new().push("users"));
        let orders = Subspace::new(Tuple::new().push("orders"));

        let user_key = users.pack(&Tuple::new().push("alice"));
        let order_key = orders.pack(&Tuple::new().push("12345"));

        assert!(users.contains(&user_key));
        assert!(!users.contains(&order_key));
        assert!(orders.contains(&order_key));
        assert!(!orders.contains(&user_key));
    }

    #[test]
    fn test_nested_subspace() {
        let users = Subspace::new(Tuple::new().push("users"));
        let alice = users.subspace(&Tuple::new().push("alice"));

        // Alice's subspace should have a longer prefix
        assert!(alice.raw_prefix().len() > users.raw_prefix().len());

        // Alice's keys should be in users subspace
        let key = alice.pack(&Tuple::new().push("profile"));
        assert!(users.contains(&key));
        assert!(alice.contains(&key));

        // Unpack from alice's perspective
        let unpacked = alice.unpack(&key).unwrap();
        assert_eq!(unpacked, Tuple::new().push("profile"));
    }

    #[test]
    fn test_subspace_range() {
        let users = Subspace::new(Tuple::new().push("users"));
        let (start, end) = users.range();

        // Start should be the prefix
        assert_eq!(start, users.raw_prefix());

        // End should be prefix + 0xFF
        assert_eq!(end.len(), start.len() + 1);
        assert_eq!(end[..start.len()], start[..]);
        assert_eq!(end[start.len()], 0xFF);

        // Any key in subspace should be in range
        let key = users.pack(&Tuple::new().push("alice"));
        assert!(key >= start && key < end);

        // Keys outside subspace should not be in range
        let other = Subspace::new(Tuple::new().push("orders"));
        let other_key = other.pack(&Tuple::new().push("12345"));
        assert!(!(other_key >= start && other_key < end));
    }

    #[test]
    fn test_subspace_range_of() {
        let users = Subspace::new(Tuple::new().push("users"));
        let (start, end) = users.range_of(&Tuple::new().push("alice"));

        // Keys for alice should be in range
        let alice_profile = users.pack(&Tuple::new().push("alice").push("profile"));
        let alice_settings = users.pack(&Tuple::new().push("alice").push("settings"));
        assert!(alice_profile >= start && alice_profile < end);
        assert!(alice_settings >= start && alice_settings < end);

        // Keys for bob should NOT be in range
        let bob_profile = users.pack(&Tuple::new().push("bob").push("profile"));
        assert!(!(bob_profile >= start && bob_profile < end));
    }

    #[test]
    fn test_subspace_isolation() {
        // Different subspaces should have non-overlapping ranges
        let sub1 = Subspace::new(Tuple::new().push("a"));
        let sub2 = Subspace::new(Tuple::new().push("b"));

        let (start1, end1) = sub1.range();
        let (start2, end2) = sub2.range();

        // Since "a" < "b", sub1's range should be entirely before sub2's
        assert!(end1 <= start2);

        // Cross-checks
        let key1 = sub1.pack(&Tuple::new().push("key"));
        let key2 = sub2.pack(&Tuple::new().push("key"));

        assert!(key1 >= start1 && key1 < end1);
        assert!(!(key1 >= start2 && key1 < end2));

        assert!(key2 >= start2 && key2 < end2);
        assert!(!(key2 >= start1 && key2 < end1));
    }

    #[test]
    fn test_pack_element() {
        let users = Subspace::new(Tuple::new().push("users"));

        let key1 = users.pack_element("alice");
        let key2 = users.pack(&Tuple::new().push("alice"));

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_default_subspace() {
        let root = Subspace::default();
        assert!(root.raw_prefix().is_empty());

        // Any key should be in the root subspace
        let key = Tuple::new().push("anything").pack();
        assert!(root.contains(&key));
    }

    #[test]
    fn test_strinc() {
        let sub = Subspace::new(Tuple::new().push("test"));
        let incremented = sub.strinc().unwrap();

        // Should be strictly greater than the prefix
        assert!(incremented > sub.raw_prefix().to_vec());

        // Any key in subspace should be less than strinc result
        let key = sub.pack(&Tuple::new().push("anything"));
        assert!(key < incremented);
    }

    #[test]
    fn test_from_bytes() {
        let original = Subspace::new(Tuple::new().push("users"));
        let from_bytes = Subspace::from_bytes(original.raw_prefix().to_vec());

        // Should have the same raw prefix
        assert_eq!(original.raw_prefix(), from_bytes.raw_prefix());
    }
}
