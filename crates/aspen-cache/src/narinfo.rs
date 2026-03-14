//! Nix binary cache metadata types.
//!
//! This module defines the data structures for storing Nix store path metadata
//! in the Aspen distributed cache. The design follows the Nix narinfo format
//! but stores data as JSON in the Raft KV store for easy querying.

use nix_compat::narinfo;
use nix_compat::nixbase32;
use nix_compat::store_path::StorePathRef;
use serde::Deserialize;
use serde::Serialize;

use crate::error::CacheError;
use crate::error::Result;

// Tiger Style: All limits explicit and bounded
/// Maximum number of references per cache entry.
pub const MAX_REFERENCES: usize = 1000;
/// Maximum length of deriver path.
pub const MAX_DERIVER_LENGTH: usize = 1024;
/// Maximum length of store path.
pub const MAX_STORE_PATH_LENGTH: usize = 512;
/// Store hash length (Nix uses 32 bytes = 52 base32 chars, but we also support base16).
pub const STORE_HASH_LENGTH: usize = 32;

/// KV key prefix for cache entries.
pub const CACHE_KEY_PREFIX: &str = "_cache:narinfo:";
/// KV key for cache statistics.
pub const CACHE_STATS_KEY: &str = "_cache:stats";

/// A cache entry representing a Nix store path in the distributed cache.
///
/// This contains all metadata needed to retrieve and verify a NAR archive
/// from the blob store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheEntry {
    /// Full Nix store path (e.g., /nix/store/abc...-name).
    pub store_path: String,

    /// The hash portion of the store path (e.g., abc...).
    /// This is used as the lookup key.
    pub store_hash: String,

    /// BLAKE3 hash of the NAR archive in the blob store.
    /// This is used to retrieve the blob via iroh-blobs.
    pub blob_hash: String,

    /// Size of the NAR archive in bytes.
    pub nar_size: u64,

    /// SHA256 hash of the NAR archive (Nix's native format).
    /// Format: "sha256:base16_or_base32_hash"
    pub nar_hash: String,

    /// Original file size (may differ from NAR size due to NAR overhead).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,

    /// Store paths this entry references (dependencies).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,

    /// Deriver store path (the .drv that built this).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deriver: Option<String>,

    /// Unix timestamp when this entry was created.
    pub created_at: u64,

    /// Node ID that built and uploaded this entry.
    pub created_by_node: u64,

    /// CI job ID that created this entry (if from CI build).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ci_job_id: Option<String>,

    /// CI pipeline run ID (if from CI build).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ci_run_id: Option<String>,
}

