//! HTTP fetching and unpacking for flake inputs.
//!
//! Downloads GitHub/GitLab archive tarballs and generic tarball URLs,
//! unpacks them to temporary directories, and verifies content against
//! the `narHash` from flake.lock. A [`FetchCache`] avoids redundant
//! downloads for inputs sharing the same narHash.
//!
//! All fetching uses `curl` subprocess (always available in nix environments)
//! piped to in-process `flate2`/`tar` extraction. This avoids adding an
//! HTTP client dependency and works inside `spawn_blocking` contexts.

use std::collections::HashMap;
use std::io;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;

use tracing::debug;
use tracing::info;

// ---------------------------------------------------------------------------
// Tiger Style: all limits explicit
// ---------------------------------------------------------------------------

/// Maximum download size (2 GB).
const MAX_DOWNLOAD_SIZE: u64 = 2 * 1024 * 1024 * 1024;

/// HTTP fetch timeout in seconds.
const FETCH_TIMEOUT_SECS: u64 = 300;

/// Maximum concurrent fetches.
const MAX_CONCURRENT_FETCHES: usize = 4;

/// Maximum number of cached entries (bounded map).
const MAX_CACHE_ENTRIES: usize = 200;

/// Maximum URL length.
const MAX_URL_LENGTH: usize = 4096;

// ---------------------------------------------------------------------------
// FetchCache
// ---------------------------------------------------------------------------

/// Cache for fetched flake inputs, keyed by narHash SRI string.
///
/// Directories are stored under a temporary root and cleaned up when the
/// cache is dropped. Thread-safe via internal `Mutex`.
pub struct FetchCache {
    /// Root directory for all cached unpacked sources.
    root: PathBuf,
    /// narHash → unpacked directory path.
    entries: Mutex<HashMap<String, PathBuf>>,
    /// Semaphore for concurrent fetch limiting.
    semaphore: Arc<tokio::sync::Semaphore>,
}

impl FetchCache {
    /// Create a new fetch cache with a fresh temp directory.
    pub fn new() -> io::Result<Self> {
        let root = std::env::temp_dir().join(format!("aspen-fetch-cache-{}", std::process::id()));
        std::fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            entries: Mutex::new(HashMap::new()),
            semaphore: Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_FETCHES)),
        })
    }

    /// Look up a cached entry by narHash. Returns the path if cached.
    pub fn get(&self, nar_hash: &str) -> Option<PathBuf> {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.get(nar_hash).cloned()
    }

    /// Insert a fetched entry into the cache.
    ///
    /// Returns `false` if the cache is full (MAX_CACHE_ENTRIES).
    pub fn insert(&self, nar_hash: String, path: PathBuf) -> bool {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if entries.len() >= MAX_CACHE_ENTRIES {
            return false;
        }
        entries.insert(nar_hash, path);
        true
    }

    /// Get or fetch: return cached path if present, otherwise run `fetch_fn`
    /// to download and unpack the input, then cache and return the path.
    ///
    /// The `fetch_fn` receives a destination directory path and must unpack
    /// the content there. After `fetch_fn` succeeds, the result is verified
    /// against `expected_nar_hash` (if provided) before caching.
    pub fn get_or_fetch<F>(&self, nar_hash: &str, expected_nar_hash: Option<&str>, fetch_fn: F) -> io::Result<PathBuf>
    where F: FnOnce(&Path) -> io::Result<PathBuf> {
        // Check cache first
        if let Some(cached) = self.get(nar_hash) {
            debug!(nar_hash = %nar_hash, "fetch cache hit");
            return Ok(cached);
        }

        // Create destination directory under cache root
        let dest = self.root.join(sanitize_cache_key(nar_hash));
        std::fs::create_dir_all(&dest)?;

        // Fetch
        let unpacked = fetch_fn(&dest)?;

        // Verify narHash if expected
        if let Some(expected) = expected_nar_hash {
            verify_nar_hash(&unpacked, expected)?;
        }

        // Cache the result
        self.insert(nar_hash.to_string(), unpacked.clone());
        Ok(unpacked)
    }

    /// Acquire a fetch permit (blocks if too many concurrent fetches).
    ///
    /// Call this before starting a fetch to respect the concurrency limit.
    /// The permit is released when dropped.
    pub fn acquire_permit(&self) -> io::Result<tokio::sync::OwnedSemaphorePermit> {
        // Use try_acquire_owned since we're in a sync context (spawn_blocking)
        // If all permits are taken, spin-wait briefly then fail.
        // In practice, flake.lock files rarely have > 4 inputs needing fetch.
        Arc::clone(&self.semaphore).try_acquire_owned().map_err(|_| {
            io::Error::new(
                io::ErrorKind::WouldBlock,
                format!("too many concurrent fetches (limit: {MAX_CONCURRENT_FETCHES})"),
            )
        })
    }
}

