//! The decoded immutable-node cache.
//!
//! The cache holds verified decodes of immutable blocks. An entry is keyed by
//! the canonical node reference, the profile, and the codec version, and it
//! keeps the exact bytes it decoded: a hit requires the same identity key and
//! byte-identical input, so a block whose bytes differ from the cached bytes is
//! a miss and is decoded and verified again. A cached entry is never replaced
//! or mutated under its key.
//!
//! Retention is advisory. The cache changes how often a block is decoded and
//! nothing else: canonical roots, diff records, validation outcomes, ordering,
//! and admission are identical for every retention setting, including a
//! disabled cache. The work counters an operation reports are measurements and
//! do vary with retention.
//!
//! The policy is a segmented least-recently-used cache. A new entry enters the
//! probation segment; a hit in probation promotes it to the protected segment.
//! A scan of blocks seen once only cycles the probation segment, so a hot
//! working set in the protected segment survives the scan. Eviction order
//! comes from a logical tick, never from a clock.

use std::collections::BTreeMap;

use super::*;

/// The bound on the entries one cache segment may hold.
pub const MAX_DECODED_NODE_CACHE_SEGMENT: usize = 65_536;

/// The identity of a cached decoded node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DecodedNodeKey {
    pub node_ref: NodeRef,
    pub profile_ref: ProfileRef,
    pub format_version: u32,
    pub node_codec: String,
}

impl DecodedNodeKey {
    /// The key of a block decoded under a profile.
    pub fn for_block(profile: &ProllyProfile, node_ref: &NodeRef) -> Self {
        Self {
            node_ref: node_ref.clone(),
            profile_ref: profile.profile_ref.clone(),
            format_version: profile.format_version,
            node_codec: profile.node_codec.clone(),
        }
    }
}

/// The observations a cache records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DecodedNodeCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub insertions: u64,
    pub promotions: u64,
    pub evictions: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Segment {
    Probation,
    Protected,
}

#[derive(Debug, Clone)]
struct CachedNode {
    bytes: Vec<u8>,
    node: ProllyNode,
    tick: u64,
    segment: Segment,
}

/// A bounded cache of verified, decoded immutable nodes.
#[derive(Debug, Clone)]
pub struct DecodedNodeCache {
    probation_capacity: usize,
    protected_capacity: usize,
    entries: BTreeMap<DecodedNodeKey, CachedNode>,
    probation: BTreeMap<u64, DecodedNodeKey>,
    protected: BTreeMap<u64, DecodedNodeKey>,
    tick: u64,
    stats: DecodedNodeCacheStats,
}

impl DecodedNodeCache {
    /// A cache with the given segment capacities, each clamped to the bound.
    pub fn new(probation_capacity: usize, protected_capacity: usize) -> Self {
        Self {
            probation_capacity: probation_capacity.min(MAX_DECODED_NODE_CACHE_SEGMENT),
            protected_capacity: protected_capacity.min(MAX_DECODED_NODE_CACHE_SEGMENT),
            entries: BTreeMap::new(),
            probation: BTreeMap::new(),
            protected: BTreeMap::new(),
            tick: 0,
            stats: DecodedNodeCacheStats::default(),
        }
    }

    /// A cache that retains nothing.
    pub fn disabled() -> Self {
        Self::new(0, 0)
    }

    /// A cache that retains every node one operation decodes.
    pub fn operation_scoped(node_count: usize) -> Self {
        Self::new(node_count, 0)
    }

    /// The number of retained entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache retains nothing.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether the cache retains a node under the key.
    pub fn contains(&self, key: &DecodedNodeKey) -> bool {
        self.entries.contains_key(key)
    }

    /// Whether the key sits in the protected segment.
    pub fn is_protected(&self, key: &DecodedNodeKey) -> bool {
        self.entries.get(key).is_some_and(|entry| entry.segment == Segment::Protected)
    }

    /// The observations the cache recorded.
    pub fn stats(&self) -> DecodedNodeCacheStats {
        self.stats
    }

