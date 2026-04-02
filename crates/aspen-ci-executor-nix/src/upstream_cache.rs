//! HTTP client for upstream Nix binary caches (e.g., cache.nixos.org).
//!
//! Fetches narinfo metadata and NAR archives from upstream caches to populate
//! the cluster's PathInfoService. This enables closure bootstrap without a
//! local nix installation — the cluster can resolve transitive dependencies
//! by querying the public binary cache.

use std::collections::HashSet;
use std::collections::VecDeque;
use std::io::Cursor;
use std::sync::Arc;

use nix_compat::nixbase32;
use nix_compat::store_path::StorePath;
use snix_castore::blobservice::BlobService;
use snix_castore::directoryservice::DirectoryService;
use snix_store::nar::ingest_nar_and_hash;
use snix_store::pathinfoservice::PathInfoService;
use tracing::debug;
use tracing::info;
use tracing::warn;

// ============================================================================
// Tiger Style: all limits explicit
// ============================================================================

/// Maximum NAR size to fetch from upstream (2 GB).
const MAX_NAR_FETCH_SIZE: u64 = 2 * 1024 * 1024 * 1024;

/// Maximum number of paths in a single closure population.
const MAX_CLOSURE_PATHS: usize = 50_000;

/// HTTP timeout for narinfo requests (30s).
const NARINFO_TIMEOUT_SECS: u64 = 30;

/// HTTP timeout for NAR downloads (300s).
const NAR_TIMEOUT_SECS: u64 = 300;

/// Maximum concurrent NAR fetches.
const MAX_CONCURRENT_FETCHES: usize = 8;

/// Default upstream cache URL.
pub const DEFAULT_CACHE_URL: &str = "https://cache.nixos.org";

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for an upstream binary cache.
#[derive(Debug, Clone)]
pub struct UpstreamCacheEntry {
    /// Base URL (e.g., "https://cache.nixos.org").
    pub url: String,
    /// Trusted public key (e.g., "cache.nixos.org-1:6NCH...").
    pub public_key: String,
}

/// Configuration for upstream cache client.
#[derive(Debug, Clone)]
pub struct UpstreamCacheConfig {
    /// Ordered list of upstream caches to query.
    pub caches: Vec<UpstreamCacheEntry>,
}

impl Default for UpstreamCacheConfig {
    fn default() -> Self {
        Self {
            caches: vec![UpstreamCacheEntry {
                url: DEFAULT_CACHE_URL.to_string(),
                public_key: "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=".to_string(),
            }],
        }
    }
}

// ============================================================================
// Parsed narinfo
// ============================================================================

/// Parsed narinfo metadata from an upstream cache.
#[derive(Debug, Clone)]
pub struct NarInfoResponse {
    /// Store path basename (e.g., "abc123-hello-2.12.1").
    pub store_path: String,
    /// URL of the NAR file (relative to cache base).
    pub url: String,
    /// Compression type ("xz", "zstd", "bzip2", "none").
    pub compression: String,
    /// NAR hash (SHA-256, raw bytes).
    pub nar_sha256: [u8; 32],
    /// NAR size in bytes.
    pub nar_size: u64,
    /// File hash of the compressed NAR (for verification).
    pub file_hash: Option<String>,
    /// File size of the compressed NAR.
    pub file_size: Option<u64>,
    /// Store path references (basenames).
    pub references: Vec<String>,
    /// Deriver store path basename, if any.
    pub deriver: Option<String>,
}

// ============================================================================
// UpstreamCacheClient
// ============================================================================

/// HTTP client for querying upstream Nix binary caches.
pub struct UpstreamCacheClient {
    client: reqwest::Client,
    config: UpstreamCacheConfig,
    blob_service: Arc<dyn BlobService>,
    directory_service: Arc<dyn DirectoryService>,
    pathinfo_service: Arc<dyn PathInfoService>,
}