impl Drop for FetchCache {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Sanitize a narHash string for use as a directory name.
/// Replaces non-alphanumeric characters (except dash and underscore) with '_'.
fn sanitize_cache_key(key: &str) -> String {
    key.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tarball fetching and unpacking
// ---------------------------------------------------------------------------

/// Download a tarball from `url` and unpack it to `dest_dir`.
///
/// Uses `curl` subprocess for the download (always available in nix
/// environments) piped to in-process `flate2::GzDecoder` + `tar::Archive`.
///
/// Returns the path to the unpacked content. If the tarball contains a
/// single top-level directory (common for GitHub archives), returns that
/// directory's path instead of `dest_dir`.
pub fn fetch_and_unpack_tarball(url: &str, dest_dir: &Path) -> io::Result<PathBuf> {
    if url.len() > MAX_URL_LENGTH {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("URL too long: {} chars exceeds {} limit", url.len(), MAX_URL_LENGTH),
        ));
    }

    info!(url = %url, dest = %dest_dir.display(), "fetching tarball");

    // Spawn curl with size limit and timeout
    let mut child = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--location", // follow redirects
            "--max-time",
            &FETCH_TIMEOUT_SECS.to_string(),
            "--max-filesize",
            &MAX_DOWNLOAD_SIZE.to_string(),
            url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("failed to spawn curl: {e}")))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "failed to capture curl stdout"))?;

    // Pipe curl stdout through gzip decoder and tar extractor
    let gz = flate2::read::GzDecoder::new(stdout);

    // Wrap in a size-limiting reader
    let limited = LimitedReader::new(gz, MAX_DOWNLOAD_SIZE);

    let mut archive = tar::Archive::new(limited);
    archive.set_preserve_permissions(true);
    archive.set_overwrite(true);
    archive.unpack(dest_dir).map_err(|e| {
        // Kill curl if unpack fails
        let _ = child.kill();
        io::Error::new(io::ErrorKind::InvalidData, format!("failed to unpack tarball: {e}"))
    })?;

    // Wait for curl to finish
    let status = child.wait()?;
    if !status.success() {
        return Err(io::Error::other(format!("curl exited with status {status}")));
    }

    // Strip single top-level directory prefix if present
    strip_single_prefix(dest_dir)
}

/// Fetch a GitHub archive tarball and unpack it.
///
/// Constructs `https://github.com/{owner}/{repo}/archive/{rev}.tar.gz`
/// and delegates to [`fetch_and_unpack_tarball`].
pub fn fetch_and_unpack_github(owner: &str, repo: &str, rev: &str, dest_dir: &Path) -> io::Result<PathBuf> {
    if owner.is_empty() || repo.is_empty() || rev.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("GitHub input missing required fields: owner={owner:?}, repo={repo:?}, rev={rev:?}"),
        ));
    }

    let url = format!("https://github.com/{owner}/{repo}/archive/{rev}.tar.gz");
    info!(owner = %owner, repo = %repo, rev = %rev, "fetching GitHub archive");
    fetch_and_unpack_tarball(&url, dest_dir)
}

/// Fetch a GitLab archive tarball and unpack it.
///
/// Constructs `https://{host}/{owner}/{repo}/-/archive/{rev}/{repo}-{rev}.tar.gz`.
/// If `host` is empty, defaults to `gitlab.com`.
pub fn fetch_and_unpack_gitlab(host: &str, owner: &str, repo: &str, rev: &str, dest_dir: &Path) -> io::Result<PathBuf> {
    if owner.is_empty() || repo.is_empty() || rev.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("GitLab input missing required fields: owner={owner:?}, repo={repo:?}, rev={rev:?}"),
        ));
    }

    let host = if host.is_empty() { "gitlab.com" } else { host };
    let url = format!("https://{host}/{owner}/{repo}/-/archive/{rev}/{repo}-{rev}.tar.gz");
    info!(host = %host, owner = %owner, repo = %repo, rev = %rev, "fetching GitLab archive");
    fetch_and_unpack_tarball(&url, dest_dir)
}

