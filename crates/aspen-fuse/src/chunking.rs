//! Transparent file chunking for large files.
//!
//! Files larger than CHUNK_SIZE (512 KB) are automatically split into chunks
//! stored as separate KV entries. Files ≤ CHUNK_SIZE remain as single entries
//! for backwards compatibility.
//!
//! ## Storage Layout
//!
//! **Small file** (≤ 512 KB):
//! - `key` → raw file data
//!
//! **Large file** (> 512 KB):
//! - `key` → manifest: `CHUNK_MANIFEST_MAGIC` + postcard-serialized `ChunkManifest`
//! - `key.chunk.000000` → first 512 KB
//! - `key.chunk.000001` → next 512 KB
//! - etc.
//!
//! ## Chunking Functions
//!
//! These are free functions that take `&AspenFs` and call the raw KV methods
//! (kv_read_raw, kv_write_raw, kv_delete_raw) which bypass the MAX_VALUE_SIZE
//! check since individual chunks are within limits.
//!
//! NOTE: The following methods need to be added to AspenFs as pub(crate):
//! - `kv_read_raw(&self, key: &str) -> io::Result<Option<Vec<u8>>>`
//! - `kv_write_raw(&self, key: &str, value: &[u8]) -> io::Result<()>`
//! - `kv_delete_raw(&self, key: &str) -> io::Result<()>`
//!
//! These are identical to kv_read/kv_write/kv_delete but skip the MAX_VALUE_SIZE
//! check for chunk storage.

use std::io;

use serde::Deserialize;
use serde::Serialize;

use crate::constants::CHUNK_KEY_SUFFIX;
use crate::constants::CHUNK_MANIFEST_MAGIC;
use crate::constants::CHUNK_SIZE;
use crate::constants::CHUNK_SIZE_U32;
use crate::constants::MAX_FILE_SIZE;
use crate::fs::AspenFs;

fn usize_from_u64(value: u64) -> io::Result<usize> {
    usize::try_from(value).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "value does not fit in usize"))
}

fn usize_from_u32(value: u32) -> io::Result<usize> {
    usize::try_from(value).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "value does not fit in usize"))
}

fn u64_from_usize(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn checked_div_u64(numerator: u64, denominator: u64) -> io::Result<u64> {
    numerator
        .checked_div(denominator)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "division by zero"))
}

/// Chunk manifest stored at the main key for chunked files.
///
/// The manifest describes how the file is split into chunks.
/// Serialized using postcard and prefixed with CHUNK_MANIFEST_MAGIC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkManifest {
    /// Total file size in bytes.
    pub total_size: u64,
    /// Size of each chunk in bytes (except possibly the last chunk).
    pub chunk_size: u32,
    /// Number of chunks.
    pub chunk_count: u32,
}

