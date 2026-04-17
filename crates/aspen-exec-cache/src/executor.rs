//! Cached process executor.
//!
//! Wraps process execution with cache-check-before / capture-after logic.
//! On cache hit, replays stored outputs without launching the process.
//! On cache miss, runs the process with read tracking, then stores the result.

use std::collections::HashMap;
use std::time::Instant;

use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::constants::CACHE_LOOKUP_TIMEOUT_MS;
use crate::constants::DEFAULT_ENV_VARS;
use crate::constants::DEFAULT_TTL_MS;
use crate::constants::EXTRA_ENV_VAR;
use crate::constants::MAX_OUTPUT_BLOB_SIZE;
use crate::constants::MAX_OUTPUT_FILES;
use crate::error::ExecCacheError;
use crate::error::Result;
use crate::index::CacheKvStore;
use crate::index::ExecCacheIndex;
use crate::read_tracker::ReadSet;
use crate::read_tracker::ReadTracker;
use crate::types::CacheEntry;
use crate::types::CacheKey;
use crate::types::OutputMapping;
use crate::verified::cache_key::compute_cache_key;
use crate::verified::cache_key::compute_env_hash;

/// Result of a cached execution attempt.
#[derive(Debug, Clone)]
pub struct CachedExecResult {
    /// Process exit code.
    pub exit_code: i32,
    /// Stdout content.
    pub stdout: Vec<u8>,
    /// Stderr content.
    pub stderr: Vec<u8>,
    /// Output file mappings (path → blob hash).
    pub outputs: Vec<OutputMapping>,
    /// Whether this was a cache hit.
    pub is_cache_hit: bool,
    /// Time spent on cache lookup (microseconds).
    pub lookup_time_us: u64,
}

/// Statistics from a cached executor session.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: u32,
    /// Number of cache misses.
    pub misses: u32,
    /// Number of cache lookup timeouts.
    pub timeouts: u32,
    /// Total time saved by cache hits (estimated, in milliseconds).
    pub time_saved_ms: u64,
}

/// Trait for blob storage operations needed by the executor.
///
/// Abstracts over iroh-blobs for testability.
pub trait BlobStore: Send + Sync {
    /// Store bytes as a blob, returning its BLAKE3 hash.
    fn store(&self, data: &[u8]) -> std::result::Result<[u8; 32], String>;

    /// Fetch blob content by hash.
    fn fetch(&self, hash: &[u8; 32]) -> std::result::Result<Vec<u8>, String>;
}

/// Parameters for storing an execution result in the cache.
pub struct StoreResultParams<'a> {
    /// The command that was executed.
    pub command: &'a str,
    /// Command-line arguments.
    pub args: &'a [String],
    /// Environment variables during execution.
    pub env: &'a HashMap<String, String>,
    /// Set of files read during execution (from FUSE tracking).
    pub read_set: &'a ReadSet,
    /// Process exit code.
    pub exit_code: i32,
    /// Captured stdout.
    pub stdout: &'a [u8],
    /// Captured stderr.
    pub stderr: &'a [u8],
    /// Output files as (path, content) pairs.
    pub output_files: &'a [(&'a str, &'a [u8])],
    /// Current timestamp in milliseconds.
    pub now_ms: u64,
}

/// Cached process executor.
///
/// Wraps process launches with transparent caching:
/// 1. Compute cache key from command + args + env + input file hashes
/// 2. Check cache index for a hit
/// 3. On hit: replay stored outputs
/// 4. On miss: delegate to inner executor, capture outputs, store in cache
pub struct CachedExecutor<S, B> {
    /// Cache index backed by KV store.
    index: ExecCacheIndex<S>,
    /// Blob store for output content.
    blobs: B,
    /// Read tracker for FUSE-level input tracking.
    tracker: ReadTracker,
    /// Accumulated statistics.
    stats: std::sync::Mutex<CacheStats>,
    /// Cache entry TTL in milliseconds.
    ttl_ms: u64,
    /// Cache lookup timeout in milliseconds.
    lookup_timeout_ms: u64,
}

#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "cache lookup latency measurement owns its monotonic timing boundary"
)]
fn cache_lookup_started_at() -> Instant {
    Instant::now()
}

fn elapsed_lookup_time_us(started_at: Instant) -> u64 {
    match u64::try_from(started_at.elapsed().as_micros()) {
        Ok(lookup_time_us) => lookup_time_us,
        Err(_) => u64::MAX,
    }
}

