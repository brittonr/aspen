//! Core types for the execution cache.

use serde::Deserialize;
use serde::Serialize;

/// A cache key: 32-byte BLAKE3 hash of command + args + env + sorted input hashes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey(pub [u8; 32]);

impl CacheKey {
    /// Encode as lowercase hex string.
    pub fn to_hex(&self) -> String {
        let mut buf = [0u8; 64];
        for (hex_bytes, byte) in buf.chunks_exact_mut(2).zip(self.0.iter().copied()) {
            let hi = byte >> 4;
            let lo = byte & 0x0f;
            hex_bytes[0] = encode_hex_digit(hi);
            hex_bytes[1] = encode_hex_digit(lo);
        }
        // SAFETY: hex encoding only produces ASCII bytes
        unsafe { String::from_utf8_unchecked(buf.to_vec()) }
    }

    /// Decode from a 64-character hex string.
    ///
    /// Returns `None` if the string is not exactly 64 hex characters.
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 64 {
            return None;
        }
        let mut bytes = [0u8; 32];
        for (byte, hex_pair) in bytes.iter_mut().zip(hex.as_bytes().chunks_exact(2)) {
            let hi = hex_digit(hex_pair[0])?;
            let lo = hex_digit(hex_pair[1])?;
            *byte = (hi << 4) | lo;
        }
        Some(CacheKey(bytes))
    }
}

impl std::fmt::Display for CacheKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// Encode a 4-bit value as a lowercase hex digit.
fn encode_hex_digit(nibble: u8) -> u8 {
    match nibble {
        0..=9 => b'0'.saturating_add(nibble),
        _ => b'a'.saturating_add(nibble.saturating_sub(10)),
    }
}

/// Decode a single hex digit to its 4-bit value.
fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => b.checked_sub(b'0'),
        b'a'..=b'f' => b.checked_sub(b'a').map(|value| value.saturating_add(10)),
        b'A'..=b'F' => b.checked_sub(b'A').map(|value| value.saturating_add(10)),
        _ => None,
    }
}

/// Mapping from an output file path to its blob hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputMapping {
    /// Relative path of the output file.
    pub path: String,
    /// BLAKE3 hash of the file contents stored as an iroh-blob.
    pub hash: [u8; 32],
    /// Size of the output file in bytes.
    pub size_bytes: u64,
}

/// A cached execution result stored in the Raft KV.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Process exit code.
    pub exit_code: i32,
    /// BLAKE3 hash of captured stdout, stored as an iroh-blob.
    pub stdout_hash: [u8; 32],
    /// BLAKE3 hash of captured stderr, stored as an iroh-blob.
    pub stderr_hash: [u8; 32],
    /// Output files produced by the process.
    pub outputs: Vec<OutputMapping>,
    /// Timestamp when this entry was created (milliseconds since epoch).
    pub created_at_ms: u64,
    /// Time-to-live for this entry (milliseconds).
    pub ttl_ms: u64,
    /// Timestamp of last access (milliseconds since epoch), for LRU eviction.
    pub last_accessed_ms: u64,
    /// Cache keys of child processes (for recursive process tree caching).
    pub child_keys: Vec<CacheKey>,
}

impl CacheEntry {
    /// Check if this entry has expired given the current time.
    pub fn is_expired(&self, now_ms: u64) -> bool {
        now_ms > self.created_at_ms.saturating_add(self.ttl_ms)
    }

    /// Serialize to JSON bytes for KV storage.
    pub fn to_json(&self) -> crate::Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|source| crate::ExecCacheError::Serialize { source })
    }

    /// Deserialize from JSON bytes.
    pub fn from_json(bytes: &[u8]) -> crate::Result<Self> {
        serde_json::from_slice(bytes).map_err(|source| crate::ExecCacheError::Deserialize { source })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_hex_roundtrip() {
        let key = CacheKey([
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23,
            0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
        ]);
        let hex = key.to_hex();
        assert_eq!(hex, "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let decoded = CacheKey::from_hex(&hex).expect("valid hex");
        assert_eq!(key, decoded);
    }

    #[test]
    fn cache_key_from_hex_rejects_short() {
        assert!(CacheKey::from_hex("0123").is_none());
    }

    #[test]
    fn cache_key_from_hex_rejects_invalid() {
        let bad = "zz23456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert!(CacheKey::from_hex(bad).is_none());
    }

    #[test]
    fn cache_key_from_hex_accepts_uppercase() {
        let upper = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";
        let key = CacheKey::from_hex(upper).expect("uppercase hex valid");
        assert_eq!(key.0[1], 0x23);
    }

    #[test]
    fn cache_entry_expiration() {
        let entry = CacheEntry {
            exit_code: 0,
            stdout_hash: [0; 32],
            stderr_hash: [0; 32],
            outputs: vec![],
            created_at_ms: 1000,
            ttl_ms: 500,
            last_accessed_ms: 1000,
            child_keys: vec![],
        };
        // Within TTL
        assert!(!entry.is_expired(1200));
        // At exact boundary
        assert!(!entry.is_expired(1500));
        // Past TTL
        assert!(entry.is_expired(1501));
    }

    #[test]
    fn cache_entry_json_roundtrip() {
        let entry = CacheEntry {
            exit_code: 1,
            stdout_hash: [0xaa; 32],
            stderr_hash: [0xbb; 32],
            outputs: vec![OutputMapping {
                path: "target/main".to_string(),
                hash: [0xcc; 32],
                size_bytes: 4096,
            }],
            created_at_ms: 1710000000000,
            ttl_ms: 86400000,
            last_accessed_ms: 1710000000000,
            child_keys: vec![CacheKey([0xdd; 32])],
        };
        let json = entry.to_json().expect("serialize");
        let decoded = CacheEntry::from_json(&json).expect("deserialize");
        assert_eq!(entry, decoded);
    }

    #[test]
    fn cache_entry_saturating_expiration() {
        // created_at near u64::MAX — saturating_add prevents overflow
        let entry = CacheEntry {
            exit_code: 0,
            stdout_hash: [0; 32],
            stderr_hash: [0; 32],
            outputs: vec![],
            created_at_ms: u64::MAX - 10,
            ttl_ms: 100,
            last_accessed_ms: u64::MAX - 10,
            child_keys: vec![],
        };
        // saturating_add clamps to u64::MAX, so no time can exceed it
        assert!(!entry.is_expired(u64::MAX));
    }
}