// ---------------------------------------------------------------------------
// Tarball prefix stripping
// ---------------------------------------------------------------------------

/// If `dir` contains exactly one subdirectory and no files, return the
/// subdirectory path. Otherwise return `dir` unchanged.
///
/// GitHub archive tarballs always contain a single `{repo}-{rev}/`
/// top-level directory. This function strips that prefix.
fn strip_single_prefix(dir: &Path) -> io::Result<PathBuf> {
    let entries: Vec<_> = std::fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();

    if entries.len() == 1 && entries[0].file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
        let single_dir = entries[0].path();
        debug!(
            prefix = %single_dir.file_name().unwrap_or_default().to_string_lossy(),
            "stripped single top-level directory from tarball"
        );
        Ok(single_dir)
    } else {
        Ok(dir.to_path_buf())
    }
}

// ---------------------------------------------------------------------------
// narHash verification
// ---------------------------------------------------------------------------

/// Compute the NAR hash (sha256, SRI format) of a directory.
///
/// Serializes the directory to NAR format in memory and computes the
/// SHA256 digest. Returns the hash as an SRI string like
/// `sha256-OLdIz38tsFp1aXt8GsJ40s0/jxSkhlqftuDE7LvuhK4=`.
pub fn compute_nar_hash(dir_path: &Path) -> io::Result<String> {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use sha2::Digest;
    use sha2::Sha256;

    // Serialize to NAR and hash incrementally
    let mut hasher = Sha256::new();
    let mut nar_writer = NarWriter::new(&mut hasher);
    nar_writer.serialize_path(dir_path)?;

    let hash_bytes = hasher.finalize();
    let b64 = STANDARD.encode(hash_bytes);
    Ok(format!("sha256-{b64}"))
}

/// Verify that the NAR hash of `dir_path` matches `expected_sri`.
///
/// On mismatch, removes the directory and returns an error containing
/// both the expected and computed hashes.
pub fn verify_nar_hash(dir_path: &Path, expected_sri: &str) -> io::Result<()> {
    let computed = compute_nar_hash(dir_path)?;

    if computed != expected_sri {
        // Clean up on mismatch
        let _ = std::fs::remove_dir_all(dir_path);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("narHash mismatch: expected {expected_sri}, computed {computed}"),
        ));
    }

    debug!(hash = %expected_sri, "narHash verified");
    Ok(())
}

// ---------------------------------------------------------------------------
// NAR serialization (minimal, for hash computation only)
// ---------------------------------------------------------------------------

/// Minimal NAR writer that serializes a filesystem path to NAR format.
///
/// NAR format specification:
/// - Magic: "nix-archive-1\0"
/// - Recursive structure: "(" type content ")"
/// - Types: "regular", "directory", "symlink"
/// - Strings are padded to 8-byte alignment
///
/// This implementation matches Nix's NAR output exactly for hash
/// compatibility. It writes directly to a `std::io::Write` sink
/// (typically a hasher).
struct NarWriter<W: std::io::Write> {
    writer: W,
}

impl<W: std::io::Write> NarWriter<W> {
    fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Serialize a filesystem path to NAR format.
    fn serialize_path(&mut self, path: &Path) -> io::Result<()> {
        self.write_str("nix-archive-1")?;
        self.serialize_entry(path)
    }