    /// The verified decode of exactly these bytes under the key, if retained.
    ///
    /// A hit requires byte-identical input. Bytes that differ from the cached
    /// bytes are a miss: the caller decodes and verifies them, and the cached
    /// entry stays as it is.
    pub fn get(&mut self, key: &DecodedNodeKey, bytes: &[u8]) -> Option<ProllyNode> {
        let Some(entry) = self.entries.get(key) else {
            self.stats.misses = self.stats.misses.saturating_add(1);
            return None;
        };
        if entry.bytes != bytes {
            self.stats.misses = self.stats.misses.saturating_add(1);
            return None;
        }
        let node = entry.node.clone();
        let segment = entry.segment;
        let old_tick = entry.tick;
        self.stats.hits = self.stats.hits.saturating_add(1);
        self.tick = self.tick.saturating_add(1);
        let new_tick = self.tick;
        match segment {
            Segment::Probation if self.protected_capacity > 0 => {
                self.probation.remove(&old_tick);
                self.protected.insert(new_tick, key.clone());
                self.set_position(key, new_tick, Segment::Protected);
                self.stats.promotions = self.stats.promotions.saturating_add(1);
                self.demote_protected_overflow();
            }
            Segment::Probation => {
                self.probation.remove(&old_tick);
                self.probation.insert(new_tick, key.clone());
                self.set_position(key, new_tick, Segment::Probation);
            }
            Segment::Protected => {
                self.protected.remove(&old_tick);
                self.protected.insert(new_tick, key.clone());
                self.set_position(key, new_tick, Segment::Protected);
            }
        }
        Some(node)
    }

    /// Retain a verified decode of the bytes under the key.
    ///
    /// An existing entry under the key is never replaced: cached contents are
    /// immutable under their reference.
    pub fn insert(&mut self, key: DecodedNodeKey, bytes: Vec<u8>, node: ProllyNode) {
        if self.probation_capacity == 0 || self.entries.contains_key(&key) {
            return;
        }
        self.tick = self.tick.saturating_add(1);
        self.probation.insert(self.tick, key.clone());
        self.entries.insert(key, CachedNode {
            bytes,
            node,
            tick: self.tick,
            segment: Segment::Probation,
        });
        self.stats.insertions = self.stats.insertions.saturating_add(1);
        self.evict_probation_overflow();
    }

    fn set_position(&mut self, key: &DecodedNodeKey, tick: u64, segment: Segment) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.tick = tick;
            entry.segment = segment;
        }
    }

    fn demote_protected_overflow(&mut self) {
        while self.protected.len() > self.protected_capacity {
            let Some((_, key)) = self.protected.pop_first() else {
                break;
            };
            self.tick = self.tick.saturating_add(1);
            self.probation.insert(self.tick, key.clone());
            let tick = self.tick;
            self.set_position(&key, tick, Segment::Probation);
        }
        self.evict_probation_overflow();
    }

    fn evict_probation_overflow(&mut self) {
        while self.probation.len() > self.probation_capacity {
            let Some((_, key)) = self.probation.pop_first() else {
                break;
            };
            self.entries.remove(&key);
            self.stats.evictions = self.stats.evictions.saturating_add(1);
        }
    }
}

/// Decode a block through the cache.
///
/// The decode runs, and verifies the block identity, only on a miss. The
/// returned flag says whether the block was decoded.
pub fn decode_block_cached(
    profile: &ProllyProfile,
    block: &EncodedBlock,
    cache: &mut DecodedNodeCache,
) -> Result<(ProllyNode, bool), Vec<ProllyIssue>> {
    let key = DecodedNodeKey::for_block(profile, &block.node_ref);
    if let Some(node) = cache.get(&key, &block.bytes) {
        return Ok((node, false));
    }
    let node = decode_block(profile, block)?;
    cache.insert(key, block.bytes.clone(), node.clone());
    Ok((node, true))
}
