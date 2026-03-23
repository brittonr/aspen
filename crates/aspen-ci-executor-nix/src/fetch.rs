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
use wait_timeout::ChildExt;

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
// Git clone and extraction
// ---------------------------------------------------------------------------

/// Timeout for git clone operations in seconds.
const GIT_CLONE_TIMEOUT_SECS: u64 = 600;

/// Maximum recursion depth for git submodules.
const MAX_SUBMODULE_DEPTH: u32 = 10;

/// Characters forbidden in git ref/rev strings to prevent shell injection.
/// Covers shell metacharacters, backticks, pipes, etc.
const FORBIDDEN_REF_CHARS: &[char] = &[
    '`', '$', '|', ';', '&', '(', ')', '{', '}', '<', '>', '!', '\\', '\n', '\r', '\0', '\'', '"',
];

/// Metadata extracted from a git commit.
#[derive(Debug, Clone)]
pub struct GitMetadata {
    /// Full 40-char hex commit hash.
    pub rev: String,
    /// Commit timestamp (unix seconds).
    pub last_modified: u64,
    /// Number of ancestor commits (0 for shallow clones).
    pub rev_count: u64,
}

/// Validate a git ref or rev string for safety.
///
/// Rejects strings containing shell metacharacters that could be used
/// for command injection when passed to the git CLI.
fn validate_git_ref(value: &str, field_name: &str) -> io::Result<()> {
    if value.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("{field_name} must not be empty")));
    }
    if value.len() > MAX_URL_LENGTH {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{field_name} too long: {} chars", value.len()),
        ));
    }
    if let Some(bad) = value.chars().find(|c| FORBIDDEN_REF_CHARS.contains(c)) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{field_name} contains forbidden character: {bad:?}"),
        ));
    }
    Ok(())
}

/// Resolve a git ref and/or rev to a 40-char hex commit hash.
///
/// Runs `git rev-parse` in the given bare repo. When `rev` is provided,
/// validates it resolves in the repo. When only `ref` is given, resolves
/// the ref to its tip. Defaults to HEAD when neither is given.
pub fn resolve_git_rev(bare_repo: &Path, rev: Option<&str>, git_ref: Option<&str>) -> io::Result<String> {
    let target = match (rev, git_ref) {
        (Some(r), _) => {
            validate_git_ref(r, "rev")?;
            r.to_string()
        }
        (None, Some(r)) => {
            validate_git_ref(r, "ref")?;
            r.to_string()
        }
        (None, None) => "HEAD".to_string(),
    };

    let output = Command::new("git")
        .args(["rev-parse", "--verify", &target])
        .current_dir(bare_repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("failed to spawn git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("git rev-parse failed for {target}: {stderr}")));
    }

    let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if resolved.len() != 40 || !resolved.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("git rev-parse returned invalid hash: {resolved}"),
        ));
    }

    Ok(resolved)
}

/// Extract commit metadata from a bare git repo.
///
/// Runs `git log -1 --format='%H %ct'` for the commit hash and timestamp,
/// and `git rev-list --count` for the ancestor count (returns 0 for
/// shallow clones).
pub fn get_git_metadata(bare_repo: &Path, rev: &str) -> io::Result<GitMetadata> {
    validate_git_ref(rev, "rev")?;

    // Get commit hash and timestamp
    let output = Command::new("git")
        .args(["log", "-1", "--format=%H %ct", rev])
        .current_dir(bare_repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("failed to spawn git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("git log failed for {rev}: {stderr}")));
    }

    let line = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unexpected git log output: {line}")));
    }

    let commit_hash = parts[0].to_string();
    let last_modified: u64 = parts[1]
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("failed to parse commit timestamp: {e}")))?;

    // Get rev-count (0 for shallow clones)
    let count_output = Command::new("git")
        .args(["rev-list", "--count", rev])
        .current_dir(bare_repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let rev_count = match count_output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().unwrap_or(0),
        _ => 0,
    };

    Ok(GitMetadata {
        rev: commit_hash,
        last_modified,
        rev_count,
    })
}

/// Clone a git repository and extract a specific revision.
///
/// For non-submodule fetches, uses a bare clone + `git archive` for
/// minimal disk usage. For submodule fetches, uses a full clone with
/// `--recurse-submodules`.
///
/// Returns the path to the extracted checkout directory.
pub fn fetch_and_clone_git(
    url: &str,
    rev: Option<&str>,
    git_ref: Option<&str>,
    submodules: bool,
    dest_dir: &Path,
) -> io::Result<PathBuf> {
    if url.len() > MAX_URL_LENGTH {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("URL too long: {} chars exceeds {} limit", url.len(), MAX_URL_LENGTH),
        ));
    }
    if let Some(r) = rev {
        validate_git_ref(r, "rev")?;
    }
    if let Some(r) = git_ref {
        validate_git_ref(r, "ref")?;
    }

    info!(url = %url, rev = ?rev, git_ref = ?git_ref, submodules = submodules, "cloning git repository");

    if submodules {
        fetch_git_with_submodules(url, rev, git_ref, dest_dir)
    } else {
        fetch_git_bare(url, rev, git_ref, dest_dir)
    }
}