impl CacheEntry {
    /// Create a new cache entry with required fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        store_path: String,
        store_hash: String,
        blob_hash: String,
        nar_size: u64,
        nar_hash: String,
        created_at: u64,
        created_by_node: u64,
    ) -> Self {
        Self {
            store_path,
            store_hash,
            blob_hash,
            nar_size,
            nar_hash,
            file_size: None,
            references: Vec::new(),
            deriver: None,
            created_at,
            created_by_node,
            ci_job_id: None,
            ci_run_id: None,
        }
    }

    /// Set the references for this entry.
    pub fn with_references(mut self, references: Vec<String>) -> Result<Self> {
        if references.len() > MAX_REFERENCES {
            return Err(CacheError::TooManyReferences {
                count: references.len() as u32,
                max: MAX_REFERENCES as u32,
            });
        }
        self.references = references;
        Ok(self)
    }

    /// Set the deriver for this entry.
    pub fn with_deriver(mut self, deriver: Option<String>) -> Result<Self> {
        if let Some(ref d) = deriver
            && d.len() > MAX_DERIVER_LENGTH
        {
            return Err(CacheError::DeriverTooLong {
                length_bytes: d.len() as u64,
                max_bytes: MAX_DERIVER_LENGTH as u64,
            });
        }
        self.deriver = deriver;
        Ok(self)
    }

    /// Set CI metadata for this entry.
    pub fn with_ci_metadata(mut self, job_id: Option<String>, run_id: Option<String>) -> Self {
        self.ci_job_id = job_id;
        self.ci_run_id = run_id;
        self
    }

    /// Generate the KV key for this entry.
    pub fn kv_key(&self) -> String {
        format!("{}{}", CACHE_KEY_PREFIX, self.store_hash)
    }

    /// Serialize this entry to JSON bytes for KV storage.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| CacheError::Serialization { message: e.to_string() })
    }

    /// Deserialize an entry from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(|e| CacheError::Deserialization { message: e.to_string() })
    }

    /// Compute the Nix narinfo fingerprint used for signing.
    ///
    /// Uses `nix_compat::narinfo::fingerprint()` to produce the canonical format:
    /// `1;{store_path};sha256:{nix32_nar_hash};{nar_size};{comma_separated_absolute_ref_paths}`
    pub fn fingerprint(&self) -> String {
        // Parse our store path
        let store_path = StorePathRef::from_bytes(
            self.store_path.strip_prefix("/nix/store/").unwrap_or(&self.store_path).as_bytes(),
        );

        // Decode nar_hash from "sha256:<nix32>" to [u8; 32]
        let nar_hash_bytes =
            self.nar_hash.strip_prefix("sha256:").and_then(|h| nixbase32::decode_fixed::<32>(h.as_bytes()).ok());

        // Parse references as store paths
        let ref_paths: Vec<StorePathRef<'_>> = self
            .references
            .iter()
            .filter_map(|r| {
                let basename = r.strip_prefix("/nix/store/").unwrap_or(r);
                StorePathRef::from_bytes(basename.as_bytes()).ok()
            })
            .collect();

        // If we can use nix-compat's fingerprint function, do so
        if let (Ok(sp), Some(hash)) = (store_path, nar_hash_bytes) {
            narinfo::fingerprint(&sp, &hash, self.nar_size, ref_paths.iter())
        } else {
            // Fallback for entries with non-standard hashes (e.g., hex-encoded)
            let mut sorted_refs = self.references.clone();
            sorted_refs.sort();
            let refs_str = sorted_refs.join(",");
            format!("1;{};{};{};{}", self.store_path, self.nar_hash, self.nar_size, refs_str)
        }
    }

    /// Render this cache entry as a Nix narinfo text document.
    ///
    /// The narinfo format is the standard Nix binary cache metadata format.
    /// If a signature is provided, it's included as the `Sig:` field.
    ///
    /// URL points to `/nar/{blob_hash}.nar` for the gateway to serve.
    pub fn to_narinfo(&self, signature: Option<&str>) -> String {
        let mut lines = Vec::with_capacity(10);

        lines.push(format!("StorePath: {}", self.store_path));
        lines.push(format!("URL: nar/{}.nar", self.blob_hash));
        lines.push("Compression: none".to_string());

        // Use nix-compat nixbase32 for NarHash if possible
        lines.push(format!("NarHash: {}", self.nar_hash));
        lines.push(format!("NarSize: {}", self.nar_size));

        // References: basenames (store path without /nix/store/ prefix)
        let ref_basenames: Vec<&str> = self.references.iter().filter_map(|r| r.strip_prefix("/nix/store/")).collect();
        lines.push(format!("References: {}", ref_basenames.join(" ")));

        if let Some(ref deriver) = self.deriver {
            if let Some(basename) = deriver.strip_prefix("/nix/store/") {
                lines.push(format!("Deriver: {basename}"));
            } else {
                lines.push(format!("Deriver: {deriver}"));
            }
        }

        if let Some(file_size) = self.file_size {
            lines.push(format!("FileSize: {file_size}"));
        } else {
            // FileSize equals NarSize when Compression is none
            lines.push(format!("FileSize: {}", self.nar_size));
        }

        if let Some(sig) = signature {
            lines.push(format!("Sig: {sig}"));
        }

        let mut result = lines.join("\n");
        result.push('\n');
        result
    }
}

/// Cache statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of cache entries.
    pub total_entries: u64,

    /// Total size of all NAR archives in bytes.
    pub total_nar_bytes: u64,

    /// Number of cache queries.
    pub query_count: u64,

    /// Number of cache hits.
    pub hit_count: u64,

    /// Number of cache misses.
    pub miss_count: u64,

    /// Unix timestamp of last update.
    pub last_updated: u64,
}

impl CacheStats {
    /// Create new empty stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a cache hit.
    pub fn record_hit(&mut self) {
        self.query_count += 1;
        self.hit_count += 1;
    }

    /// Record a cache miss.
    pub fn record_miss(&mut self) {
        self.query_count += 1;
        self.miss_count += 1;
    }

    /// Record a new entry being added.
    pub fn record_entry_added(&mut self, nar_size: u64) {
        self.total_entries += 1;
        self.total_nar_bytes += nar_size;
    }

    /// Calculate hit rate as a percentage.
    pub fn hit_rate_percent(&self) -> f64 {
        if self.query_count == 0 {
            0.0
        } else {
            (self.hit_count as f64 / self.query_count as f64) * 100.0
        }
    }

