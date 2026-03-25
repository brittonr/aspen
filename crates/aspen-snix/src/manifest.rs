//! Chunk manifest for chunked blob storage.
//!
//! A manifest records the ordered list of chunk hashes and sizes that
//! compose a blob. Serialized with postcard for compact storage.

use serde::Deserialize;
use serde::Serialize;

/// Maximum manifest size that can be stored inline in KV (64 KiB).
/// Larger manifests are stored as blobs themselves.
pub const MAX_INLINE_MANIFEST_SIZE: usize = 64 * 1024;

/// A single entry in a chunk manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// BLAKE3 hash of the chunk (32 bytes).
    pub hash: [u8; 32],
    /// Size of the chunk in bytes.
    pub size: u32,
}

/// Ordered manifest of chunks composing a blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// Chunk entries in order.
    pub entries: Vec<ManifestEntry>,
    /// Total size of the original blob in bytes.
    pub total_size: u64,
}

impl Manifest {
    /// Create a manifest from chunk entries.
    // r[impl snix.store.manifest-entry-bound]
    pub fn new(entries: Vec<ManifestEntry>) -> Self {
        let total_size = entries.iter().map(|e| e.size as u64).sum();
        Self { entries, total_size }
    }

    /// Number of chunks in the manifest.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the manifest is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Serialize to postcard bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(self)
    }

    /// Deserialize from postcard bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
    }

    /// Whether this manifest is small enough to store inline in KV.
    pub fn is_inline(&self) -> bool {
        // Estimate: 36 bytes per entry (32 hash + 4 size) + 8 (total_size) + overhead
        let estimated = self.entries.len() * 36 + 16;
        estimated <= MAX_INLINE_MANIFEST_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty() {
        let m = Manifest::new(vec![]);
        assert!(m.is_empty());
        assert_eq!(m.total_size, 0);

        let bytes = m.to_bytes().unwrap();
        let m2 = Manifest::from_bytes(&bytes).unwrap();
        assert_eq!(m, m2);
    }

    #[test]
    fn roundtrip_single_entry() {
        let m = Manifest::new(vec![ManifestEntry {
            hash: [42u8; 32],
            size: 65536,
        }]);
        assert_eq!(m.len(), 1);
        assert_eq!(m.total_size, 65536);

        let bytes = m.to_bytes().unwrap();
        let m2 = Manifest::from_bytes(&bytes).unwrap();
        assert_eq!(m, m2);
    }

    #[test]
    // r[verify snix.store.manifest-entry-bound]
    fn roundtrip_many_entries() {
        let entries: Vec<ManifestEntry> = (0..100u8)
            .map(|i| ManifestEntry {
                hash: [i; 32],
                size: 64 * 1024 + i as u32 * 100,
            })
            .collect();
        let m = Manifest::new(entries);
        assert_eq!(m.len(), 100);

        let bytes = m.to_bytes().unwrap();
        let m2 = Manifest::from_bytes(&bytes).unwrap();
        assert_eq!(m, m2);
    }

    #[test]
    fn total_size_computed() {
        let m = Manifest::new(vec![
            ManifestEntry {
                hash: [0u8; 32],
                size: 1000,
            },
            ManifestEntry {
                hash: [1u8; 32],
                size: 2000,
            },
            ManifestEntry {
                hash: [2u8; 32],
                size: 3000,
            },
        ]);
        assert_eq!(m.total_size, 6000);
    }

    #[test]
    fn inline_threshold() {
        // Small manifest should be inline
        let small = Manifest::new(vec![ManifestEntry {
            hash: [0u8; 32],
            size: 1024,
        }]);
        assert!(small.is_inline());

        // Large manifest (>64KiB worth of entries) should not be inline
        let entries: Vec<ManifestEntry> = (0..2000u16)
            .map(|i| ManifestEntry {
                hash: [(i % 256) as u8; 32],
                size: 64 * 1024,
            })
            .collect();
        let large = Manifest::new(entries);
        assert!(!large.is_inline());
    }
}