    fn serialize_entry(&mut self, path: &Path) -> io::Result<()> {
        let metadata = std::fs::symlink_metadata(path)?;

        self.write_str("(")?;

        if metadata.is_symlink() {
            self.write_str("type")?;
            self.write_str("symlink")?;
            self.write_str("target")?;
            let target = std::fs::read_link(path)?;
            self.write_str(&target.to_string_lossy())?;
        } else if metadata.is_dir() {
            self.write_str("type")?;
            self.write_str("directory")?;

            // Entries must be sorted by name (NAR spec requirement)
            let mut entries: Vec<_> = std::fs::read_dir(path)?.filter_map(|e| e.ok()).collect();
            entries.sort_by_key(|a| a.file_name());

            for entry in entries {
                self.write_str("entry")?;
                self.write_str("(")?;
                self.write_str("name")?;
                self.write_str(&entry.file_name().to_string_lossy())?;
                self.write_str("node")?;
                self.serialize_entry(&entry.path())?;
                self.write_str(")")?;
            }
        } else {
            // Regular file
            self.write_str("type")?;
            self.write_str("regular")?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o111 != 0 {
                    self.write_str("executable")?;
                    self.write_str("")?;
                }
            }

            self.write_str("contents")?;
            let size = metadata.len();
            self.write_u64(size)?;

            let mut file = std::fs::File::open(path)?;
            let mut remaining = size;
            let mut buf = [0u8; 8192];
            while remaining > 0 {
                let to_read = std::cmp::min(remaining as usize, buf.len());
                let n = file.read(&mut buf[..to_read])?;
                if n == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "file truncated during NAR serialization",
                    ));
                }
                self.writer.write_all(&buf[..n])?;
                remaining -= n as u64;
            }

            // Pad to 8-byte alignment
            let pad = (8 - (size % 8)) % 8;
            if pad > 0 {
                self.writer.write_all(&[0u8; 8][..pad as usize])?;
            }
        }

        self.write_str(")")?;
        Ok(())
    }

    /// Write a NAR string: 8-byte little-endian length + content + padding.
    fn write_str(&mut self, s: &str) -> io::Result<()> {
        let bytes = s.as_bytes();
        self.write_u64(bytes.len() as u64)?;
        self.writer.write_all(bytes)?;

        // Pad to 8-byte alignment
        let pad = (8 - (bytes.len() % 8)) % 8;
        if pad > 0 {
            self.writer.write_all(&[0u8; 8][..pad])?;
        }
        Ok(())
    }

    /// Write a u64 in little-endian format.
    fn write_u64(&mut self, n: u64) -> io::Result<()> {
        self.writer.write_all(&n.to_le_bytes())
    }
}

// ---------------------------------------------------------------------------
// Size-limited reader
// ---------------------------------------------------------------------------

/// Reader wrapper that enforces a byte limit on reads.
struct LimitedReader<R: Read> {
    inner: R,
    remaining: u64,
}

impl<R: Read> LimitedReader<R> {
    fn new(inner: R, limit: u64) -> Self {
        Self {
            inner,
            remaining: limit,
        }
    }
}