    /// Serialize to JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| CacheError::Serialization { message: e.to_string() })
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(|e| CacheError::Deserialization { message: e.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_roundtrip() {
        let entry = CacheEntry::new(
            "/nix/store/abc123-hello".to_string(),
            "abc123".to_string(),
            "blake3hash".to_string(),
            1024,
            "sha256:deadbeef".to_string(),
            1234567890,
            1,
        );

        let bytes = entry.to_bytes().unwrap();
        let decoded = CacheEntry::from_bytes(&bytes).unwrap();
        assert_eq!(entry, decoded);
    }

    #[test]
    fn test_cache_entry_kv_key() {
        let entry = CacheEntry::new(
            "/nix/store/abc123-hello".to_string(),
            "abc123".to_string(),
            "blake3hash".to_string(),
            1024,
            "sha256:deadbeef".to_string(),
            1234567890,
            1,
        );

        assert_eq!(entry.kv_key(), "_cache:narinfo:abc123");
    }

    #[test]
    fn test_cache_entry_too_many_references() {
        let entry = CacheEntry::new(
            "/nix/store/abc123-hello".to_string(),
            "abc123".to_string(),
            "blake3hash".to_string(),
            1024,
            "sha256:deadbeef".to_string(),
            1234567890,
            1,
        );

        let too_many: Vec<String> = (0..MAX_REFERENCES + 1).map(|i| format!("ref{i}")).collect();
        let result = entry.with_references(too_many);
        assert!(result.is_err());
    }

    #[test]
    fn test_narinfo_complete_entry() {
        let entry = CacheEntry::new(
            "/nix/store/abc123-hello-2.10".to_string(),
            "abc123".to_string(),
            "b3hashvalue".to_string(),
            2048,
            "sha256:deadbeefcafe".to_string(),
            1234567890,
            1,
        )
        .with_references(vec![
            "/nix/store/xyz789-glibc-2.38".to_string(),
            "/nix/store/def456-gcc-13".to_string(),
        ])
        .unwrap()
        .with_deriver(Some("/nix/store/drv111-hello-2.10.drv".to_string()))
        .unwrap();

        let narinfo = entry.to_narinfo(Some("aspen-cache:c2lnbmF0dXJl"));

        assert!(narinfo.contains("StorePath: /nix/store/abc123-hello-2.10"));
        assert!(narinfo.contains("URL: nar/b3hashvalue.nar"));
        assert!(narinfo.contains("Compression: none"));
        assert!(narinfo.contains("NarHash: sha256:deadbeefcafe"));
        assert!(narinfo.contains("NarSize: 2048"));
        // References are basenames
        assert!(narinfo.contains("References: xyz789-glibc-2.38 def456-gcc-13"));
        assert!(narinfo.contains("Deriver: drv111-hello-2.10.drv"));
        assert!(narinfo.contains("FileSize: 2048"));
        assert!(narinfo.contains("Sig: aspen-cache:c2lnbmF0dXJl"));
        assert!(narinfo.ends_with('\n'));
    }

    #[test]
    fn test_narinfo_minimal_entry() {
        let entry = CacheEntry::new(
            "/nix/store/abc123-hello".to_string(),
            "abc123".to_string(),
            "b3hash".to_string(),
            512,
            "sha256:aabbccdd".to_string(),
            1000,
            1,
        );

        let narinfo = entry.to_narinfo(None);

        assert!(narinfo.contains("StorePath: /nix/store/abc123-hello"));
        assert!(narinfo.contains("References: "));
        assert!(!narinfo.contains("Deriver:"));
        assert!(!narinfo.contains("Sig:"));
    }

    #[test]
    fn test_narinfo_with_signature() {
        let entry = CacheEntry::new(
            "/nix/store/abc123-hello".to_string(),
            "abc123".to_string(),
            "b3hash".to_string(),
            1024,
            "sha256:aabb".to_string(),
            1000,
            1,
        );

        let without_sig = entry.to_narinfo(None);
        assert!(!without_sig.contains("Sig:"));

        let with_sig = entry.to_narinfo(Some("my-cache:base64sig"));
        assert!(with_sig.contains("Sig: my-cache:base64sig"));
    }

    #[test]
    fn test_fingerprint_basic() {
        let entry = CacheEntry::new(
            "/nix/store/abc123-hello".to_string(),
            "abc123".to_string(),
            "b3hash".to_string(),
            1024,
            "sha256:deadbeef".to_string(),
            1000,
            1,
        );

        assert_eq!(entry.fingerprint(), "1;/nix/store/abc123-hello;sha256:deadbeef;1024;");
    }

    #[test]
    fn test_fingerprint_with_sorted_refs() {
        let entry = CacheEntry::new(
            "/nix/store/abc123-hello".to_string(),
            "abc123".to_string(),
            "b3hash".to_string(),
            2048,
            "sha256:cafe".to_string(),
            1000,
            1,
        )
        .with_references(vec![
            "/nix/store/zzz-last".to_string(),
            "/nix/store/aaa-first".to_string(),
            "/nix/store/mmm-middle".to_string(),
        ])
        .unwrap();

        let fp = entry.fingerprint();
        // References should be sorted
        assert_eq!(
            fp,
            "1;/nix/store/abc123-hello;sha256:cafe;2048;/nix/store/aaa-first,/nix/store/mmm-middle,/nix/store/zzz-last"
        );
    }

    #[test]
    fn test_narinfo_roundtrip_parseable() {
        // Create a realistic cache entry
        let entry = CacheEntry::new(
            "/nix/store/w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb-hello-2.12.1".to_string(),
            "w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb".to_string(),
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".to_string(),
            226552,
            "sha256:1b0ri7q2g18vi4ql05z4ni5g098drmi2w3sswgjyqqwdrbig0w08".to_string(),
            1709000000,
            1,
        )
        .with_references(vec!["/nix/store/7gx4kiv5m0i7d7qkixq2cwzbr10lvxwc-glibc-2.38-27".to_string()])
        .unwrap()
        .with_deriver(Some("/nix/store/dh3yb2m5p1s8dxaqzarq2kq8f1y7sz2m-hello-2.12.1.drv".to_string()))
        .unwrap();

        let narinfo = entry.to_narinfo(Some("aspen-cache:fakebase64sig"));

        // Verify all required fields present
        assert!(narinfo.contains("StorePath:"));
        assert!(narinfo.contains("URL: nar/"));
        assert!(narinfo.contains("Compression: none"));
        assert!(narinfo.contains("NarHash: sha256:"));
        assert!(narinfo.contains("NarSize: 226552"));
        assert!(narinfo.contains("References:"));
        assert!(narinfo.contains("Deriver:"));
        assert!(narinfo.contains("FileSize:"));
        assert!(narinfo.contains("Sig:"));

        // Verify Nix-parseable format: each line is "Key: Value"
        for line in narinfo.trim().lines() {
            assert!(line.contains(": "), "narinfo line should be 'Key: Value', got: {line}");
        }
    }

    #[test]
    fn test_narinfo_nix_compat_parseable() {
        // Create a realistic entry with valid nix32 nar_hash and store path
        let entry = CacheEntry::new(
            "/nix/store/w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb-hello-2.12.1".to_string(),
            "w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb".to_string(),
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".to_string(),
            226552,
            "sha256:1b0ri7q2g18vi4ql05z4ni5g098drmi2w3sswgjyqqwdrbig0w08".to_string(),
            1709000000,
            1,
        )
        .with_references(vec!["/nix/store/7gx4kiv5m0i7d7qkixq2cwzbr10lvxwc-glibc-2.38-27".to_string()])
        .unwrap()
        .with_deriver(Some("/nix/store/dh3yb2m5p1s8dxaqzarq2kq8f1y7sz2m-hello-2.12.1.drv".to_string()))
        .unwrap();

        // Generate a real signature
        let signing_key = crate::signing::CacheSigningKey::generate("test-cache").unwrap();
        let fingerprint = entry.fingerprint();
        let signature = signing_key.sign_fingerprint(&fingerprint);

        let narinfo_str = entry.to_narinfo(Some(&signature));

        // Parse with nix-compat to verify it's valid
        let parsed =
            nix_compat::narinfo::NarInfo::parse(&narinfo_str).expect("narinfo should be parseable by nix-compat");

        assert_eq!(parsed.store_path.to_string(), "w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb-hello-2.12.1");
        assert_eq!(parsed.nar_size, 226552);
        assert_eq!(parsed.references.len(), 1);
        assert!(!parsed.signatures.is_empty());
    }

    #[test]
    fn test_cache_stats() {
        let mut stats = CacheStats::new();
        assert_eq!(stats.hit_rate_percent(), 0.0);

        stats.record_hit();
        stats.record_hit();
        stats.record_miss();

        assert_eq!(stats.query_count, 3);
        assert_eq!(stats.hit_count, 2);
        assert_eq!(stats.miss_count, 1);
        assert!((stats.hit_rate_percent() - 66.666).abs() < 0.01);
    }
}