fn output_file_limit() -> usize {
    match usize::try_from(MAX_OUTPUT_FILES) {
        Ok(output_file_limit) => output_file_limit,
        Err(_) => usize::MAX,
    }
}

impl<S: CacheKvStore, B: BlobStore> CachedExecutor<S, B> {
    /// Create a new cached executor.
    pub fn new(index: ExecCacheIndex<S>, blobs: B, tracker: ReadTracker) -> Self {
        Self {
            index,
            blobs,
            tracker,
            stats: std::sync::Mutex::new(CacheStats::default()),
            ttl_ms: DEFAULT_TTL_MS,
            lookup_timeout_ms: CACHE_LOOKUP_TIMEOUT_MS,
        }
    }

    /// Set the TTL for new cache entries.
    pub fn with_ttl_ms(mut self, ttl_ms: u64) -> Self {
        self.ttl_ms = ttl_ms;
        self
    }

    /// Set the cache lookup timeout.
    pub fn with_lookup_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.lookup_timeout_ms = timeout_ms;
        self
    }

    /// Get a reference to the read tracker.
    pub fn tracker(&self) -> &ReadTracker {
        &self.tracker
    }

    /// Get accumulated cache statistics.
    pub fn stats(&self) -> CacheStats {
        self.stats.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Try to get a cached result for a command execution.
    ///
    /// Call this AFTER the process's input files have been read and tracked.
    /// If the cache has a hit, returns the cached result. Otherwise returns None
    /// and the caller should run the process normally.
    pub fn try_cache_lookup(
        &self,
        pid: u32,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
        now_ms: u64,
    ) -> Option<CachedExecResult> {
        let start = cache_lookup_started_at();

        // Get the read set for this PID
        // Note: we peek without consuming — finalize happens later
        let read_set = self.tracker.finalize(pid)?;
        let key = self.compute_key(command, args, env, &read_set);

        let lookup_time_us = elapsed_lookup_time_us(start);

        // Check timeout
        if lookup_time_us > self.lookup_timeout_ms.saturating_mul(1000) {
            let mut stats = self.stats.lock().ok()?;
            stats.timeouts = stats.timeouts.saturating_add(1);
            warn!(
                cache_key = %key,
                lookup_time_us,
                "cache lookup exceeded timeout, falling through"
            );
            return None;
        }

        // Look up in the index
        let entry = match self.index.get(&key, now_ms) {
            Ok(Some(entry)) => entry,
            Ok(None) => {
                let mut stats = self.stats.lock().ok()?;
                stats.misses = stats.misses.saturating_add(1);
                debug!(cache_key = %key, "cache miss");
                return None;
            }
            Err(e) => {
                warn!(cache_key = %key, error = %e, "cache lookup failed");
                let mut stats = self.stats.lock().ok()?;
                stats.misses = stats.misses.saturating_add(1);
                return None;
            }
        };

        // Verify child keys are still valid (recursive cache check)
        for child_key in &entry.child_keys {
            match self.index.get(child_key, now_ms) {
                Ok(Some(_)) => {} // child still valid
                _ => {
                    debug!(
                        cache_key = %key,
                        child_key = %child_key,
                        "child cache key invalid, treating as miss"
                    );
                    let mut stats = self.stats.lock().ok()?;
                    stats.misses = stats.misses.saturating_add(1);
                    return None;
                }
            }
        }

        // Cache hit — fetch stdout and stderr from blobs
        let stdout = self.blobs.fetch(&entry.stdout_hash).ok()?;
        let stderr = self.blobs.fetch(&entry.stderr_hash).ok()?;

        let mut stats = self.stats.lock().ok()?;
        stats.hits = stats.hits.saturating_add(1);

        info!(
            cache_key = %key,
            exit_code = entry.exit_code,
            output_count = entry.outputs.len(),
            lookup_time_us,
            "cache hit — skipping execution"
        );

        Some(CachedExecResult {
            exit_code: entry.exit_code,
            stdout,
            stderr,
            outputs: entry.outputs.clone(),
            is_cache_hit: true,
            lookup_time_us,
        })
    }

    /// Store execution results in the cache after a process completes.
    ///
    /// Call this after a cache miss when the process has finished executing.
    /// Captures stdout, stderr, exit code, and output files into the cache.
    pub fn store_result(&self, params: &StoreResultParams<'_>) -> Result<CacheKey> {
        let StoreResultParams {
            command,
            args,
            env,
            read_set,
            exit_code,
            stdout,
            stderr,
            output_files,
            now_ms,
        } = params;
        // Check output file count
        if output_files.len() > output_file_limit() {
            return Err(ExecCacheError::TooManyOutputFiles {
                count: output_files.len() as u32,
                max: MAX_OUTPUT_FILES,
            });
        }

        // Check individual output sizes
        for (path, data) in output_files.iter() {
            if data.len() as u64 > MAX_OUTPUT_BLOB_SIZE {
                warn!(path, size = data.len(), "output too large, skipping cache storage");
                return Err(ExecCacheError::OutputTooLarge {
                    size_bytes: data.len() as u64,
                    max_bytes: MAX_OUTPUT_BLOB_SIZE,
                });
            }
        }

        let key = self.compute_key(command, args, env, read_set);

        // Store stdout and stderr as blobs
        let stdout_hash = self.blobs.store(stdout).map_err(|message| ExecCacheError::BlobStore { message })?;
        let stderr_hash = self.blobs.store(stderr).map_err(|message| ExecCacheError::BlobStore { message })?;

        // Store output files as blobs
        let mut outputs = Vec::with_capacity(output_files.len());
        for (path, data) in output_files.iter() {
            let hash = self.blobs.store(data).map_err(|message| ExecCacheError::BlobStore { message })?;
            outputs.push(OutputMapping {
                path: (*path).to_string(),
                hash,
                size_bytes: data.len() as u64,
            });
        }

        let entry = CacheEntry {
            exit_code: *exit_code,
            stdout_hash,
            stderr_hash,
            outputs,
            created_at_ms: *now_ms,
            ttl_ms: self.ttl_ms,
            last_accessed_ms: *now_ms,
            child_keys: read_set.child_keys.clone(),
        };

        self.index.put(&key, &entry)?;

        info!(
            cache_key = %key,
            exit_code,
            output_count = output_files.len(),
            "cached execution result"
        );

        Ok(key)
    }

    /// Materialize cached output files by fetching from blob store.
    ///
    /// Returns the output file contents for the caller to write to disk.
    pub fn materialize_outputs(&self, outputs: &[OutputMapping]) -> Result<Vec<(String, Vec<u8>)>> {
        let mut result = Vec::with_capacity(outputs.len());
        for mapping in outputs {
            let data = self.blobs.fetch(&mapping.hash).map_err(|message| ExecCacheError::BlobStore { message })?;
            result.push((mapping.path.clone(), data));
        }
        Ok(result)
    }

    /// Compute the cache key for a command execution.
    fn compute_key(
        &self,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
        read_set: &ReadSet,
    ) -> CacheKey {
        // Build curated env var list
        let mut env_vars: Vec<(&str, Option<&str>)> =
            DEFAULT_ENV_VARS.iter().map(|name| (*name, env.get(*name).map(String::as_str))).collect();

        // Add extra vars from ASPEN_EXEC_CACHE_ENV
        if let Some(extra) = env.get(EXTRA_ENV_VAR) {
            for var_name in extra.split(',') {
                let var_name = var_name.trim();
                if !var_name.is_empty() {
                    env_vars.push((var_name, env.get(var_name).map(String::as_str)));
                }
            }
        }

        let env_hash = compute_env_hash(&env_vars);

        let arg_bytes: Vec<&[u8]> = args.iter().map(|a| a.as_bytes()).collect();
        let mut input_hashes = read_set.input_hashes();

        compute_cache_key(command.as_bytes(), &arg_bytes, &env_hash, &mut input_hashes)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::RwLock;

    use super::*;
    use crate::index::test_store::InMemoryKvStore;

    /// In-memory blob store for testing.
    struct InMemoryBlobStore {
        data: RwLock<BTreeMap<[u8; 32], Vec<u8>>>,
    }

    impl InMemoryBlobStore {
        fn new() -> Self {
            Self {
                data: RwLock::new(BTreeMap::new()),
            }
        }
    }

    impl BlobStore for InMemoryBlobStore {
        fn store(&self, data: &[u8]) -> std::result::Result<[u8; 32], String> {
            let hash = *blake3::hash(data).as_bytes();
            self.data.write().map_err(|e| e.to_string())?.insert(hash, data.to_vec());
            Ok(hash)
        }

        fn fetch(&self, hash: &[u8; 32]) -> std::result::Result<Vec<u8>, String> {
            self.data
                .read()
                .map_err(|e| e.to_string())?
                .get(hash)
                .cloned()
                .ok_or_else(|| format!("blob not found: {:?}", &hash[..4]))
        }
    }

    fn make_executor() -> CachedExecutor<InMemoryKvStore, InMemoryBlobStore> {
        let index = ExecCacheIndex::new(InMemoryKvStore::new());
        let blobs = InMemoryBlobStore::new();
        let tracker = ReadTracker::new();
        CachedExecutor::new(index, blobs, tracker)
    }

    fn sample_env() -> HashMap<String, String> {
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
        env
    }

    #[test]
    fn store_and_lookup_roundtrip() {
        let exec = make_executor();
        exec.tracker().set_enabled(true);

        let now = 1_000_000u64;
        let command = "rustc";
        let args = vec!["src/main.rs".to_string(), "-o".to_string(), "target/main".to_string()];
        let env = sample_env();

        // Simulate a process that reads files
        exec.tracker().start_session(100);
        exec.tracker().record_read(100, "src/main.rs".into(), [0x11; 32]);
        exec.tracker().record_read(100, "src/lib.rs".into(), [0x22; 32]);
        let read_set = exec.tracker().finalize(100).unwrap();

        // Store result
        let _key = exec
            .store_result(&StoreResultParams {
                command,
                args: &args,
                env: &env,
                read_set: &read_set,
                exit_code: 0,
                stdout: b"compiled OK",
                stderr: b"",
                output_files: &[("target/main", b"binary content")],
                now_ms: now,
            })
            .unwrap();

        // Now simulate the same process again
        exec.tracker().start_session(200);
        exec.tracker().record_read(200, "src/main.rs".into(), [0x11; 32]);
        exec.tracker().record_read(200, "src/lib.rs".into(), [0x22; 32]);

        let result = exec.try_cache_lookup(200, command, &args, &env, now + 1000);
        assert!(result.is_some(), "should hit cache");

        let result = result.unwrap();
        assert!(result.is_cache_hit);
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, b"compiled OK");
        assert_eq!(result.stderr, b"");
        assert_eq!(result.outputs.len(), 1);
        assert_eq!(result.outputs[0].path, "target/main");
    }

    #[test]
    fn different_input_produces_miss() {
        let exec = make_executor();
        exec.tracker().set_enabled(true);

        let now = 1_000_000u64;
        let command = "rustc";
        let args = vec!["src/main.rs".to_string()];
        let env = sample_env();

        // Store with input hash [0x11; 32]
        exec.tracker().start_session(100);
        exec.tracker().record_read(100, "src/main.rs".into(), [0x11; 32]);
        let read_set = exec.tracker().finalize(100).unwrap();
        exec.store_result(&StoreResultParams {
            command,
            args: &args,
            env: &env,
            read_set: &read_set,
            exit_code: 0,
            stdout: b"ok",
            stderr: b"",
            output_files: &[],
            now_ms: now,
        })
        .unwrap();

        // Lookup with different input hash [0x22; 32]
        exec.tracker().start_session(200);
        exec.tracker().record_read(200, "src/main.rs".into(), [0x22; 32]);
        let result = exec.try_cache_lookup(200, command, &args, &env, now + 1000);
        assert!(result.is_none(), "different input should miss");
    }

    #[test]
    fn materialize_outputs_fetches_blobs() {
        let exec = make_executor();
        exec.tracker().set_enabled(true);

        let now = 1_000_000u64;
        let command = "gcc";
        let args = vec!["main.c".to_string()];
        let env = sample_env();

        exec.tracker().start_session(100);
        exec.tracker().record_read(100, "main.c".into(), [0x11; 32]);
        let read_set = exec.tracker().finalize(100).unwrap();

        exec.store_result(&StoreResultParams {
            command,
            args: &args,
            env: &env,
            read_set: &read_set,
            exit_code: 0,
            stdout: b"",
            stderr: b"",
            output_files: &[("a.out", b"ELF binary"), ("a.o", b"object file")],
            now_ms: now,
        })
        .unwrap();

        // Lookup and materialize
        exec.tracker().start_session(200);
        exec.tracker().record_read(200, "main.c".into(), [0x11; 32]);
        let result = exec.try_cache_lookup(200, command, &args, &env, now + 1000).unwrap();

        let files = exec.materialize_outputs(&result.outputs).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].1, b"ELF binary");
        assert_eq!(files[1].1, b"object file");
    }

    #[test]
    fn stores_failed_execution() {
        let exec = make_executor();
        exec.tracker().set_enabled(true);

        let now = 1_000_000u64;
        exec.tracker().start_session(100);
        exec.tracker().record_read(100, "bad.c".into(), [0x11; 32]);
        let read_set = exec.tracker().finalize(100).unwrap();

        exec.store_result(&StoreResultParams {
            command: "gcc",
            args: &["bad.c".into()],
            env: &sample_env(),
            read_set: &read_set,
            exit_code: 1,
            stdout: b"",
            stderr: b"error: syntax",
            output_files: &[],
            now_ms: now,
        })
        .unwrap();

        // Replays the failure
        exec.tracker().start_session(200);
        exec.tracker().record_read(200, "bad.c".into(), [0x11; 32]);
        let result = exec.try_cache_lookup(200, "gcc", &["bad.c".into()], &sample_env(), now + 1000).unwrap();
        assert_eq!(result.exit_code, 1);
        assert_eq!(result.stderr, b"error: syntax");
    }

    #[test]
    fn stats_tracking() {
        let exec = make_executor();
        exec.tracker().set_enabled(true);

        let now = 1_000_000u64;

        // Miss
        exec.tracker().start_session(100);
        exec.tracker().record_read(100, "f".into(), [0x11; 32]);
        let _ = exec.try_cache_lookup(100, "cmd", &[], &sample_env(), now);

        let stats = exec.stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 0);

        // Store, then hit
        exec.tracker().start_session(200);
        exec.tracker().record_read(200, "f".into(), [0x11; 32]);
        let rs = exec.tracker().finalize(200).unwrap();
        exec.store_result(&StoreResultParams {
            command: "cmd",
            args: &[],
            env: &sample_env(),
            read_set: &rs,
            exit_code: 0,
            stdout: b"",
            stderr: b"",
            output_files: &[],
            now_ms: now,
        })
        .unwrap();

        exec.tracker().start_session(300);
        exec.tracker().record_read(300, "f".into(), [0x11; 32]);
        let _ = exec.try_cache_lookup(300, "cmd", &[], &sample_env(), now + 1000);

        let stats = exec.stats();
        assert_eq!(stats.hits, 1);
    }

    #[test]
    fn rejects_too_many_output_files() {
        let exec = make_executor();
        exec.tracker().set_enabled(true);

        exec.tracker().start_session(100);
        let read_set = exec.tracker().finalize(100).unwrap();

        let too_many: Vec<(&str, &[u8])> = (0..MAX_OUTPUT_FILES + 1).map(|_| ("f", &b"x"[..])).collect();

        let result = exec.store_result(&StoreResultParams {
            command: "cmd",
            args: &[],
            env: &sample_env(),
            read_set: &read_set,
            exit_code: 0,
            stdout: b"",
            stderr: b"",
            output_files: &too_many,
            now_ms: 1000,
        });
        assert!(matches!(result, Err(ExecCacheError::TooManyOutputFiles { .. })));
    }

    #[test]
    fn child_key_validation() {
        let exec = make_executor();
        exec.tracker().set_enabled(true);

        let now = 1_000_000u64;

        // Store a child result
        exec.tracker().start_session(10);
        exec.tracker().record_read(10, "child.rs".into(), [0x11; 32]);
        let child_rs = exec.tracker().finalize(10).unwrap();
        let child_key = exec
            .store_result(&StoreResultParams {
                command: "rustc",
                args: &["child.rs".into()],
                env: &sample_env(),
                read_set: &child_rs,
                exit_code: 0,
                stdout: b"",
                stderr: b"",
                output_files: &[],
                now_ms: now,
            })
            .unwrap();

        // Store a parent result referencing the child
        let parent_rs = ReadSet {
            records: vec![crate::read_tracker::ReadRecord {
                path: "parent.rs".into(),
                hash: [0x22; 32],
            }],
            child_keys: vec![child_key],
        };
        exec.store_result(&StoreResultParams {
            command: "cargo",
            args: &["build".into()],
            env: &sample_env(),
            read_set: &parent_rs,
            exit_code: 0,
            stdout: b"",
            stderr: b"",
            output_files: &[],
            now_ms: now,
        })
        .unwrap();

        // Lookup parent — should hit because child is still valid
        exec.tracker().start_session(200);
        exec.tracker().record_read(200, "parent.rs".into(), [0x22; 32]);
        let result = exec.try_cache_lookup(200, "cargo", &["build".into()], &sample_env(), now + 1000);
        assert!(result.is_some(), "parent should hit when child is valid");

        // Delete child, then parent should miss
        exec.index.delete(&child_key).unwrap();

        exec.tracker().start_session(300);
        exec.tracker().record_read(300, "parent.rs".into(), [0x22; 32]);
        let result = exec.try_cache_lookup(300, "cargo", &["build".into()], &sample_env(), now + 2000);
        assert!(result.is_none(), "parent should miss when child is invalid");
    }
}