impl UpstreamCacheClient {
    /// Create a new client with the given config and snix services.
    pub fn new(
        config: UpstreamCacheConfig,
        blob_service: Arc<dyn BlobService>,
        directory_service: Arc<dyn DirectoryService>,
        pathinfo_service: Arc<dyn PathInfoService>,
    ) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(NAR_TIMEOUT_SECS))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| format!("failed to build HTTP client: {e}"))?;

        Ok(Self {
            client,
            config,
            blob_service,
            directory_service,
            pathinfo_service,
        })
    }

    /// Fetch narinfo for a store path hash from the first cache that has it.
    ///
    /// `store_path_hash` is the nix32-encoded 20-byte digest (e.g.,
    /// "w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb").
    pub async fn fetch_narinfo(&self, store_path_hash: &str) -> Result<NarInfoResponse, FetchError> {
        for cache in &self.config.caches {
            let url = format!("{}/{}.narinfo", cache.url.trim_end_matches('/'), store_path_hash);

            let response = self
                .client
                .get(&url)
                .timeout(std::time::Duration::from_secs(NARINFO_TIMEOUT_SECS))
                .send()
                .await
                .map_err(|e| FetchError::Network(format!("{url}: {e}")))?;

            if response.status() == reqwest::StatusCode::NOT_FOUND {
                debug!(hash = %store_path_hash, cache = %cache.url, "narinfo not found");
                continue;
            }

            if !response.status().is_success() {
                warn!(hash = %store_path_hash, status = %response.status(), "narinfo request failed");
                continue;
            }

            let body = response.text().await.map_err(|e| FetchError::Network(format!("reading narinfo body: {e}")))?;

            return parse_narinfo(&body)
                .map_err(|e| FetchError::Parse(format!("parsing narinfo for {store_path_hash}: {e}")));
        }

        Err(FetchError::NotFound(store_path_hash.to_string()))
    }

    /// Download a NAR from the upstream cache and ingest it into snix services.
    ///
    /// Delegates to `fetch_and_ingest_nar_inner` with this client's services.
    pub async fn fetch_and_ingest_nar(&self, cache_url: &str, narinfo: &NarInfoResponse) -> Result<(), FetchError> {
        fetch_and_ingest_nar_inner(
            &self.client,
            cache_url,
            narinfo,
            Arc::clone(&self.blob_service),
            Arc::clone(&self.directory_service),
            &*self.pathinfo_service,
        )
        .await
    }

    /// Populate the cluster's PathInfoService with the transitive closure
    /// of the given store path basenames from upstream caches.
    ///
    /// BFS over narinfo references with concurrent NAR fetching bounded by
    /// `MAX_CONCURRENT_FETCHES`. Narinfo lookups (cheap) run inline to
    /// discover references; NAR downloads (expensive) run concurrently via
    /// a `JoinSet`. Completed fetches enqueue newly-discovered references
    /// back into the BFS queue.
    pub async fn populate_closure(&self, initial_basenames: &[String]) -> Result<PopulateReport, FetchError> {
        let mut report = PopulateReport::default();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_FETCHES));
        let mut fetch_tasks: tokio::task::JoinSet<FetchTaskResult> = tokio::task::JoinSet::new();

        // Seed queue
        for name in initial_basenames {
            if visited.insert(name.clone()) {
                queue.push_back(name.clone());
            }
        }

        loop {
            // Drain the BFS queue: resolve narinfos inline, spawn NAR fetches
            while let Some(basename) = queue.pop_front() {
                if visited.len() > MAX_CLOSURE_PATHS {
                    warn!(limit = MAX_CLOSURE_PATHS, visited = visited.len(), "closure population hit path limit");
                    report.hit_limit = true;
                    break;
                }

                // Check if already in PathInfoService
                let abs_path = format!("/nix/store/{basename}");
                if let Ok(sp) = StorePath::<String>::from_absolute_path(abs_path.as_bytes())
                    && let Ok(Some(existing)) = self.pathinfo_service.get(*sp.digest()).await
                {
                    report.already_present = report.already_present.saturating_add(1);
                    for reference in &existing.references {
                        let ref_name = reference.to_string();
                        if visited.insert(ref_name.clone()) {
                            queue.push_back(ref_name);
                        }
                    }
                    continue;
                }

                // Extract store path hash from basename
                let hash_part = match basename.find('-') {
                    Some(idx) => basename[..idx].to_string(),
                    None => {
                        report.unresolved.push(basename.clone());
                        continue;
                    }
                };

                // Fetch narinfo inline (cheap, ~50ms)
                let narinfo = match self.fetch_narinfo(&hash_part).await {
                    Ok(ni) => ni,
                    Err(FetchError::NotFound(_)) => {
                        debug!(basename = %basename, "not found in any upstream cache");
                        report.unresolved.push(basename);
                        continue;
                    }
                    Err(e) => {
                        warn!(basename = %basename, error = %e, "upstream narinfo fetch failed");
                        report.errors.push(format!("{basename}: {e}"));
                        continue;
                    }
                };

                // Enqueue references for BFS immediately
                for reference in &narinfo.references {
                    if visited.insert(reference.clone()) {
                        queue.push_back(reference.clone());
                    }
                }

                // Spawn concurrent NAR fetch (expensive, seconds)
                let cache_url =
                    self.config.caches.first().map(|c| c.url.clone()).unwrap_or_else(|| DEFAULT_CACHE_URL.to_string());
                let client = self.client.clone();
                let blob_svc = Arc::clone(&self.blob_service);
                let dir_svc = Arc::clone(&self.directory_service);
                let pathinfo_svc = Arc::clone(&self.pathinfo_service);
                let permit = semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|_| FetchError::Network("semaphore closed".to_string()))?;
                let bn = basename.clone();

                fetch_tasks.spawn(async move {
                    let result =
                        fetch_nar_with_retry(client, cache_url, narinfo, blob_svc, dir_svc, pathinfo_svc).await;
                    drop(permit);
                    FetchTaskResult { basename: bn, result }
                });
            }

            // No more items in queue — drain remaining fetch tasks
            if fetch_tasks.is_empty() {
                break;
            }

            // Wait for the next fetch to complete
            match fetch_tasks.join_next().await {
                Some(Ok(task_result)) => {
                    match task_result.result {
                        Ok(retries) => {
                            report.fetched = report.fetched.saturating_add(1);
                            report.retries = report.retries.saturating_add(retries);
                        }
                        Err(e) => {
                            warn!(basename = %task_result.basename, error = %e, "NAR fetch/ingest failed");
                            report.errors.push(format!("{}: {e}", task_result.basename));
                        }
                    }
                    // The BFS queue may have new items from earlier narinfo lookups;
                    // loop back to drain it before waiting for more fetches.
                }
                Some(Err(e)) => {
                    warn!(error = %e, "NAR fetch task panicked");
                    report.errors.push(format!("task panic: {e}"));
                }
                None => break,
            }
        }

        // Drain any remaining completed tasks
        while let Some(result) = fetch_tasks.join_next().await {
            match result {
                Ok(task_result) => match task_result.result {
                    Ok(retries) => {
                        report.fetched = report.fetched.saturating_add(1);
                        report.retries = report.retries.saturating_add(retries);
                    }
                    Err(e) => {
                        report.errors.push(format!("{}: {e}", task_result.basename));
                    }
                },
                Err(e) => {
                    report.errors.push(format!("task panic: {e}"));
                }
            }
        }

        info!(
            fetched = report.fetched,
            already_present = report.already_present,
            unresolved = report.unresolved.len(),
            errors = report.errors.len(),
            retries = report.retries,
            "closure population complete"
        );

        Ok(report)
    }
}