/// Bare clone + git archive extraction (no submodules).
fn fetch_git_bare(url: &str, rev: Option<&str>, git_ref: Option<&str>, dest_dir: &Path) -> io::Result<PathBuf> {
    let bare_dir = dest_dir.join("bare.git");
    std::fs::create_dir_all(&bare_dir)?;

    // Build clone args
    let mut clone_args = vec!["clone".to_string(), "--bare".to_string()];

    // Use --single-branch when we have a specific ref
    if let Some(r) = git_ref {
        clone_args.push("--single-branch".to_string());
        clone_args.push("--branch".to_string());
        // Strip refs/heads/ prefix if present — git clone --branch wants just the name
        let branch = r.strip_prefix("refs/heads/").unwrap_or(r);
        clone_args.push(branch.to_string());
    }

    clone_args.push(url.to_string());
    clone_args.push(bare_dir.to_string_lossy().to_string());

    run_git_with_timeout(&clone_args, None)?;

    // Resolve the target revision
    let resolved_rev = resolve_git_rev(&bare_dir, rev, git_ref)?;

    // Extract via git archive
    let checkout_dir = dest_dir.join("checkout");
    std::fs::create_dir_all(&checkout_dir)?;

    let archive_output = Command::new("git")
        .args(["archive", "--format=tar", &resolved_rev])
        .current_dir(&bare_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| io::Error::other(format!("failed to spawn git archive: {e}")))?;

    if !archive_output.status.success() {
        let stderr = String::from_utf8_lossy(&archive_output.stderr);
        return Err(io::Error::other(format!("git archive failed: {stderr}")));
    }

    // Check archive size
    let archive_size = archive_output.stdout.len() as u64;
    if archive_size > MAX_DOWNLOAD_SIZE {
        return Err(io::Error::other(format!(
            "git archive size {} exceeds maximum {} bytes",
            archive_size, MAX_DOWNLOAD_SIZE
        )));
    }

    // Unpack the tar archive
    let mut archive = tar::Archive::new(&archive_output.stdout[..]);
    archive.set_preserve_permissions(true);
    archive
        .unpack(&checkout_dir)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("failed to unpack git archive: {e}")))?;

    // Clean up bare repo to save disk
    let _ = std::fs::remove_dir_all(&bare_dir);

    info!(
        url = %url,
        rev = %resolved_rev,
        checkout = %checkout_dir.display(),
        "git checkout extracted"
    );
    Ok(checkout_dir)
}

/// Full clone with submodule support.
fn fetch_git_with_submodules(
    url: &str,
    rev: Option<&str>,
    git_ref: Option<&str>,
    dest_dir: &Path,
) -> io::Result<PathBuf> {
    let checkout_dir = dest_dir.join("checkout");
    std::fs::create_dir_all(&checkout_dir)?;

    let mut clone_args = vec![
        "clone".to_string(),
        "--recurse-submodules".to_string(),
        format!("--recurse-submodules-default=on-demand"),
        format!("--shallow-submodules"),
    ];

    if let Some(r) = git_ref {
        clone_args.push("--branch".to_string());
        let branch = r.strip_prefix("refs/heads/").unwrap_or(r);
        clone_args.push(branch.to_string());
    }

    clone_args.push(url.to_string());
    clone_args.push(checkout_dir.to_string_lossy().to_string());

    run_git_with_timeout(&clone_args, None)?;

    // Checkout the specific rev if provided
    if let Some(r) = rev {
        run_git_with_timeout(&["checkout".to_string(), r.to_string()], Some(&checkout_dir))?;
        // Re-init submodules at the checked-out rev
        run_git_with_timeout(
            &[
                "submodule".to_string(),
                "update".to_string(),
                "--init".to_string(),
                "--recursive".to_string(),
                "--depth=1".to_string(),
            ],
            Some(&checkout_dir),
        )?;
    }

    // Enforce submodule depth limit
    check_submodule_depth(&checkout_dir, 0)?;

    // Remove .git directories to match Nix's fetchGit behavior
    remove_git_dirs(&checkout_dir)?;

    info!(
        url = %url,
        checkout = %checkout_dir.display(),
        "git checkout with submodules extracted"
    );
    Ok(checkout_dir)
}

