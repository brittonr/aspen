use std::cmp::Ordering;

use super::TupleError;
use super::decoding::decode_element;
use super::element::Element;

// =============================================================================
// Tuple Type
// =============================================================================

/// An ordered collection of typed elements that can be packed into bytes.
///
/// Tuples are the fundamental building block for structured keys. When packed,
/// they produce bytes that sort lexicographically in the same order as the
/// original tuple elements.
///
/// # Example
///
/// ```
/// use aspen_layer::Tuple;
///
/// let t1 = Tuple::new().push("users").push(1i64);
/// let t2 = Tuple::new().push("users").push(2i64);
///
/// assert!(t1.pack() < t2.pack()); // Lexicographic ordering preserved
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tuple {
    pub(crate) elements: Vec<Element>,
}

impl Tuple {
    /// Create a new empty tuple.
    pub fn new() -> Self {
        Self { elements: Vec::new() }
    }

    /// Create a tuple with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            elements: Vec::with_capacity(capacity),
        }
    }

    /// Push an element onto the tuple (builder pattern).
    pub fn push<E: Into<Element>>(mut self, element: E) -> Self {
        self.elements.push(element.into());
        self
    }

    /// Push an element onto the tuple (mutating).
    pub fn push_mut<E: Into<Element>>(&mut self, element: E) {
        self.elements.push(element.into());
    }

    /// Get the number of elements in the tuple.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if the tuple is empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Get an element by index.
    pub fn get(&self, index: usize) -> Option<&Element> {
        self.elements.get(index)
    }

    /// Get an iterator over the elements.
    pub fn iter(&self) -> impl Iterator<Item = &Element> {
        self.elements.iter()
    }

    /// Pack the tuple into bytes.
    ///
    /// The resulting bytes will sort lexicographically in the same order
    /// as the original tuple elements.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.elements.len() * 8);
        self.pack_into(&mut buf);
        buf
    }

    /// Pack the tuple into an existing buffer.
    pub fn pack_into(&self, buf: &mut Vec<u8>) {
        for elem in &self.elements {
            elem.pack_into(buf);
        }
    }

    /// Unpack a tuple from bytes.
    ///
    /// Returns the decoded tuple and ensures all bytes were consumed.
    pub fn unpack(data: &[u8]) -> Result<Self, TupleError> {
        let (tuple, consumed) = Self::unpack_partial(data)?;
        if consumed != data.len() {
            // Extra bytes after tuple - this is valid, return what we got
        }
        Ok(tuple)
    }

    /// Unpack a tuple from bytes, returning how many bytes were consumed.
    ///
    /// This is useful for parsing nested tuples or concatenated data.
    pub fn unpack_partial(data: &[u8]) -> Result<(Self, usize), TupleError> {
        let mut tuple = Tuple::new();
        let mut offset = 0;

        while offset < data.len() {
            let (elem, consumed) = decode_element(data, offset)?;
            tuple.elements.push(elem);
            offset += consumed;
        }

        Ok((tuple, offset))
    }

    /// Get the range of keys that have this tuple as a prefix.
    ///
    /// Returns `(start_key, end_key)` where:
    /// - `start_key` is the packed tuple
    /// - `end_key` is the packed tuple with 0xFF appended (exclusive bound)
    ///
    /// This is useful for range scans over all keys with a given prefix.
    ///
    /// # Example
    ///
    /// ```
    /// use aspen_layer::Tuple;
    ///
    /// let prefix = Tuple::new().push("users");
    /// let (start, end) = prefix.range();
    ///
    /// // Scan all keys starting with ("users", ...)
    /// // storage.scan_range(start, end);
    /// ```
    pub fn range(&self) -> (Vec<u8>, Vec<u8>) {
        let start = self.pack();
        let mut end = start.clone();
        end.push(0xFF); // 0xFF is the highest byte, so this is an exclusive upper bound
        (start, end)
    }

    /// Increment the last byte of the packed tuple to get a strict upper bound.
    ///
    /// This is the FDB `strinc` operation - useful for getting an exclusive
    /// end key for range queries.
    pub fn strinc(&self) -> Option<Vec<u8>> {
        let mut packed = self.pack();
        strinc(&mut packed).then_some(packed)
    }
}

impl PartialOrd for Tuple {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Tuple {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare by packing for correct FDB ordering
        self.pack().cmp(&other.pack())
    }
}

/// Increment a byte string to get a strict upper bound (FDB strinc).
///
/// Returns true if the increment was successful, false if the string was all 0xFF.
fn strinc(data: &mut Vec<u8>) -> bool {
    // Find the last non-0xFF byte and increment it
    while let Some(&last) = data.last() {
        if last < 0xFF {
            let len = data.len();
            data[len - 1] = last + 1;
            return true;
        }
        data.pop();
    }
    false
}

#[cfg(test)]
pub(super) fn strinc_for_test(data: &mut Vec<u8>) -> bool {
    strinc(data)
}