impl ChunkManifest {
    /// Create a new chunk manifest for a file.
    ///
    /// # Errors
    ///
    /// Returns error if file_size exceeds MAX_FILE_SIZE.
    pub fn new(file_size: u64) -> io::Result<Self> {
        if file_size > MAX_FILE_SIZE {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "file too large"));
        }

        let chunk_size_bytes = CHUNK_SIZE_U32;
        if chunk_size_bytes == 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "chunk size must be non-zero"));
        }
        let chunk_count_u64 = checked_div_u64(
            file_size.saturating_add(u64::from(chunk_size_bytes).saturating_sub(1)),
            u64::from(chunk_size_bytes),
        )?;
        let chunk_count = u32::try_from(chunk_count_u64)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "chunk count exceeds u32"))?;

        Ok(Self {
            total_size: file_size,
            chunk_size: chunk_size_bytes,
            chunk_count,
        })
    }

    /// Serialize manifest to bytes (with magic prefix).
    pub fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut result = Vec::with_capacity(CHUNK_MANIFEST_MAGIC.len().saturating_add(256));
        result.extend_from_slice(CHUNK_MANIFEST_MAGIC);

        let serialized = postcard::to_allocvec(self).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        result.extend_from_slice(&serialized);

        Ok(result)
    }

    /// Deserialize manifest from bytes (expects magic prefix).
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if !is_chunk_manifest(data) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "not a chunk manifest"));
        }

        let payload = &data[CHUNK_MANIFEST_MAGIC.len()..];
        postcard::from_bytes(payload).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Get the size of a specific chunk.
    ///
    /// All chunks except possibly the last are chunk_size bytes.
    pub fn chunk_size_at(&self, chunk_index: u32) -> u32 {
        if chunk_index >= self.chunk_count {
            return 0;
        }

        if chunk_index == self.chunk_count.saturating_sub(1) {
            // Last chunk may be smaller
            let Some(remainder) = self.total_size.checked_rem(u64::from(self.chunk_size)) else {
                return self.chunk_size;
            };
            if remainder == 0 {
                self.chunk_size
            } else {
                u32::try_from(remainder).unwrap_or(self.chunk_size)
            }
        } else {
            self.chunk_size
        }
    }

    /// Calculate which chunk contains a given byte offset.
    pub fn chunk_index_for_offset(&self, offset: u64) -> u32 {
        if self.chunk_size == 0 {
            return 0;
        }
        let Some(index) = offset.checked_div(u64::from(self.chunk_size)) else {
            return 0;
        };
        u32::try_from(index.min(u64::from(self.chunk_count.saturating_sub(1))))
            .unwrap_or(self.chunk_count.saturating_sub(1))
    }

    /// Calculate the byte offset within a chunk for a given file offset.
    pub fn offset_within_chunk(&self, offset: u64) -> u32 {
        if self.chunk_size == 0 {
            return 0;
        }
        let Some(chunk_offset_u64) = offset.checked_rem(u64::from(self.chunk_size)) else {
            return 0;
        };
        u32::try_from(chunk_offset_u64).unwrap_or(0)
    }
}

/// Check if a value is a chunk manifest (starts with CHUNK_MANIFEST_MAGIC).
pub fn is_chunk_manifest(data: &[u8]) -> bool {
    data.starts_with(CHUNK_MANIFEST_MAGIC)
}

/// Build a chunk key from a base key and chunk index.
///
/// Format: `{key}{CHUNK_KEY_SUFFIX}{index:06}`
/// Example: `foo/bar.chunk.000000`, `foo/bar.chunk.000001`
fn chunk_key(base_key: &str, chunk_index: u32) -> String {
    format!("{}{}{:06}", base_key, CHUNK_KEY_SUFFIX, chunk_index)
}

/// Read a file's contents, transparently reassembling chunks if needed.
///
/// Returns `None` if the file doesn't exist, `Some(data)` otherwise.
/// Handles both chunked and non-chunked files.
pub fn chunked_read(fs: &AspenFs, key: &str) -> io::Result<Option<Vec<u8>>> {
    // NOTE: This should call fs.kv_read_raw(key) once that method exists.
    // For now, this is a placeholder showing the expected implementation.
    let value = fs.kv_read(key)?;
    debug_assert!(!key.is_empty());

    let Some(value) = value else {
        return Ok(None);
    };

    // Check if it's a manifest or raw data
    if !is_chunk_manifest(&value) {
        // Small file: return as-is
        return Ok(Some(value));
    }

    // Large file: read manifest and reassemble chunks
    let manifest = ChunkManifest::from_bytes(&value)?;

    if manifest.total_size == 0 {
        return Ok(Some(Vec::new()));
    }

    let mut result = Vec::with_capacity(usize_from_u64(manifest.total_size)?);

    for chunk_index in 0..manifest.chunk_count {
        let chunk_k = chunk_key(key, chunk_index);
        let chunk_data =
            fs.kv_read(&chunk_k)?.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "chunk missing"))?;

        result.extend_from_slice(&chunk_data);
    }

    // Verify we got the expected size
    debug_assert!(manifest.chunk_count == 0 || manifest.chunk_size > 0);
    if result.len() != usize_from_u64(manifest.total_size)? {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "size mismatch"));
    }

    Ok(Some(result))
}