/// Run a git command with a timeout, returning an error on failure.
fn run_git_with_timeout(args: &[String], cwd: Option<&Path>) -> io::Result<()> {
    use std::time::Duration;

    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("failed to spawn git: {e}")))?;

    let timeout = Duration::from_secs(GIT_CLONE_TIMEOUT_SECS);
    match child.wait_timeout(timeout) {
        Ok(Some(status)) => {
            if status.success() {
                Ok(())
            } else {
                // Read stderr for error message
                let mut stderr_buf = String::new();
                if let Some(mut stderr) = child.stderr.take() {
                    let _ = stderr.read_to_string(&mut stderr_buf);
                }
                Err(io::Error::other(format!(
                    "git {} failed (exit {}): {}",
                    args.first().unwrap_or(&String::new()),
                    status.code().unwrap_or(-1),
                    stderr_buf.chars().take(500).collect::<String>()
                )))
            }
        }
        Ok(None) => {
            // Timed out — kill the process
            let _ = child.kill();
            let _ = child.wait();
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("git command timed out after {GIT_CLONE_TIMEOUT_SECS}s"),
            ))
        }
        Err(e) => Err(io::Error::other(format!("failed to wait for git process: {e}"))),
    }
}

/// Recursively check submodule nesting depth.
fn check_submodule_depth(dir: &Path, depth: u32) -> io::Result<()> {
    if depth > MAX_SUBMODULE_DEPTH {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("submodule depth {} exceeds maximum {MAX_SUBMODULE_DEPTH}", depth),
        ));
    }

    // Look for .gitmodules to detect submodules
    let gitmodules = dir.join(".gitmodules");
    if gitmodules.exists() {
        // Check each subdirectory that might be a submodule
        for entry in std::fs::read_dir(dir)?.filter_map(|e| e.ok()) {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let subdir = entry.path();
                // A submodule directory has a .git file (not directory) or .git directory
                if subdir.join(".git").exists() {
                    check_submodule_depth(&subdir, depth + 1)?;
                }
            }
        }
    }
    Ok(())
}

