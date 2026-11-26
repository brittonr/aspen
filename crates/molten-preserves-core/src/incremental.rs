//! The canonical packed encoder driven event by event.
//!
//! Shells that walk their own value tree (the `molten` bridge drives it from `preserves::IOValue`
//! through the `preserves::Writer` events) call these methods in preorder and get canonical bytes
//! without building a `PackedValue` copy: atom payloads are written once. While a set or dictionary
//! is open, output goes to one shared arena; each element or entry is a range of it, and the ranges
//! are sorted by their bytes when the compound closes, the order `crate::writer::encode_canonical`
//! produces. No buffer is allocated per element.

/// One open set or dictionary: where its bytes begin in the arena and where each chunk starts.
struct Group {
    base: usize,
    starts: alloc::vec::Vec<usize>,
}

/// Canonical packed output with an arena for open sets and dictionaries.
#[derive(Default)]
pub struct CanonicalWriter {
    base: alloc::vec::Vec<u8>,
    arena: alloc::vec::Vec<u8>,
    groups: alloc::vec::Vec<Group>,
    ranges: alloc::vec::Vec<(usize, usize)>,
    sorted: alloc::vec::Vec<u8>,
}

impl CanonicalWriter {
    pub fn new() -> Self {
        Self::default()
    }

    fn current(&mut self) -> &mut alloc::vec::Vec<u8> {
        if self.groups.is_empty() {
            &mut self.base
        } else {
            &mut self.arena
        }
    }

    fn atom(&mut self, tag: u8, payload: &[u8]) {
        crate::writer::write_atom(self.current(), tag, payload);
    }

    pub fn boolean(&mut self, value: bool) {
        self.current().push(if value { crate::tags::TRUE } else { crate::tags::FALSE });
    }

    /// An IEEE 754 binary64 value by its exact bit pattern.
    pub fn double(&mut self, bits: u64) {
        self.atom(crate::tags::IEEE754, &bits.to_be_bytes());
    }

    pub fn signed(&mut self, value: i128) {
        let bytes = value.to_be_bytes();
        let start = crate::model::signed::redundant_prefix_len(&bytes);
        self.atom(crate::tags::SIGNED_INTEGER, &bytes[start..]);
    }

    pub fn unsigned(&mut self, value: u128) {
        if let Ok(signed) = i128::try_from(value) {
            return self.signed(signed);
        }
        let mut bytes = [0; 17];
        bytes[1..].copy_from_slice(&value.to_be_bytes());
        self.atom(crate::tags::SIGNED_INTEGER, &bytes);
    }

    /// A wider integer given as its minimal big-endian two's-complement bytes.
    pub fn integer_bytes(&mut self, minimal: &[u8]) {
        self.atom(crate::tags::SIGNED_INTEGER, minimal);
    }

    pub fn string(&mut self, text: &str) {
        self.atom(crate::tags::STRING, text.as_bytes());
    }

    pub fn byte_string(&mut self, bytes: &[u8]) {
        self.atom(crate::tags::BYTE_STRING, bytes);
    }

    pub fn symbol(&mut self, name: &str) {
        self.atom(crate::tags::SYMBOL, name.as_bytes());
    }

    pub fn open_record(&mut self) {
        self.current().push(crate::tags::RECORD);
    }

    pub fn open_sequence(&mut self) {
        self.current().push(crate::tags::SEQUENCE);
    }

    /// Closes a record or sequence.
    pub fn close(&mut self) {
        self.current().push(crate::tags::END);
    }

    /// Opens a set; call `next_item` after each element and `close_sorted` at the end.
    pub fn open_set(&mut self) {
        self.open_group(crate::tags::SET);
    }

    /// Opens a dictionary; call `next_item` after each value and `close_sorted` at the end.
    pub fn open_dictionary(&mut self) {
        self.open_group(crate::tags::DICTIONARY);
    }

    fn open_group(&mut self, tag: u8) {
        self.current().push(tag);
        let base = self.arena.len();
        self.groups.push(Group {
            base,
            starts: alloc::vec![base],
        });
    }

    /// Starts the chunk of the next set element or dictionary entry.
    pub fn next_item(&mut self) {
        let end = self.arena.len();
        if let Some(group) = self.groups.last_mut() {
            group.starts.push(end);
        }
    }

    /// Sorts the open group's chunks into the enclosing output and closes the set or dictionary.
    pub fn close_sorted(&mut self) {
        let Some(group) = self.groups.pop() else {
            return;
        };
        let end = self.arena.len();
        self.ranges.clear();
        self.ranges.extend(group.starts.windows(2).map(|pair| (pair[0], pair[1])));
        if let Some(&last) = group.starts.last() {
            self.ranges.push((last, end));
        }
        let arena = &self.arena;
        self.ranges.sort_unstable_by(|left, right| arena[left.0..left.1].cmp(&arena[right.0..right.1]));
        self.sorted.clear();
        self.sorted.reserve(end - group.base + 1);
        for &(start, stop) in &self.ranges {
            self.sorted.extend_from_slice(&self.arena[start..stop]);
        }
        self.sorted.push(crate::tags::END);
        self.arena.truncate(group.base);
        let sorted = core::mem::take(&mut self.sorted);
        self.current().extend_from_slice(&sorted);
        self.sorted = sorted;
    }

    /// Prefixes the next value as `Embedded`.
    pub fn embedded(&mut self) {
        self.current().push(crate::tags::EMBEDDED);
    }

    /// The canonical bytes; `None` if a set or dictionary is still open.
    pub fn finish(self) -> Option<alloc::vec::Vec<u8>> {
        self.groups.is_empty().then_some(self.base)
    }
}