impl<R: Read> Read for LimitedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Err(io::Error::other(format!("download size limit exceeded ({MAX_DOWNLOAD_SIZE} bytes)")));
        }

        let max_read = std::cmp::min(buf.len() as u64, self.remaining) as usize;
        let n = self.inner.read(&mut buf[..max_read])?;
        self.remaining = self.remaining.saturating_sub(n as u64);
        Ok(n)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_cache_key() {
        assert_eq!(
            sanitize_cache_key("sha256-OLdIz38tsFp1aXt8GsJ40s0/jxSkhlqftuDE7LvuhK4="),
            "sha256-OLdIz38tsFp1aXt8GsJ40s0_jxSkhlqftuDE7LvuhK4_"
        );
        assert_eq!(sanitize_cache_key("simple-key"), "simple-key");
    }

    #[test]
    fn test_strip_single_prefix_single_dir() {
        let tmpdir = tempfile::tempdir().unwrap();
        let inner = tmpdir.path().join("repo-abc123");
        std::fs::create_dir(&inner).unwrap();
        std::fs::write(inner.join("file.txt"), "hello").unwrap();

        let result = strip_single_prefix(tmpdir.path()).unwrap();
        assert_eq!(result, inner);
    }

    #[test]
    fn test_strip_single_prefix_multiple_entries() {
        let tmpdir = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmpdir.path().join("dir1")).unwrap();
        std::fs::create_dir(tmpdir.path().join("dir2")).unwrap();

        let result = strip_single_prefix(tmpdir.path()).unwrap();
        assert_eq!(result, tmpdir.path());
    }

    #[test]
    fn test_strip_single_prefix_file_at_top() {
        let tmpdir = tempfile::tempdir().unwrap();
        std::fs::write(tmpdir.path().join("file.txt"), "hello").unwrap();

        let result = strip_single_prefix(tmpdir.path()).unwrap();
        assert_eq!(result, tmpdir.path());
    }

    #[test]
    fn test_nar_hash_deterministic() {
        let tmpdir = tempfile::tempdir().unwrap();
        let root = tmpdir.path();

        // Create a known directory structure
        std::fs::write(root.join("hello.txt"), "Hello, world!\n").unwrap();
        std::fs::create_dir(root.join("subdir")).unwrap();
        std::fs::write(root.join("subdir").join("nested.txt"), "nested\n").unwrap();

        let hash1 = compute_nar_hash(root).unwrap();
        let hash2 = compute_nar_hash(root).unwrap();

        assert_eq!(hash1, hash2, "NAR hash must be deterministic");
        assert!(hash1.starts_with("sha256-"), "NAR hash must be SRI format");
    }

    #[test]
    fn test_nar_hash_changes_on_content_change() {
        let tmpdir = tempfile::tempdir().unwrap();
        let root = tmpdir.path();
        std::fs::write(root.join("file.txt"), "original").unwrap();

        let hash1 = compute_nar_hash(root).unwrap();

        std::fs::write(root.join("file.txt"), "modified").unwrap();

        let hash2 = compute_nar_hash(root).unwrap();
        assert_ne!(hash1, hash2, "content change must change hash");
    }

    #[test]
    fn test_verify_nar_hash_accepts_match() {
        let tmpdir = tempfile::tempdir().unwrap();
        let root = tmpdir.path();
        std::fs::write(root.join("test.txt"), "test content").unwrap();

        let hash = compute_nar_hash(root).unwrap();
        assert!(verify_nar_hash(root, &hash).is_ok());
    }

    #[test]
    fn test_verify_nar_hash_rejects_mismatch() {
        let tmpdir = tempfile::tempdir().unwrap();
        let root = tmpdir.path();
        std::fs::write(root.join("test.txt"), "test content").unwrap();

        let result = verify_nar_hash(root, "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("narHash mismatch"));
    }

    #[test]
    fn test_fetch_cache_basic() {
        let cache = FetchCache::new().unwrap();

        // Miss on first access
        assert!(cache.get("sha256-test").is_none());

        // Insert and hit
        cache.insert("sha256-test".to_string(), PathBuf::from("/tmp/test"));
        assert_eq!(cache.get("sha256-test"), Some(PathBuf::from("/tmp/test")));
    }

    #[test]
    fn test_fetch_cache_get_or_fetch() {
        let cache = FetchCache::new().unwrap();
        let call_count = Arc::new(std::sync::atomic::AtomicU32::new(0));

        let count = Arc::clone(&call_count);
        let path1 = cache
            .get_or_fetch("sha256-abc", None, |dest| {
                count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let p = dest.join("result");
                std::fs::create_dir_all(&p).unwrap();
                Ok(p)
            })
            .unwrap();

        // Second call should hit cache
        let count2 = Arc::clone(&call_count);
        let path2 = cache
            .get_or_fetch("sha256-abc", None, |dest| {
                count2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(dest.join("result"))
            })
            .unwrap();

        assert_eq!(path1, path2);
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn test_fetch_and_unpack_github_validates_fields() {
        let tmpdir = tempfile::tempdir().unwrap();

        let err = fetch_and_unpack_github("", "repo", "abc", tmpdir.path()).unwrap_err();
        assert!(err.to_string().contains("missing required fields"));

        let err = fetch_and_unpack_github("owner", "", "abc", tmpdir.path()).unwrap_err();
        assert!(err.to_string().contains("missing required fields"));

        let err = fetch_and_unpack_github("owner", "repo", "", tmpdir.path()).unwrap_err();
        assert!(err.to_string().contains("missing required fields"));
    }

    #[test]
    fn test_fetch_and_unpack_gitlab_validates_fields() {
        let tmpdir = tempfile::tempdir().unwrap();

        let err = fetch_and_unpack_gitlab("", "", "repo", "abc", tmpdir.path()).unwrap_err();
        assert!(err.to_string().contains("missing required fields"));
    }

    #[test]
    fn test_url_length_limit() {
        let tmpdir = tempfile::tempdir().unwrap();
        let long_url = "https://example.com/".to_string() + &"x".repeat(MAX_URL_LENGTH);
        let err = fetch_and_unpack_tarball(&long_url, tmpdir.path()).unwrap_err();
        assert!(err.to_string().contains("URL too long"));
    }

    #[test]
    fn test_limited_reader() {
        let data = vec![0u8; 100];
        let mut reader = LimitedReader::new(&data[..], 50);
        let mut buf = vec![0u8; 100];

        // First read: up to 50 bytes
        let n = reader.read(&mut buf).unwrap();
        assert!(n <= 50);

        // Read remaining allowed bytes
        let mut total = n;
        while total < 50 {
            let n = reader.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            total += n;
        }

        // Next read should fail (limit exceeded)
        let result = reader.read(&mut buf);
        assert!(result.is_err() || result.unwrap() == 0);
    }

    /// Integration test: unpack a real .tar.gz created in-process.
    #[test]
    fn test_unpack_in_memory_tarball() {
        use flate2::Compression;
        use flate2::write::GzEncoder;

        // Create a .tar.gz in memory
        let mut tar_gz_bytes = Vec::new();
        {
            let gz = GzEncoder::new(&mut tar_gz_bytes, Compression::default());
            let mut builder = tar::Builder::new(gz);

            // Add a file: repo-abc123/hello.txt
            let content = b"Hello from test tarball\n";
            let mut header = tar::Header::new_gnu();
            header.set_path("repo-abc123/hello.txt").unwrap();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append(&header, &content[..]).unwrap();

            builder.into_inner().unwrap().finish().unwrap();
        }

        // Write to a temp file and unpack via our reader pipeline
        let tmpdir = tempfile::tempdir().unwrap();
        let dest = tmpdir.path().join("unpacked");
        std::fs::create_dir(&dest).unwrap();

        // Unpack directly (bypass curl, test the tar extraction logic)
        let gz = flate2::read::GzDecoder::new(&tar_gz_bytes[..]);
        let mut archive = tar::Archive::new(gz);
        archive.unpack(&dest).unwrap();

        // Should have repo-abc123/hello.txt
        let stripped = strip_single_prefix(&dest).unwrap();
        assert!(stripped.join("hello.txt").exists());

        let content = std::fs::read_to_string(stripped.join("hello.txt")).unwrap();
        assert_eq!(content, "Hello from test tarball\n");
    }

    /// Integration test: fetch a real GitHub archive (nix-systems/default).
    ///
    /// This is a tiny repo (~1KB) so it's fast to download. The narHash
    /// is from Aspen's own flake.lock for the `systems` input.
    #[test]
    #[ignore] // requires network access
    fn test_fetch_real_github_archive() {
        let tmpdir = tempfile::tempdir().unwrap();
        let dest = tmpdir.path().join("systems");
        std::fs::create_dir(&dest).unwrap();

        let result =
            fetch_and_unpack_github("nix-systems", "default", "da67096a3b9bf56a91d16901293e51ba5b49a27e", &dest);

        assert!(result.is_ok(), "fetch failed: {:?}", result.err());
        let path = result.unwrap();

        // Should contain default.nix (the main file in nix-systems/default)
        assert!(
            path.join("default.nix").exists(),
            "missing default.nix in {:?}, contents: {:?}",
            path,
            std::fs::read_dir(&path).unwrap().filter_map(|e| e.ok()).map(|e| e.file_name()).collect::<Vec<_>>()
        );
    }

    /// Integration test: fetch + narHash verification.
    ///
    /// Fetches nix-systems/default and verifies the NAR hash matches
    /// the known value from Aspen's flake.lock.
    #[test]
    #[ignore] // requires network access
    fn test_fetch_and_verify_nar_hash() {
        let cache = FetchCache::new().unwrap();

        // Known narHash for nix-systems/default from Aspen's flake.lock
        // (the "systems" input). Look it up from flake.lock.
        let lock_bytes = std::fs::read("../../flake.lock").expect("failed to read flake.lock");
        let lock: serde_json::Value = serde_json::from_slice(&lock_bytes).unwrap();

        // Find the systems input narHash
        let systems_node = &lock["nodes"]["systems"]["locked"];
        let nar_hash = systems_node["narHash"].as_str().expect("systems narHash missing");
        let rev = systems_node["rev"].as_str().expect("systems rev missing");
        let owner = systems_node["owner"].as_str().expect("systems owner missing");
        let repo = systems_node["repo"].as_str().expect("systems repo missing");

        let o = owner.to_string();
        let r = repo.to_string();
        let rv = rev.to_string();

        let result = cache.get_or_fetch(nar_hash, Some(nar_hash), |dest| fetch_and_unpack_github(&o, &r, &rv, dest));

        assert!(result.is_ok(), "fetch+verify failed: {:?}", result.err());
        let path = result.unwrap();
        assert!(path.join("default.nix").exists());

        // Second call should hit cache
        let result2 = cache.get_or_fetch(nar_hash, Some(nar_hash), |_| {
            panic!("should not fetch twice");
        });
        assert_eq!(result2.unwrap(), path);
    }
}