/// Remove .git directories recursively to produce a clean checkout.
fn remove_git_dirs(dir: &Path) -> io::Result<()> {
    let git_dir = dir.join(".git");
    if git_dir.exists() {
        if git_dir.is_dir() {
            std::fs::remove_dir_all(&git_dir)?;
        } else {
            // .git file (submodule pointer)
            std::fs::remove_file(&git_dir)?;
        }
    }

    let gitmodules = dir.join(".gitmodules");
    if gitmodules.exists() {
        std::fs::remove_file(&gitmodules)?;
    }

    // Recurse into subdirectories
    for entry in std::fs::read_dir(dir)?.filter_map(|e| e.ok()) {
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            remove_git_dirs(&entry.path())?;
        }
    }
    Ok(())
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

    // -----------------------------------------------------------------------
    // Git clone tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_git_ref_rejects_shell_metacharacters() {
        assert!(validate_git_ref("abc;rm -rf /", "rev").is_err());
        assert!(validate_git_ref("abc`whoami`", "rev").is_err());
        assert!(validate_git_ref("abc$(cmd)", "rev").is_err());
        assert!(validate_git_ref("abc|cat", "rev").is_err());
        assert!(validate_git_ref("abc&bg", "rev").is_err());
        assert!(validate_git_ref("abc\ninjection", "rev").is_err());
        assert!(validate_git_ref("abc\0null", "rev").is_err());
    }

    #[test]
    fn test_validate_git_ref_accepts_valid_refs() {
        assert!(validate_git_ref("abc123def456", "rev").is_ok());
        assert!(validate_git_ref("refs/heads/main", "ref").is_ok());
        assert!(validate_git_ref("refs/tags/v1.0.0", "ref").is_ok());
        assert!(validate_git_ref("feature/my-branch", "ref").is_ok());
        assert!(validate_git_ref("HEAD", "ref").is_ok());
    }

    #[test]
    fn test_validate_git_ref_rejects_empty() {
        assert!(validate_git_ref("", "rev").is_err());
    }

    #[test]
    fn test_validate_git_ref_rejects_too_long() {
        let long = "a".repeat(MAX_URL_LENGTH + 1);
        assert!(validate_git_ref(&long, "rev").is_err());
    }

    #[test]
    fn test_fetch_and_clone_git_rejects_long_url() {
        let tmpdir = tempfile::tempdir().unwrap();
        let long_url = "https://example.com/".to_string() + &"x".repeat(MAX_URL_LENGTH);
        let err = fetch_and_clone_git(&long_url, None, None, false, tmpdir.path()).unwrap_err();
        assert!(err.to_string().contains("URL too long"));
    }

    #[test]
    fn test_fetch_and_clone_git_rejects_bad_rev() {
        let tmpdir = tempfile::tempdir().unwrap();
        let err = fetch_and_clone_git("https://example.com/repo.git", Some("abc;rm -rf /"), None, false, tmpdir.path())
            .unwrap_err();
        assert!(err.to_string().contains("forbidden character"));
    }

    #[test]
    fn test_fetch_and_clone_git_rejects_bad_ref() {
        let tmpdir = tempfile::tempdir().unwrap();
        let err = fetch_and_clone_git(
            "https://example.com/repo.git",
            None,
            Some("refs/heads/`whoami`"),
            false,
            tmpdir.path(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("forbidden character"));
    }

    #[test]
    fn test_git_metadata_from_local_repo() {
        // Create a local git repo and verify metadata extraction
        let tmpdir = tempfile::tempdir().unwrap();
        let repo_dir = tmpdir.path().join("repo");
        std::fs::create_dir(&repo_dir).unwrap();

        // Init and commit
        Command::new("git").args(["init"]).current_dir(&repo_dir).output().unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&repo_dir)
            .output()
            .unwrap();
        Command::new("git").args(["config", "user.name", "Test"]).current_dir(&repo_dir).output().unwrap();
        std::fs::write(repo_dir.join("file.txt"), "hello").unwrap();
        Command::new("git").args(["add", "."]).current_dir(&repo_dir).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(&repo_dir).output().unwrap();

        let meta = get_git_metadata(&repo_dir, "HEAD").unwrap();
        assert_eq!(meta.rev.len(), 40);
        assert!(meta.rev.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(meta.last_modified > 0);
        assert!(meta.rev_count >= 1);
    }

    #[test]
    fn test_resolve_git_rev_from_local_repo() {
        let tmpdir = tempfile::tempdir().unwrap();
        let repo_dir = tmpdir.path().join("repo");
        std::fs::create_dir(&repo_dir).unwrap();

        Command::new("git").args(["init"]).current_dir(&repo_dir).output().unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&repo_dir)
            .output()
            .unwrap();
        Command::new("git").args(["config", "user.name", "Test"]).current_dir(&repo_dir).output().unwrap();
        std::fs::write(repo_dir.join("file.txt"), "hello").unwrap();
        Command::new("git").args(["add", "."]).current_dir(&repo_dir).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(&repo_dir).output().unwrap();

        // Resolve HEAD
        let rev = resolve_git_rev(&repo_dir, None, None).unwrap();
        assert_eq!(rev.len(), 40);

        // Resolve by explicit rev
        let rev2 = resolve_git_rev(&repo_dir, Some(&rev), None).unwrap();
        assert_eq!(rev, rev2);

        // Resolve nonexistent ref
        let err = resolve_git_rev(&repo_dir, None, Some("refs/heads/nonexistent-branch-xyz"));
        assert!(err.is_err(), "should fail for nonexistent ref");
    }

    #[test]
    fn test_fetch_and_clone_git_local_repo() {
        let tmpdir = tempfile::tempdir().unwrap();

        // Create source repo
        let src = tmpdir.path().join("src");
        std::fs::create_dir(&src).unwrap();
        Command::new("git").args(["init"]).current_dir(&src).output().unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&src)
            .output()
            .unwrap();
        Command::new("git").args(["config", "user.name", "Test"]).current_dir(&src).output().unwrap();
        std::fs::write(src.join("hello.txt"), "hello from git").unwrap();
        Command::new("git").args(["add", "."]).current_dir(&src).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(&src).output().unwrap();

        // Clone into dest
        let dest = tmpdir.path().join("dest");
        std::fs::create_dir(&dest).unwrap();

        let src_url = src.to_string_lossy().to_string();
        let result = fetch_and_clone_git(&src_url, None, None, false, &dest);
        assert!(result.is_ok(), "clone failed: {:?}", result.err());

        let checkout = result.unwrap();
        assert!(checkout.join("hello.txt").exists());
        let content = std::fs::read_to_string(checkout.join("hello.txt")).unwrap();
        assert_eq!(content, "hello from git");

        // .git should be removed (bare clone + archive)
        assert!(!checkout.join(".git").exists());
    }

    /// Integration test: clone nix-systems/default via git URL.
    #[test]
    #[ignore] // requires network access
    fn test_fetch_and_clone_git_remote() {
        let tmpdir = tempfile::tempdir().unwrap();
        let dest = tmpdir.path().join("nix-systems");
        std::fs::create_dir(&dest).unwrap();

        let result = fetch_and_clone_git(
            "https://github.com/nix-systems/default.git",
            Some("da67096a3b9bf56a91d16901293e51ba5b49a27e"),
            None,
            false,
            &dest,
        );

        assert!(result.is_ok(), "clone failed: {:?}", result.err());
        let checkout = result.unwrap();
        assert!(checkout.join("default.nix").exists(), "missing default.nix in {:?}", checkout,);
    }
}