/// Read a range of a file, only fetching the affected chunks.
///
/// Reads `size` bytes starting at `offset`. Returns fewer bytes if EOF is reached.
pub fn chunked_read_range(fs: &AspenFs, key: &str, offset: u64, size: u32) -> io::Result<Vec<u8>> {
    let value = fs.kv_read(key)?.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "file not found"))?;

    if !is_chunk_manifest(&value) {
        let start_bytes = usize_from_u64(offset)?;
        if start_bytes >= value.len() {
            return Ok(Vec::new());
        }
        let end_bytes = start_bytes.saturating_add(usize_from_u32(size)?).min(value.len());
        debug_assert!(start_bytes <= end_bytes);
        return Ok(value[start_bytes..end_bytes].to_vec());
    }

    let manifest = ChunkManifest::from_bytes(&value)?;
    debug_assert!(manifest.chunk_count == 0 || manifest.chunk_size > 0);

    if offset >= manifest.total_size {
        return Ok(Vec::new());
    }

    let end_offset_bytes = offset.saturating_add(u64::from(size)).min(manifest.total_size);
    let bytes_to_read_bytes = end_offset_bytes.saturating_sub(offset);
    let mut result = Vec::with_capacity(usize_from_u64(bytes_to_read_bytes)?);

    let start_chunk = manifest.chunk_index_for_offset(offset);
    let end_chunk = manifest.chunk_index_for_offset(end_offset_bytes.saturating_sub(1));

    for chunk_index in start_chunk..=end_chunk {
        if chunk_index >= manifest.chunk_count {
            break;
        }

        let chunk_k = chunk_key(key, chunk_index);
        let chunk_data =
            fs.kv_read(&chunk_k)?.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "chunk missing"))?;

        let chunk_start_offset_bytes = u64::from(chunk_index).saturating_mul(u64::from(manifest.chunk_size));
        let chunk_end_offset_bytes = chunk_start_offset_bytes.saturating_add(u64_from_usize(chunk_data.len()));

        let read_start_bytes = if offset > chunk_start_offset_bytes {
            usize_from_u64(offset.saturating_sub(chunk_start_offset_bytes))?
        } else {
            0
        };

        let read_end_bytes = if end_offset_bytes < chunk_end_offset_bytes {
            usize_from_u64(end_offset_bytes.saturating_sub(chunk_start_offset_bytes))?
        } else {
            chunk_data.len()
        };

        if read_start_bytes < chunk_data.len()
            && read_end_bytes <= chunk_data.len()
            && read_start_bytes <= read_end_bytes
        {
            result.extend_from_slice(&chunk_data[read_start_bytes..read_end_bytes]);
        }
    }

    debug_assert!(result.len() <= usize_from_u64(bytes_to_read_bytes).unwrap_or(usize::MAX));
    Ok(result)
}

/// Write a file's contents, splitting into chunks if needed.
///
/// If `data.len() <= CHUNK_SIZE`, stores as a single KV entry (backwards compatible).
/// Otherwise, creates a manifest and stores chunks.
pub fn chunked_write(fs: &AspenFs, key: &str, data: &[u8]) -> io::Result<()> {
    let data_len = u64::try_from(data.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "file length exceeds u64"))?;
    debug_assert!(!key.is_empty());

    if data_len > MAX_FILE_SIZE {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "file too large"));
    }

    // Small file: write directly (backwards compatible)
    let chunk_threshold_bytes = u64::try_from(CHUNK_SIZE).unwrap_or(u64::MAX);
    if data_len <= chunk_threshold_bytes {
        // Delete any existing chunks first (in case we're overwriting a large file)
        chunked_delete(fs, key)?;
        return fs.kv_write(key, data);
    }

    // Large file: create manifest and chunks
    let manifest = ChunkManifest::new(data_len)?;

    // Write chunks first
    for chunk_index in 0..manifest.chunk_count {
        let chunk_start = usize_from_u64(u64::from(chunk_index).saturating_mul(u64::from(manifest.chunk_size)))?;
        let chunk_end = chunk_start.saturating_add(usize_from_u32(manifest.chunk_size)?).min(data.len());

        let chunk_data = &data[chunk_start..chunk_end];
        let chunk_k = chunk_key(key, chunk_index);

        fs.kv_write(&chunk_k, chunk_data)?;
    }

    // Write manifest last (makes the chunked file visible atomically)
    debug_assert!(manifest.chunk_count > 0);
    let manifest_bytes = manifest.to_bytes()?;
    fs.kv_write(key, &manifest_bytes)?;

    Ok(())
}