/// Result from a single concurrent NAR fetch task.
struct FetchTaskResult {
    basename: String,
    /// Ok(retry_count) on success, Err on failure.
    result: Result<u32, FetchError>,
}

/// Maximum number of retry attempts for transient errors.
const MAX_RETRIES: u32 = 2;

/// Fetch and ingest a NAR with retry for transient HTTP errors.
///
/// Retries on 5xx, timeout, and connection reset. Does NOT retry on
/// 404, hash mismatch, or decompression errors.
///
/// Takes `Arc`s so it can be spawned into a `JoinSet`.
async fn fetch_nar_with_retry(
    client: reqwest::Client,
    cache_url: String,
    narinfo: NarInfoResponse,
    blob_service: Arc<dyn BlobService>,
    directory_service: Arc<dyn DirectoryService>,
    pathinfo_service: Arc<dyn PathInfoService>,
) -> Result<u32, FetchError> {
    let mut retries = 0u32;

    loop {
        match fetch_and_ingest_nar_inner(
            &client,
            &cache_url,
            &narinfo,
            Arc::clone(&blob_service),
            Arc::clone(&directory_service),
            &*pathinfo_service,
        )
        .await
        {
            Ok(()) => return Ok(retries),
            Err(e) if is_transient_error(&e) && retries < MAX_RETRIES => {
                retries = retries.saturating_add(1);
                let backoff_ms = 1000u64.saturating_mul(retries as u64);
                debug!(
                    path = %narinfo.store_path,
                    retry = retries,
                    backoff_ms = backoff_ms,
                    error = %e,
                    "retrying transient NAR fetch error"
                );
                tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
            }
            Err(e) => return Err(e),
        }
    }
}

