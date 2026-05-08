//! Content-defined chunking for blob deduplication.
//!
//! The production default uses FastCDC to split blobs into variable-size chunks
//! with BLAKE3 per-chunk hashing. Chunks are sized between
//! [`MIN_CHUNK_SIZE`] and [`MAX_CHUNK_SIZE`] with an average of
//! [`AVG_CHUNK_SIZE`]. Experimental chunkers are kept behind explicit feature
//! gates and config so physical chunking can be evaluated without changing blob
//! identity or default production behavior.

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

#[cfg(feature = "experimental-vectorcdc")]
const AVG_CHUNK_MASK: u64 = (AVG_CHUNK_SIZE as u64) - 1;

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

/// Available chunking algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkerAlgorithm {
    /// Production default: `fastcdc::v2020::FastCDC`.
    FastCdc,
    /// Experimental hashless CDC candidate for VectorCDC-style evaluation.
    ///
    /// This variant is available only with the `experimental-vectorcdc` feature
    /// and is intentionally not the default.
    ExperimentalHashlessCdc,
}

impl ChunkerAlgorithm {
    /// Stable report/config name for this algorithm.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FastCdc => "fastcdc-v2020",
            Self::ExperimentalHashlessCdc => "experimental-hashless-cdc",
        }
    }
}

/// Error returned when a requested chunker is not available in this build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkingError {
    /// The algorithm requires a feature that is not enabled.
    UnsupportedAlgorithm(ChunkerAlgorithm),
}

impl core::fmt::Display for ChunkingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedAlgorithm(algorithm) => {
                write!(f, "chunking algorithm '{}' is not available in this build", algorithm.as_str())
            }
        }
    }
}

impl std::error::Error for ChunkingError {}

/// Split a blob into content-defined chunks using the production default.
///
/// Returns an empty vec if `data` is empty. This preserves the existing FastCDC
/// behavior and is the only path used by production callers unless they opt into
/// [`chunk_blob_with_algorithm`] explicitly.
pub fn chunk_blob(data: &[u8]) -> Vec<Chunk> {
    chunk_blob_fastcdc(data)
}

/// Split a blob using an explicitly selected algorithm.
///
/// Experimental candidates remain feature-gated; requesting one without the
/// matching feature returns [`ChunkingError::UnsupportedAlgorithm`].
pub fn chunk_blob_with_algorithm(data: &[u8], algorithm: ChunkerAlgorithm) -> Result<Vec<Chunk>, ChunkingError> {
    match algorithm {
        ChunkerAlgorithm::FastCdc => Ok(chunk_blob_fastcdc(data)),
        ChunkerAlgorithm::ExperimentalHashlessCdc => chunk_blob_experimental_hashless(data),
    }
}

fn chunk_blob_fastcdc(data: &[u8]) -> Vec<Chunk> {
    if data.is_empty() {
        return Vec::new();
    }

    // r[impl snix.store.chunk-size-bound]
    let chunker = FastCDC::new(data, MIN_CHUNK_SIZE, AVG_CHUNK_SIZE, MAX_CHUNK_SIZE);
    let mut chunks = Vec::new();

    for entry in chunker {
        push_chunk(&mut chunks, data, entry.offset, entry.length);
    }

    chunks
}

#[cfg(feature = "experimental-vectorcdc")]
fn chunk_blob_experimental_hashless(data: &[u8]) -> Result<Vec<Chunk>, ChunkingError> {
    Ok(chunk_blob_hashless_gear(data))
}

#[cfg(not(feature = "experimental-vectorcdc"))]
fn chunk_blob_experimental_hashless(data: &[u8]) -> Result<Vec<Chunk>, ChunkingError> {
    let _ = data;
    Err(ChunkingError::UnsupportedAlgorithm(ChunkerAlgorithm::ExperimentalHashlessCdc))
}

#[cfg(feature = "experimental-vectorcdc")]
fn chunk_blob_hashless_gear(data: &[u8]) -> Vec<Chunk> {
    if data.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut start = 0usize;
    let mut rolling = 0u64;

    for (idx, byte) in data.iter().copied().enumerate() {
        rolling = rolling.rotate_left(1) ^ gear_value(byte);
        let len = idx + 1 - start;
        let reached_min = len >= MIN_CHUNK_SIZE as usize;
        let reached_max = len >= MAX_CHUNK_SIZE as usize;
        let boundary = reached_min && (reached_max || (rolling & AVG_CHUNK_MASK) == 0);
        if boundary {
            push_chunk(&mut chunks, data, start, len);
            start = idx + 1;
            rolling = 0;
        }
    }

    if start < data.len() {
        push_chunk(&mut chunks, data, start, data.len() - start);
    }

    chunks
}