/// Write a range within a file, only updating affected chunks.
///
/// Performs a read-modify-write on the affected chunks.
/// If the write extends beyond the current file size, the file is extended.
pub fn chunked_write_range(fs: &AspenFs, key: &str, offset: u64, data: &[u8]) -> io::Result<()> {
    if data.is_empty() {
        return Ok(());
    }

    let existing = fs.kv_read(key)?;
    let end_offset_bytes = offset.saturating_add(u64_from_usize(data.len()));
    if end_offset_bytes > MAX_FILE_SIZE {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "file would be too large"));
    }

    match existing {
        None => {
            let mut new_data = vec![0u8; usize_from_u64(offset)?];
            new_data.extend_from_slice(data);
            debug_assert_eq!(new_data.len(), usize_from_u64(end_offset_bytes).unwrap_or(usize::MAX));
            chunked_write(fs, key, &new_data)
        }
        Some(ref value) if !is_chunk_manifest(value) => {
            let mut file_data = value.clone();
            let required_size_bytes = usize_from_u64(end_offset_bytes)?;
            if file_data.len() < required_size_bytes {
                file_data.resize(required_size_bytes, 0);
            }

            let write_start_bytes = usize_from_u64(offset)?;
            let write_end_bytes = usize_from_u64(end_offset_bytes)?;
            debug_assert!(write_start_bytes <= write_end_bytes);
            file_data[write_start_bytes..write_end_bytes].copy_from_slice(data);
            chunked_write(fs, key, &file_data)
        }
        Some(ref value) => {
            let manifest = ChunkManifest::from_bytes(value)?;
            let new_size_bytes = end_offset_bytes.max(manifest.total_size);
            let new_manifest = ChunkManifest::new(new_size_bytes)?;

            let start_chunk = new_manifest.chunk_index_for_offset(offset);
            let end_chunk = new_manifest.chunk_index_for_offset(end_offset_bytes.saturating_sub(1));

            for chunk_index in start_chunk..=end_chunk {
                let chunk_k = chunk_key(key, chunk_index);
                let chunk_capacity_bytes = usize_from_u32(new_manifest.chunk_size)?;
                let mut chunk_data = if chunk_index < manifest.chunk_count {
                    fs.kv_read(&chunk_k)?.unwrap_or_else(|| vec![0u8; chunk_capacity_bytes])
                } else {
                    vec![0u8; chunk_capacity_bytes]
                };

                let expected_chunk_size_bytes = usize_from_u32(new_manifest.chunk_size_at(chunk_index))?;
                if chunk_data.len() < expected_chunk_size_bytes {
                    chunk_data.resize(expected_chunk_size_bytes, 0);
                }

                let chunk_start_offset_bytes =
                    u64::from(chunk_index).saturating_mul(u64::from(new_manifest.chunk_size));
                let chunk_end_offset_bytes = chunk_start_offset_bytes.saturating_add(u64_from_usize(chunk_data.len()));

                let write_start_in_chunk_bytes = if offset > chunk_start_offset_bytes {
                    usize_from_u64(offset.saturating_sub(chunk_start_offset_bytes))?
                } else {
                    0
                };

                let write_end_in_chunk_bytes = if end_offset_bytes < chunk_end_offset_bytes {
                    usize_from_u64(end_offset_bytes.saturating_sub(chunk_start_offset_bytes))?
                } else {
                    chunk_data.len()
                };

                let data_start_bytes = if chunk_start_offset_bytes > offset {
                    usize_from_u64(chunk_start_offset_bytes.saturating_sub(offset))?
                } else {
                    0
                };

                let data_end_bytes = data_start_bytes
                    .saturating_add(write_end_in_chunk_bytes.saturating_sub(write_start_in_chunk_bytes));

                if data_end_bytes <= data.len()
                    && write_start_in_chunk_bytes < chunk_data.len()
                    && write_end_in_chunk_bytes <= chunk_data.len()
                {
                    chunk_data[write_start_in_chunk_bytes..write_end_in_chunk_bytes]
                        .copy_from_slice(&data[data_start_bytes..data_end_bytes]);
                }

                let final_chunk_size_bytes = usize_from_u32(new_manifest.chunk_size_at(chunk_index))?;
                if chunk_data.len() > final_chunk_size_bytes {
                    chunk_data.truncate(final_chunk_size_bytes);
                }

                fs.kv_write(&chunk_k, &chunk_data)?;
            }

            if new_size_bytes != manifest.total_size {
                let manifest_bytes = new_manifest.to_bytes()?;
                fs.kv_write(key, &manifest_bytes)?;
            }

            Ok(())
        }
    }
}

