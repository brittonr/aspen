//! Sets and dictionaries ordered by canonical packed bytes.
//!
//! Each element (set) or key (dictionary) is stored under its canonical packed encoding, so
//! iteration follows the canonical wire order and two elements are equal exactly when their
//! canonical bytes are equal. The encoder writes the stored bytes directly.

/// A set of distinct values in canonical packed byte order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PackedSet {
    elements: alloc::collections::BTreeMap<alloc::vec::Vec<u8>, super::PackedValue>,
}

impl PackedSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `element`; false when an element with the same canonical bytes is already present.
    pub fn insert(&mut self, element: super::PackedValue) -> bool {
        let encoded = crate::writer::encode_canonical(&element);
        self.insert_encoded(encoded, element)
    }

    /// Adds `element` under bytes the caller proved to be its canonical encoding.
    pub(crate) fn insert_encoded(&mut self, encoded: alloc::vec::Vec<u8>, element: super::PackedValue) -> bool {
        match self.elements.entry(encoded) {
            alloc::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(element);
                true
            }
            alloc::collections::btree_map::Entry::Occupied(_) => false,
        }
    }

    pub fn contains(&self, element: &super::PackedValue) -> bool {
        self.elements.contains_key(&crate::writer::encode_canonical(element))
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Elements in canonical packed byte order.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &super::PackedValue> {
        self.elements.values()
    }

    /// Canonical element encodings in wire order.
    pub fn encoded(&self) -> impl DoubleEndedIterator<Item = &[u8]> {
        self.elements.keys().map(alloc::vec::Vec::as_slice)
    }
}

impl FromIterator<super::PackedValue> for PackedSet {
    fn from_iter<I: IntoIterator<Item = super::PackedValue>>(elements: I) -> Self {
        let mut set = Self::new();
        for element in elements {
            set.insert(element);
        }
        set
    }
}

/// A dictionary with distinct keys in canonical packed byte order of the keys.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PackedDictionary {
    entries: alloc::collections::BTreeMap<alloc::vec::Vec<u8>, (super::PackedValue, super::PackedValue)>,
}

impl PackedDictionary {
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds `key` to `value`, returning the value it replaces.
    pub fn insert(&mut self, key: super::PackedValue, value: super::PackedValue) -> Option<super::PackedValue> {
        let encoded = crate::writer::encode_canonical(&key);
        self.entries.insert(encoded, (key, value)).map(|(_, previous)| previous)
    }

    /// Binds a key the caller proved absent and canonically encoded as `encoded`; false when the
    /// key is already bound.
    pub(crate) fn insert_encoded(
        &mut self,
        encoded: alloc::vec::Vec<u8>,
        key: super::PackedValue,
        value: super::PackedValue,
    ) -> bool {
        match self.entries.entry(encoded) {
            alloc::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert((key, value));
                true
            }
            alloc::collections::btree_map::Entry::Occupied(_) => false,
        }
    }

    pub fn get(&self, key: &super::PackedValue) -> Option<&super::PackedValue> {
        self.entries.get(&crate::writer::encode_canonical(key)).map(|(_, value)| value)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Entries in canonical packed byte order of their keys.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (&super::PackedValue, &super::PackedValue)> {
        self.entries.values().map(|(key, value)| (key, value))
    }

    /// Canonical key encodings with their values, in wire order.
    pub fn encoded_entries(&self) -> impl DoubleEndedIterator<Item = (&[u8], &super::PackedValue)> {
        self.entries.iter().map(|(encoded, (_, value))| (encoded.as_slice(), value))
    }
}

impl FromIterator<(super::PackedValue, super::PackedValue)> for PackedDictionary {
    fn from_iter<I: IntoIterator<Item = (super::PackedValue, super::PackedValue)>>(entries: I) -> Self {
        let mut dictionary = Self::new();
        for (key, value) in entries {
            dictionary.insert(key, value);
        }
        dictionary
    }
}
