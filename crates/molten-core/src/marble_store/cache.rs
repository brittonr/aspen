use std::collections::BTreeMap;
use std::collections::VecDeque;

use super::*;

/// A bounded first-in, first-out cache of committed block payloads.
///
/// Marble reads always touch the heap, so committed payloads are cached in
/// front of it. Content-addressed blocks are immutable, so a cached payload is
/// valid until its block is deleted; deletions evict explicitly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadCache {
    capacity: u32,
    payloads: BTreeMap<Digest, Vec<u8>>,
    order: VecDeque<Digest>,
}

impl ReadCache {
    pub fn new(capacity: u32) -> Self {
        let bounded = capacity.min(MAX_READ_CACHE_ENTRIES);
        Self {
            capacity: bounded,
            payloads: BTreeMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn get(&self, digest: &Digest) -> Option<&[u8]> {
        self.payloads.get(digest).map(Vec::as_slice)
    }

    /// Cache a payload, evicting the oldest entry when full.
    pub fn insert(&mut self, digest: Digest, payload: &[u8]) {
        if self.capacity == 0 || self.payloads.contains_key(&digest) {
            return;
        }
        let is_full = u32::try_from(self.order.len()).map_or(true, |length| length >= self.capacity);
        if is_full && let Some(oldest) = self.order.pop_front() {
            self.payloads.remove(&oldest);
        }
        self.order.push_back(digest);
        self.payloads.insert(digest, payload.to_vec());
    }

    /// Drop a deleted block from the cache.
    pub fn evict(&mut self, digest: &Digest) {
        if self.payloads.remove(digest).is_some() {
            self.order.retain(|cached| cached != digest);
        }
    }

    /// Drop every entry, as after a reopen.
    pub fn clear(&mut self) {
        self.payloads.clear();
        self.order.clear();
    }
}