/// Check if a FetchError is transient and should be retried.
fn is_transient_error(e: &FetchError) -> bool {
    match e {
        FetchError::Network(msg) => {
            // 5xx, timeout, connection reset
            msg.contains("5") || msg.contains("timeout") || msg.contains("reset") || msg.contains("connect")
        }
        FetchError::NotFound(_) | FetchError::Parse(_) | FetchError::Decompress(_) => false,
        FetchError::HashMismatch { .. } => false,
        FetchError::TooLarge { .. } => false,
        FetchError::Ingest(_) => false,
    }
}

// ============================================================================
// PopulateReport
// ============================================================================

/// Result of a `populate_closure` operation.
#[derive(Debug, Default)]
pub struct PopulateReport {
    /// Paths fetched from upstream and ingested.
    pub fetched: u32,
    /// Paths already present in PathInfoService (skipped).
    pub already_present: u32,
    /// Basenames that no upstream cache had.
    pub unresolved: Vec<String>,
    /// Error messages for paths that failed.
    pub errors: Vec<String>,
    /// Whether the closure population hit the path limit.
    pub hit_limit: bool,
    /// Total retry attempts across all fetches.
    pub retries: u32,
}

// ============================================================================
// FetchError
// ============================================================================

/// Errors from upstream cache operations.
#[derive(Debug)]
pub enum FetchError {
    /// HTTP/network error.
    Network(String),
    /// Narinfo not found in any cache.
    NotFound(String),
    /// Narinfo parse error.
    Parse(String),
    /// NAR decompression error.
    Decompress(String),
    /// NAR hash doesn't match narinfo.
    HashMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    /// NAR too large.
    TooLarge { path: String, size: u64, limit: u64 },
    /// Castore ingest error.
    Ingest(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "network error: {msg}"),
            Self::NotFound(hash) => write!(f, "not found: {hash}"),
            Self::Parse(msg) => write!(f, "parse error: {msg}"),
            Self::Decompress(msg) => write!(f, "decompression error: {msg}"),
            Self::HashMismatch { path, expected, actual } => {
                write!(f, "hash mismatch for {path}: expected {expected}, got {actual}")
            }
            Self::TooLarge { path, size, limit } => {
                write!(f, "NAR too large for {path}: {size} > {limit}")
            }
            Self::Ingest(msg) => write!(f, "ingest error: {msg}"),
        }
    }
}

// ============================================================================
// Standalone NAR fetch + ingest (used by concurrent tasks)
// ============================================================================