/// Delete a file and all its chunks.
///
/// Handles both chunked and non-chunked files.
pub fn chunked_delete(fs: &AspenFs, key: &str) -> io::Result<()> {
    // Read to check if it's chunked
    let value = fs.kv_read(key)?;

    if let Some(value) = value
        && is_chunk_manifest(&value)
    {
        // Delete all chunks first
        let manifest = ChunkManifest::from_bytes(&value)?;

        for chunk_index in 0..manifest.chunk_count {
            let chunk_k = chunk_key(key, chunk_index);
            if let Err(error) = fs.kv_delete(&chunk_k)
                && error.kind() != io::ErrorKind::NotFound
            {
                return Err(error);
            }
        }
    }

    // Delete the main key (manifest or raw data)
    fs.kv_delete(key)?;

    Ok(())
}

/// Get the size of a file (from manifest or raw data length).
///
/// Returns `None` if the file doesn't exist.
pub fn chunked_size(fs: &AspenFs, key: &str) -> io::Result<Option<u64>> {
    let value = fs.kv_read(key)?;

    let Some(value) = value else {
        return Ok(None);
    };

    if is_chunk_manifest(&value) {
        let manifest = ChunkManifest::from_bytes(&value)?;
        Ok(Some(manifest.total_size))
    } else {
        Ok(Some(u64::try_from(value.len()).unwrap_or(u64::MAX)))
    }
}

/// Rename source and destination keys for chunked rename.
pub struct RenameKeys<'a> {
    /// Existing key to move from.
    pub old_key: &'a str,
    /// New key to move to.
    pub new_key: &'a str,
}