#[cfg(feature = "experimental-vectorcdc")]
fn gear_value(byte: u8) -> u64 {
    let mut value = byte as u64;
    value = value.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    value ^= value >> 33;
    value = value.wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    value ^ (value >> 29)
}

fn push_chunk(chunks: &mut Vec<Chunk>, data: &[u8], offset: usize, length: usize) {
    let chunk_data = &data[offset..offset + length];
    let hash = blake3::hash(chunk_data);
    chunks.push(Chunk {
        hash,
        offset: offset as u64,
        size: length as u32,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_chunk_invariants(data: &[u8], chunks: &[Chunk]) {
        let total_size: u64 = chunks.iter().map(|c| c.size as u64).sum();
        assert_eq!(total_size, data.len() as u64);

        let mut expected_offset = 0u64;
        for (idx, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.offset, expected_offset);
            expected_offset += chunk.size as u64;

            if idx < chunks.len() - 1 {
                assert!(chunk.size >= MIN_CHUNK_SIZE, "chunk {idx} size {} below min {MIN_CHUNK_SIZE}", chunk.size);
            }
            assert!(chunk.size <= MAX_CHUNK_SIZE, "chunk {idx} size {} above max {MAX_CHUNK_SIZE}", chunk.size);

            let chunk_data = &data[chunk.offset as usize..(chunk.offset + chunk.size as u64) as usize];
            assert_eq!(chunk.hash, blake3::hash(chunk_data));
        }
    }

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
        let data: Vec<u8> = (0..1_048_576u32).map(|i| (i.wrapping_mul(2_654_435_761)) as u8).collect();
        let chunks = chunk_blob(&data);

        assert!(chunks.len() > 1, "expected multiple chunks, got {}", chunks.len());
        assert_chunk_invariants(&data, &chunks);
    }

    #[test]
    // r[verify snix.store.chunk-size-bound]
    fn chunk_sizes_within_bounds() {
        let data: Vec<u8> = (0..2_097_152u32).map(|i| (i.wrapping_mul(2_654_435_761)) as u8).collect();
        let chunks = chunk_blob(&data);
        assert_chunk_invariants(&data, &chunks);
    }

    #[test]
    fn deterministic_chunking() {
        let data: Vec<u8> = (0..512_000u32).map(|i| (i.wrapping_mul(2_654_435_761)) as u8).collect();
        let chunks1 = chunk_blob(&data);
        let chunks2 = chunk_blob(&data);
        assert_eq!(chunks1, chunks2);
    }

    #[test]
    fn explicit_fastcdc_matches_default() {
        let data: Vec<u8> = (0..768_000u32).map(|i| (i.wrapping_mul(2_654_435_761)) as u8).collect();
        assert_eq!(
            chunk_blob(&data),
            chunk_blob_with_algorithm(&data, ChunkerAlgorithm::FastCdc).expect("fastcdc supported")
        );
    }

    #[cfg(not(feature = "experimental-vectorcdc"))]
    #[test]
    fn experimental_hashless_candidate_is_feature_gated() {
        let error = chunk_blob_with_algorithm(&[1, 2, 3], ChunkerAlgorithm::ExperimentalHashlessCdc)
            .expect_err("candidate must be unavailable without explicit feature");
        assert_eq!(error, ChunkingError::UnsupportedAlgorithm(ChunkerAlgorithm::ExperimentalHashlessCdc));
    }

    #[cfg(feature = "experimental-vectorcdc")]
    #[test]
    fn experimental_hashless_candidate_preserves_chunk_invariants() {
        let data: Vec<u8> =
            (0..2_097_152u32).map(|i| (i.wrapping_mul(2_654_435_761).rotate_left(i % 31)) as u8).collect();
        let chunks =
            chunk_blob_with_algorithm(&data, ChunkerAlgorithm::ExperimentalHashlessCdc).expect("candidate enabled");
        assert!(chunks.len() > 1);
        assert_chunk_invariants(&data, &chunks);
        assert_eq!(
            chunks,
            chunk_blob_with_algorithm(&data, ChunkerAlgorithm::ExperimentalHashlessCdc).expect("candidate enabled")
        );
    }
}