/// Download, decompress, verify, and ingest a NAR into castore services.
///
/// Standalone function (no `&self`) so it can be called from spawned tasks.
async fn fetch_and_ingest_nar_inner(
    client: &reqwest::Client,
    cache_url: &str,
    narinfo: &NarInfoResponse,
    blob_service: Arc<dyn BlobService>,
    directory_service: Arc<dyn DirectoryService>,
    pathinfo_service: &dyn PathInfoService,
) -> Result<(), FetchError> {
    if narinfo.nar_size > MAX_NAR_FETCH_SIZE {
        return Err(FetchError::TooLarge {
            path: narinfo.store_path.clone(),
            size: narinfo.nar_size,
            limit: MAX_NAR_FETCH_SIZE,
        });
    }

    let nar_url = if narinfo.url.starts_with("http://") || narinfo.url.starts_with("https://") {
        narinfo.url.clone()
    } else {
        format!("{}/{}", cache_url.trim_end_matches('/'), narinfo.url)
    };

    debug!(url = %nar_url, compression = %narinfo.compression, "fetching NAR");

    let response = client
        .get(&nar_url)
        .send()
        .await
        .map_err(|e| FetchError::Network(format!("fetching NAR {nar_url}: {e}")))?;

    if !response.status().is_success() {
        return Err(FetchError::Network(format!("NAR download failed: HTTP {} for {nar_url}", response.status())));
    }

    let compressed_bytes = response.bytes().await.map_err(|e| FetchError::Network(format!("reading NAR body: {e}")))?;

    let nar_bytes = decompress_nar(&compressed_bytes, &narinfo.compression)
        .map_err(|e| FetchError::Decompress(format!("{}: {e}", narinfo.compression)))?;

    let mut cursor = Cursor::new(&nar_bytes);
    let (root_node, nar_sha256, nar_size) = ingest_nar_and_hash(blob_service, directory_service, &mut cursor, &None)
        .await
        .map_err(|e| FetchError::Ingest(format!("NAR ingest failed: {e}")))?;

    if nar_sha256 != narinfo.nar_sha256 {
        return Err(FetchError::HashMismatch {
            path: narinfo.store_path.clone(),
            expected: hex::encode(narinfo.nar_sha256),
            actual: hex::encode(nar_sha256),
        });
    }

    let store_path = StorePath::<String>::from_absolute_path(format!("/nix/store/{}", narinfo.store_path).as_bytes())
        .map_err(|e| FetchError::Parse(format!("invalid store path: {e}")))?;

    let references: Vec<StorePath<String>> = narinfo
        .references
        .iter()
        .filter_map(|r| StorePath::<String>::from_absolute_path(format!("/nix/store/{r}").as_bytes()).ok())
        .collect();

    let deriver = narinfo
        .deriver
        .as_ref()
        .and_then(|d| StorePath::<String>::from_absolute_path(format!("/nix/store/{d}").as_bytes()).ok());

    let path_info = snix_store::path_info::PathInfo {
        store_path,
        node: root_node,
        references,
        nar_size,
        nar_sha256,
        signatures: vec![],
        deriver,
        ca: None,
    };

    pathinfo_service
        .put(path_info)
        .await
        .map_err(|e| FetchError::Ingest(format!("PathInfoService put failed: {e}")))?;

    info!(
        path = %narinfo.store_path,
        nar_size = nar_size,
        "ingested path from upstream cache"
    );

    Ok(())
}

// ============================================================================
// Narinfo parsing
// ============================================================================

/// Parse a narinfo text blob into a `NarInfoResponse`.
///
/// Expected format (one field per line, `Key: Value`):
/// ```text
/// StorePath: /nix/store/abc-hello-2.12.1
/// URL: nar/abc.nar.xz
/// Compression: xz
/// FileHash: sha256:...
/// FileSize: 12345
/// NarHash: sha256:...
/// NarSize: 226552
/// References: abc-glibc-2.38
/// Deriver: xyz-hello-2.12.1.drv
/// Sig: cache.nixos.org-1:...
/// ```
fn parse_narinfo(text: &str) -> Result<NarInfoResponse, String> {
    let mut store_path = None;
    let mut url = None;
    let mut compression = "none".to_string();
    let mut nar_hash_str = None;
    let mut nar_size = None;
    let mut file_hash = None;
    let mut file_size = None;
    let mut references = Vec::new();
    let mut deriver = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = match line.split_once(": ") {
            Some(kv) => kv,
            None => continue,
        };

        match key {
            "StorePath" => {
                // Strip /nix/store/ prefix to get basename
                store_path = Some(value.strip_prefix("/nix/store/").unwrap_or(value).to_string());
            }
            "URL" => url = Some(value.to_string()),
            "Compression" => compression = value.to_string(),
            "NarHash" => nar_hash_str = Some(value.to_string()),
            "NarSize" => {
                nar_size = Some(value.parse::<u64>().map_err(|e| format!("invalid NarSize: {e}"))?);
            }
            "FileHash" => file_hash = Some(value.to_string()),
            "FileSize" => {
                file_size = Some(value.parse::<u64>().ok());
            }
            "References" => {
                if !value.is_empty() {
                    references = value.split_whitespace().map(|s| s.to_string()).collect();
                }
            }
            "Deriver" => {
                if value != "unknown-deriver" {
                    deriver = Some(value.to_string());
                }
            }
            _ => {} // Ignore Sig, CA, System, etc.
        }
    }

    let store_path = store_path.ok_or("missing StorePath")?;
    let url = url.ok_or("missing URL")?;
    let nar_hash_str = nar_hash_str.ok_or("missing NarHash")?;
    let nar_size = nar_size.ok_or("missing NarSize")?;

    // Parse NarHash — expected format: "sha256:<nix32-encoded>" or "sha256:<base16>"
    let nar_sha256 = parse_nar_hash(&nar_hash_str)?;

    Ok(NarInfoResponse {
        store_path,
        url,
        compression,
        nar_sha256,
        nar_size,
        file_hash,
        file_size: file_size.flatten(),
        references,
        deriver,
    })
}

