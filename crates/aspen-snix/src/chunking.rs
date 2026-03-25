//! Content-defined chunking for blob deduplication.
//!
//! Uses FastCDC to split blobs into variable-size chunks with BLAKE3 per-chunk
//! hashing. Chunks are sized between [`MIN_CHUNK_SIZE`] and [`MAX_CHUNK_SIZE`]
//! with an average of [`AVG_CHUNK_SIZE`].

use blake3::Hash;
use fastcdc::v2020::FastCDC;

/// Minimum chunk size in bytes (16 KiB).
pub const MIN_CHUNK_SIZE: u32 = 16 * 1024;

/// Average chunk size in bytes (64 KiB).
pub const AVG_CHUNK_SIZE: u32 = 64 * 1024;

/// Maximum chunk size in bytes (256 KiB).
pub const MAX_CHUNK_SIZE: u32 = 256 * 1024;

/// Blobs below this size are stored whole without chunking.
pub const INLINE_THRESHOLD: u64 = 256 * 1024;

/// A single chunk produced by [`chunk_blob`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// BLAKE3 hash of this chunk's data.
    pub hash: Hash,
    /// Byte offset within the original blob.
    pub offset: u64,
    /// Size of this chunk in bytes.
    pub size: u32,
}

/// Split a blob into content-defined chunks using FastCDC with BLAKE3 hashes.
///
/// Returns an empty vec if `data` is empty.
pub fn chunk_blob(data: &[u8]) -> Vec<Chunk> {
    if data.is_empty() {
        return Vec::new();
    }

    let chunker = FastCDC::new(data, MIN_CHUNK_SIZE, AVG_CHUNK_SIZE, MAX_CHUNK_SIZE);
    let mut chunks = Vec::new();

    for entry in chunker {
        let chunk_data = &data[entry.offset..entry.offset + entry.length];
        let hash = blake3::hash(chunk_data);
        chunks.push(Chunk {
            hash,
            offset: entry.offset as u64,
            size: entry.length as u32,
        });
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_blob_produces_no_chunks() {
        assert!(chunk_blob(&[]).is_empty());
    }

    #[test]
    fn small_blob_single_chunk() {
        let data = vec![42u8; 1024]; // 1 KiB — below min chunk
        let chunks = chunk_blob(&data);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].offset, 0);
        assert_eq!(chunks[0].size, 1024);
        assert_eq!(chunks[0].hash, blake3::hash(&data));
    }

    #[test]
    fn large_blob_multiple_chunks() {
        // 1 MiB of pseudo-random data (varying content for CDC boundaries)
        let data: Vec<u8> = (0..1_048_576u32).map(|i| (i.wrapping_mul(2654435761)) as u8).collect();
        let chunks = chunk_blob(&data);

        assert!(chunks.len() > 1, "expected multiple chunks, got {}", chunks.len());

        // Verify coverage: chunks must cover the entire blob
        let total_size: u64 = chunks.iter().map(|c| c.size as u64).sum();
        assert_eq!(total_size, data.len() as u64);

        // Verify contiguity
        let mut expected_offset = 0u64;
        for chunk in &chunks {
            assert_eq!(chunk.offset, expected_offset);
            expected_offset += chunk.size as u64;
        }

        // Verify each chunk hash
        for chunk in &chunks {
            let chunk_data = &data[chunk.offset as usize..(chunk.offset + chunk.size as u64) as usize];
            assert_eq!(chunk.hash, blake3::hash(chunk_data));
        }
    }

    #[test]
    fn chunk_sizes_within_bounds() {
        let data: Vec<u8> = (0..2_097_152u32).map(|i| (i.wrapping_mul(2654435761)) as u8).collect();
        let chunks = chunk_blob(&data);

        for (i, chunk) in chunks.iter().enumerate() {
            // Last chunk can be smaller than min
            if i < chunks.len() - 1 {
                assert!(chunk.size >= MIN_CHUNK_SIZE, "chunk {} size {} below min {}", i, chunk.size, MIN_CHUNK_SIZE);
            }
            assert!(chunk.size <= MAX_CHUNK_SIZE, "chunk {} size {} above max {}", i, chunk.size, MAX_CHUNK_SIZE);
        }
    }

    #[test]
    fn deterministic_chunking() {
        let data: Vec<u8> = (0..512_000u32).map(|i| (i.wrapping_mul(2654435761)) as u8).collect();
        let chunks1 = chunk_blob(&data);
        let chunks2 = chunk_blob(&data);
        assert_eq!(chunks1, chunks2);
    }
}