/// Rename a file and all its chunks.
///
/// Handles both chunked and non-chunked files.
pub fn chunked_rename(fs: &AspenFs, keys: RenameKeys<'_>) -> io::Result<()> {
    // Read to check if it's chunked
    let value = fs.kv_read(keys.old_key)?.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "file not found"))?;

    if is_chunk_manifest(&value) {
        // Chunked file: rename manifest and all chunks
        let manifest = ChunkManifest::from_bytes(&value)?;

        // Copy chunks to new location
        for chunk_index in 0..manifest.chunk_count {
            let old_chunk = chunk_key(keys.old_key, chunk_index);
            let new_chunk = chunk_key(keys.new_key, chunk_index);

            if let Some(chunk_data) = fs.kv_read(&old_chunk)? {
                fs.kv_write(&new_chunk, &chunk_data)?;
            }
        }

        // Copy manifest
        fs.kv_write(keys.new_key, &value)?;

        // Delete old chunks
        for chunk_index in 0..manifest.chunk_count {
            let old_chunk = chunk_key(keys.old_key, chunk_index);
            if let Err(error) = fs.kv_delete(&old_chunk)
                && error.kind() != io::ErrorKind::NotFound
            {
                return Err(error);
            }
        }
    } else {
        // Small file: just copy the value
        fs.kv_write(keys.new_key, &value)?;
    }

    // Delete old manifest/data
    debug_assert!(!keys.old_key.is_empty());
    debug_assert!(!keys.new_key.is_empty());
    fs.kv_delete(keys.old_key)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_serialization() {
        let manifest = ChunkManifest {
            total_size: 1_000_000,
            chunk_size: CHUNK_SIZE as u32,
            chunk_count: 2,
        };

        let bytes = manifest.to_bytes().expect("serialize failed");
        assert!(is_chunk_manifest(&bytes));

        let decoded = ChunkManifest::from_bytes(&bytes).expect("deserialize failed");
        assert_eq!(manifest, decoded);
    }

    #[test]
    fn test_manifest_new() {
        // Exactly one chunk
        let manifest = ChunkManifest::new(CHUNK_SIZE as u64).expect("new failed");
        assert_eq!(manifest.total_size, CHUNK_SIZE as u64);
        assert_eq!(manifest.chunk_size, CHUNK_SIZE as u32);
        assert_eq!(manifest.chunk_count, 1);

        // Two chunks
        let manifest = ChunkManifest::new(CHUNK_SIZE as u64 + 1).expect("new failed");
        assert_eq!(manifest.total_size, CHUNK_SIZE as u64 + 1);
        assert_eq!(manifest.chunk_count, 2);

        // Empty file
        let manifest = ChunkManifest::new(0).expect("new failed");
        assert_eq!(manifest.chunk_count, 0);

        // File too large
        let result = ChunkManifest::new(MAX_FILE_SIZE + 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_chunk_size_at() {
        let manifest = ChunkManifest::new(CHUNK_SIZE as u64 + 100).expect("new failed");
        assert_eq!(manifest.chunk_count, 2);

        // First chunk is full size
        assert_eq!(manifest.chunk_size_at(0), CHUNK_SIZE as u32);

        // Last chunk is partial
        assert_eq!(manifest.chunk_size_at(1), 100);

        // Out of bounds
        assert_eq!(manifest.chunk_size_at(2), 0);
    }

    #[test]
    fn test_chunk_index_for_offset() {
        let manifest = ChunkManifest::new(CHUNK_SIZE as u64 * 3).expect("new failed");

        assert_eq!(manifest.chunk_index_for_offset(0), 0);
        assert_eq!(manifest.chunk_index_for_offset(CHUNK_SIZE as u64 - 1), 0);
        assert_eq!(manifest.chunk_index_for_offset(CHUNK_SIZE as u64), 1);
        assert_eq!(manifest.chunk_index_for_offset(CHUNK_SIZE as u64 * 2), 2);
        assert_eq!(manifest.chunk_index_for_offset(CHUNK_SIZE as u64 * 10), 2); // clamped
    }

    #[test]
    fn test_offset_within_chunk() {
        let manifest = ChunkManifest::new(CHUNK_SIZE as u64 * 2).expect("new failed");

        assert_eq!(manifest.offset_within_chunk(0), 0);
        assert_eq!(manifest.offset_within_chunk(100), 100);
        assert_eq!(manifest.offset_within_chunk(CHUNK_SIZE as u64), 0);
        assert_eq!(manifest.offset_within_chunk(CHUNK_SIZE as u64 + 50), 50);
    }

    #[test]
    fn test_is_chunk_manifest() {
        let manifest = ChunkManifest {
            total_size: 1000,
            chunk_size: CHUNK_SIZE as u32,
            chunk_count: 1,
        };

        let bytes = manifest.to_bytes().expect("serialize failed");
        assert!(is_chunk_manifest(&bytes));

        // Raw data should not be detected as manifest
        let raw_data = b"hello world";
        assert!(!is_chunk_manifest(raw_data));

        // Empty data
        assert!(!is_chunk_manifest(&[]));

        // Partial magic
        assert!(!is_chunk_manifest(&CHUNK_MANIFEST_MAGIC[..5]));
    }

    #[test]
    fn test_chunk_key() {
        assert_eq!(chunk_key("foo/bar", 0), "foo/bar.chunk.000000");
        assert_eq!(chunk_key("foo/bar", 1), "foo/bar.chunk.000001");
        assert_eq!(chunk_key("foo/bar", 999_999), "foo/bar.chunk.999999");
    }

    #[test]
    fn test_small_file_passthrough() {
        let fs = AspenFs::new_in_memory(1000, 1000);

        let data = b"small file";
        chunked_write(&fs, "test/small", data).expect("write failed");

        // Should be stored as raw data, not chunked
        let stored = fs.kv_read("test/small").expect("read failed").expect("missing");
        assert!(!is_chunk_manifest(&stored));
        assert_eq!(stored, data);

        // Read back through chunking API
        let read_back = chunked_read(&fs, "test/small").expect("read failed").expect("missing");
        assert_eq!(read_back, data);

        // Size check
        let size = chunked_size(&fs, "test/small").expect("size failed").expect("missing");
        assert_eq!(size, data.len() as u64);

        // Delete
        chunked_delete(&fs, "test/small").expect("delete failed");
        assert!(fs.kv_read("test/small").expect("read failed").is_none());
    }

    #[test]
    fn test_large_file_roundtrip() {
        let fs = AspenFs::new_in_memory(1000, 1000);

        // Create a file larger than CHUNK_SIZE
        let size = CHUNK_SIZE + 10_000;
        let mut data = vec![0u8; size];
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = (i % 256) as u8;
        }

        chunked_write(&fs, "test/large", &data).expect("write failed");

        // Should be stored as manifest
        let stored = fs.kv_read("test/large").expect("read failed").expect("missing");
        assert!(is_chunk_manifest(&stored));

        let manifest = ChunkManifest::from_bytes(&stored).expect("parse failed");
        assert_eq!(manifest.total_size, size as u64);
        assert_eq!(manifest.chunk_count, 2);

        // Verify chunks exist
        assert!(fs.kv_read("test/large.chunk.000000").expect("read failed").is_some());
        assert!(fs.kv_read("test/large.chunk.000001").expect("read failed").is_some());

        // Read back and verify
        let read_back = chunked_read(&fs, "test/large").expect("read failed").expect("missing");
        assert_eq!(read_back.len(), data.len());
        assert_eq!(read_back, data);

        // Size check
        let file_size = chunked_size(&fs, "test/large").expect("size failed").expect("missing");
        assert_eq!(file_size, size as u64);

        // Delete and verify cleanup
        chunked_delete(&fs, "test/large").expect("delete failed");
        assert!(fs.kv_read("test/large").expect("read failed").is_none());
        assert!(fs.kv_read("test/large.chunk.000000").expect("read failed").is_none());
        assert!(fs.kv_read("test/large.chunk.000001").expect("read failed").is_none());
    }

    #[test]
    fn test_chunked_read_range() {
        let fs = AspenFs::new_in_memory(1000, 1000);

        // Create test data spanning multiple chunks
        let size = CHUNK_SIZE * 2 + 1000;
        let mut data = vec![0u8; size];
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = (i % 256) as u8;
        }

        chunked_write(&fs, "test/range", &data).expect("write failed");

        // Read from first chunk
        let chunk0 = chunked_read_range(&fs, "test/range", 0, 100).expect("read failed");
        assert_eq!(chunk0, &data[0..100]);

        // Read from second chunk
        let offset = CHUNK_SIZE as u64;
        let chunk1 = chunked_read_range(&fs, "test/range", offset, 100).expect("read failed");
        assert_eq!(chunk1, &data[CHUNK_SIZE..CHUNK_SIZE + 100]);

        // Read across chunk boundary
        let offset = (CHUNK_SIZE - 50) as u64;
        let across = chunked_read_range(&fs, "test/range", offset, 200).expect("read failed");
        assert_eq!(across, &data[CHUNK_SIZE - 50..CHUNK_SIZE + 150]);

        // Read beyond EOF
        let offset = (size - 10) as u64;
        let eof = chunked_read_range(&fs, "test/range", offset, 100).expect("read failed");
        assert_eq!(eof.len(), 10);
        assert_eq!(eof, &data[size - 10..]);

        // Read past EOF
        let offset = size as u64 + 100;
        let past = chunked_read_range(&fs, "test/range", offset, 100).expect("read failed");
        assert_eq!(past.len(), 0);
    }

    #[test]
    fn test_chunked_write_range() {
        let fs = AspenFs::new_in_memory(1000, 1000);

        // Start with a large file
        let size = CHUNK_SIZE * 2;
        let data = vec![0u8; size];
        chunked_write(&fs, "test/write_range", &data).expect("write failed");

        // Overwrite a range in the first chunk
        let new_data = b"MODIFIED";
        chunked_write_range(&fs, "test/write_range", 100, new_data).expect("write failed");

        let read_back = chunked_read(&fs, "test/write_range").expect("read failed").expect("missing");
        assert_eq!(&read_back[100..108], new_data);

        // Overwrite across chunk boundary
        let offset = (CHUNK_SIZE - 4) as u64;
        let cross_data = b"BOUNDARY";
        chunked_write_range(&fs, "test/write_range", offset, cross_data).expect("write failed");

        let read_back = chunked_read(&fs, "test/write_range").expect("read failed").expect("missing");
        assert_eq!(&read_back[CHUNK_SIZE - 4..CHUNK_SIZE + 4], cross_data);

        // Extend file
        let offset = size as u64;
        let extend_data = b"EXTENDED";
        chunked_write_range(&fs, "test/write_range", offset, extend_data).expect("write failed");

        let new_size = chunked_size(&fs, "test/write_range").expect("size failed").expect("missing");
        assert_eq!(new_size, size as u64 + extend_data.len() as u64);

        let read_back = chunked_read(&fs, "test/write_range").expect("read failed").expect("missing");
        assert_eq!(&read_back[size..size + extend_data.len()], extend_data);
    }

    #[test]
    fn test_chunked_rename() {
        let fs = AspenFs::new_in_memory(1000, 1000);

        // Create a chunked file
        let size = CHUNK_SIZE + 1000;
        let mut data = vec![0u8; size];
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = (i % 256) as u8;
        }

        chunked_write(&fs, "test/old", &data).expect("write failed");

        // Rename
        chunked_rename(&fs, RenameKeys {
            old_key: "test/old",
            new_key: "test/new",
        })
        .expect("rename failed");

        // Old should be gone
        assert!(chunked_read(&fs, "test/old").expect("read failed").is_none());
        assert!(fs.kv_read("test/old.chunk.000000").expect("read failed").is_none());

        // New should exist with same content
        let read_back = chunked_read(&fs, "test/new").expect("read failed").expect("missing");
        assert_eq!(read_back, data);

        // Verify chunks were renamed
        assert!(fs.kv_read("test/new.chunk.000000").expect("read failed").is_some());
        assert!(fs.kv_read("test/new.chunk.000001").expect("read failed").is_some());
    }

    #[test]
    fn test_overwrite_large_with_small() {
        let fs = AspenFs::new_in_memory(1000, 1000);

        // Write large file
        let large_data = vec![0xAA; CHUNK_SIZE + 1000];
        chunked_write(&fs, "test/shrink", &large_data).expect("write failed");

        // Verify it's chunked
        assert!(is_chunk_manifest(&fs.kv_read("test/shrink").expect("read failed").expect("missing")));

        // Overwrite with small file
        let small_data = b"small";
        chunked_write(&fs, "test/shrink", small_data).expect("write failed");

        // Should now be stored as raw data
        let stored = fs.kv_read("test/shrink").expect("read failed").expect("missing");
        assert!(!is_chunk_manifest(&stored));
        assert_eq!(stored, small_data);

        // Old chunks should be gone
        assert!(fs.kv_read("test/shrink.chunk.000000").expect("read failed").is_none());
    }

    #[test]
    fn test_write_range_on_nonexistent_file() {
        let fs = AspenFs::new_in_memory(1000, 1000);

        // Write at offset to non-existent file
        let data = b"NEW FILE";
        chunked_write_range(&fs, "test/new", 100, data).expect("write failed");

        // Should create file with zeros up to offset
        let read_back = chunked_read(&fs, "test/new").expect("read failed").expect("missing");
        assert_eq!(read_back.len(), 108);
        assert_eq!(&read_back[0..100], &vec![0u8; 100][..]);
        assert_eq!(&read_back[100..108], data);
    }

    #[test]
    fn test_max_file_size_enforcement() {
        let fs = AspenFs::new_in_memory(1000, 1000);

        // Try to write file exceeding MAX_FILE_SIZE
        let result = ChunkManifest::new(MAX_FILE_SIZE + 1);
        assert!(result.is_err());

        // Try to extend file beyond MAX_FILE_SIZE
        let data = vec![0u8; 1000];
        chunked_write(&fs, "test/limit", &data).expect("write failed");

        let extend = vec![0u8; 1000];
        let result = chunked_write_range(&fs, "test/limit", MAX_FILE_SIZE - 500, &extend);
        assert!(result.is_err());
    }
}