/// Parse a NarHash string like "sha256:<nix32>" or "sha256:<hex>" into raw bytes.
fn parse_nar_hash(hash_str: &str) -> Result<[u8; 32], String> {
    let hash_part = hash_str
        .strip_prefix("sha256:")
        .ok_or_else(|| format!("NarHash must start with sha256:, got: {hash_str}"))?;

    // Try nix32 first (shorter, 52 chars)
    if hash_part.len() == 52 {
        let bytes = nixbase32::decode(hash_part.as_bytes()).map_err(|e| format!("invalid nix32 NarHash: {e}"))?;
        if bytes.len() != 32 {
            return Err(format!("NarHash decoded to {} bytes, expected 32", bytes.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        return Ok(arr);
    }

    // Try hex (64 chars)
    if hash_part.len() == 64 {
        let bytes = hex::decode(hash_part).map_err(|e| format!("invalid hex NarHash: {e}"))?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        return Ok(arr);
    }

    // Try base64 (44 chars with padding, or 43 without)
    if hash_part.len() >= 43 && hash_part.len() <= 44 {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(hash_part)
            .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(hash_part))
            .map_err(|e| format!("invalid base64 NarHash: {e}"))?;
        if bytes.len() != 32 {
            return Err(format!("NarHash decoded to {} bytes, expected 32", bytes.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        return Ok(arr);
    }

    Err(format!("unrecognized NarHash format ({} chars): {hash_part}", hash_part.len()))
}

// ============================================================================
// NAR decompression
// ============================================================================

/// Decompress NAR bytes based on compression type.
fn decompress_nar(data: &[u8], compression: &str) -> Result<Vec<u8>, std::io::Error> {
    use std::io::Read;

    match compression {
        "none" => Ok(data.to_vec()),
        "xz" => {
            let mut decoder = xz2::read::XzDecoder::new(data);
            let mut out = Vec::new();
            decoder.read_to_end(&mut out)?;
            Ok(out)
        }
        "zstd" => {
            let mut decoder = zstd::Decoder::new(data)?;
            let mut out = Vec::new();
            decoder.read_to_end(&mut out)?;
            Ok(out)
        }
        "bzip2" => {
            let mut decoder = bzip2::read::BzDecoder::new(data);
            let mut out = Vec::new();
            decoder.read_to_end(&mut out)?;
            Ok(out)
        }
        other => Err(std::io::Error::new(std::io::ErrorKind::Unsupported, format!("unsupported compression: {other}"))),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_NARINFO: &str = "\
StorePath: /nix/store/w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb-hello-2.12.1
URL: nar/1b0ri7q2g18vi4ql05z4ni5g098drmi2w3sswgjyqqwdrbig0w08.nar.xz
Compression: xz
FileHash: sha256:1b0ri7q2g18vi4ql05z4ni5g098drmi2w3sswgjyqqwdrbig0w08
FileSize: 50000
NarHash: sha256:1b0ri7q2g18vi4ql05z4ni5g098drmi2w3sswgjyqqwdrbig0w08
NarSize: 226552
References: 7gx4kiv5m0i7d7qkixq2cwzbr10lvxwc-glibc-2.38-27
Deriver: dh3yb2m5p1s8dxaqzarq2kq8f1y7sz2m-hello-2.12.1.drv
Sig: cache.nixos.org-1:fakesig";

    #[test]
    fn test_parse_narinfo_complete() {
        let ni = parse_narinfo(SAMPLE_NARINFO).unwrap();
        assert_eq!(ni.store_path, "w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb-hello-2.12.1");
        assert_eq!(ni.compression, "xz");
        assert_eq!(ni.nar_size, 226552);
        assert!(ni.url.contains("nar/"));
        assert_eq!(ni.references.len(), 1);
        assert_eq!(ni.references[0], "7gx4kiv5m0i7d7qkixq2cwzbr10lvxwc-glibc-2.38-27");
        assert!(ni.deriver.is_some());
        assert_eq!(ni.file_size, Some(50000));
    }

    #[test]
    fn test_parse_narinfo_minimal() {
        let text = "\
StorePath: /nix/store/abc-minimal
URL: nar/abc.nar
NarHash: sha256:1b0ri7q2g18vi4ql05z4ni5g098drmi2w3sswgjyqqwdrbig0w08
NarSize: 100
References: ";
        let ni = parse_narinfo(text).unwrap();
        assert_eq!(ni.store_path, "abc-minimal");
        assert_eq!(ni.compression, "none");
        assert_eq!(ni.nar_size, 100);
        assert!(ni.references.is_empty());
        assert!(ni.deriver.is_none());
    }

    #[test]
    fn test_parse_narinfo_missing_store_path() {
        let text = "URL: nar/abc.nar\nNarHash: sha256:abc\nNarSize: 100\n";
        assert!(parse_narinfo(text).is_err());
    }

    #[test]
    fn test_parse_nar_hash_nix32() {
        // 52-char nix32 encoded hash
        let hash = parse_nar_hash("sha256:1b0ri7q2g18vi4ql05z4ni5g098drmi2w3sswgjyqqwdrbig0w08").unwrap();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_parse_nar_hash_hex() {
        let hex_hash = "sha256:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let hash = parse_nar_hash(hex_hash).unwrap();
        assert_eq!(hash[0], 0xab);
        assert_eq!(hash[1], 0xcd);
    }

    #[test]
    fn test_parse_nar_hash_bad_prefix() {
        assert!(parse_nar_hash("md5:abc").is_err());
    }

    #[test]
    fn test_decompress_none() {
        let data = b"hello world";
        let result = decompress_nar(data, "none").unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_decompress_unsupported() {
        assert!(decompress_nar(b"data", "lz4").is_err());
    }

    #[test]
    fn test_upstream_cache_config_default() {
        let config = UpstreamCacheConfig::default();
        assert_eq!(config.caches.len(), 1);
        assert!(config.caches[0].url.contains("cache.nixos.org"));
    }

    #[test]
    fn test_populate_report_default() {
        let report = PopulateReport::default();
        assert_eq!(report.fetched, 0);
        assert_eq!(report.already_present, 0);
        assert!(report.unresolved.is_empty());
        assert!(!report.hit_limit);
    }

    #[test]
    fn test_fetch_error_display() {
        let e = FetchError::NotFound("abc".to_string());
        assert!(e.to_string().contains("not found"));

        let e = FetchError::HashMismatch {
            path: "test".to_string(),
            expected: "aaa".to_string(),
            actual: "bbb".to_string(),
        };
        assert!(e.to_string().contains("hash mismatch"));

        let e = FetchError::TooLarge {
            path: "big".to_string(),
            size: 999,
            limit: 100,
        };
        assert!(e.to_string().contains("too large"));
    }

    #[test]
    fn test_is_transient_error_network_5xx() {
        let e = FetchError::Network("NAR download failed: HTTP 503 for http://example.com".to_string());
        assert!(is_transient_error(&e), "503 should be transient");
    }

    #[test]
    fn test_is_transient_error_timeout() {
        let e = FetchError::Network("connection timeout after 30s".to_string());
        assert!(is_transient_error(&e), "timeout should be transient");
    }

    #[test]
    fn test_is_transient_error_not_found() {
        let e = FetchError::NotFound("abc123".to_string());
        assert!(!is_transient_error(&e), "404 should not be transient");
    }

    #[test]
    fn test_is_transient_error_hash_mismatch() {
        let e = FetchError::HashMismatch {
            path: "test".to_string(),
            expected: "aaa".to_string(),
            actual: "bbb".to_string(),
        };
        assert!(!is_transient_error(&e), "hash mismatch should not be transient");
    }

    #[test]
    fn test_is_transient_error_decompress() {
        let e = FetchError::Decompress("invalid xz stream".to_string());
        assert!(!is_transient_error(&e), "decompress error should not be transient");
    }

    #[test]
    fn test_populate_report_has_retries_field() {
        let mut report = PopulateReport::default();
        assert_eq!(report.retries, 0);
        report.retries = 5;
        assert_eq!(report.retries, 5);
    }
}
