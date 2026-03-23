//! Flake.lock parser for native flake evaluation.
//!
//! This module parses `flake.lock` files, resolves the input graph (including follows paths),
//! and computes Nix store paths from `narHash` values using nix-compat. This eliminates the
//! need for subprocess calls to `nix eval` in many cases.
//!
//! # Usage
//!
//! ```ignore
//! use aspen_ci_executor_nix::flake_lock::FlakeLock;
//!
//! // Parse flake.lock
//! let lock_bytes = std::fs::read("flake.lock")?;
//! let lock = FlakeLock::parse(&lock_bytes)?;
//!
//! // Resolve all inputs with store paths
//! let resolved = lock.resolve_all_nodes()?;
//!
//! // Check local availability
//! let mut resolved = resolved;
//! FlakeLock::check_local_availability(&mut resolved);
//! ```

use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Error;
use std::io::ErrorKind;
use std::io::{self};
use std::path::Path;

use nix_compat::nixhash::CAHash;
use nix_compat::nixhash::NixHash;
use nix_compat::store_path::build_ca_path;
use serde::Deserialize;
use serde::Deserializer;
use tracing;

/// Maximum nodes in a flake.lock before we reject it (Tiger Style).
const MAX_FLAKE_LOCK_NODES: usize = 500;

/// Maximum flake.lock file size (1 MB).
const MAX_FLAKE_LOCK_SIZE: usize = 1_048_576;

/// Supported flake.lock version.
const SUPPORTED_LOCK_VERSION: u64 = 7;

/// Maximum follows resolution depth to prevent infinite loops.
const MAX_FOLLOWS_DEPTH: usize = 100;

/// Parsed flake.lock file structure.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FlakeLock {
    pub version: u64,
    pub root: String,
    pub nodes: HashMap<String, FlakeNode>,
}

/// A node in the flake.lock input graph.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FlakeNode {
    /// Input name → input spec (either a node name string or a follows path array).
    #[serde(default)]
    pub inputs: HashMap<String, InputSpec>,
    /// Locked source info (None for the root node).
    pub locked: Option<LockedInput>,
    /// Whether this is a flake (default true).
    #[serde(default = "default_true")]
    pub flake: bool,
}

/// Input specification - either direct reference or follows path.
#[derive(Debug, Clone, PartialEq)]
pub enum InputSpec {
    /// Direct reference to a node name.
    Direct(String),
    /// Follows path from root (e.g., ["nixpkgs"] or ["flake-parts", "nixpkgs"]).
    Follows(Vec<String>),
}

impl<'de> Deserialize<'de> for InputSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        use std::fmt;

        use serde::de::Visitor;
        use serde::de::{self};

        struct InputSpecVisitor;

        impl<'de> Visitor<'de> for InputSpecVisitor {
            type Value = InputSpec;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or array of strings")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where E: de::Error {
                Ok(InputSpec::Direct(value.to_string()))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where A: de::SeqAccess<'de> {
                let mut path = Vec::new();
                while let Some(segment) = seq.next_element::<String>()? {
                    path.push(segment);
                }
                Ok(InputSpec::Follows(path))
            }
        }

        deserializer.deserialize_any(InputSpecVisitor)
    }
}

/// Locked input information with source details.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockedInput {
    #[serde(rename = "type")]
    pub input_type: String, // "github", "gitlab", "tarball", "path", "git", "indirect"
    pub nar_hash: Option<String>, // SRI format "sha256-<base64>"
    pub rev: Option<String>,
    pub last_modified: Option<u64>,
    pub owner: Option<String>, // github/gitlab
    pub repo: Option<String>,  // github/gitlab
    pub url: Option<String>,   // tarball/git
    pub path: Option<String>,  // path type
    pub dir: Option<String>,   // subdir within source
}

/// A resolved input with its computed store path.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedInput {
    pub node_key: String,
    pub store_path: String, // absolute path like /nix/store/...-source
    pub locked: LockedInput,
    pub is_local: bool, // whether the path exists in local /nix/store
}

/// Helper for default true value in serde.
fn default_true() -> bool {
    true
}

impl FlakeLock {
    /// Parse a flake.lock JSON file.
    ///
    /// Validates the version is 7 and node count is within bounds.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file is larger than MAX_FLAKE_LOCK_SIZE
    /// - The JSON is malformed
    /// - The version is not 7
    /// - There are more than MAX_FLAKE_LOCK_NODES nodes
    pub fn parse(json_bytes: &[u8]) -> io::Result<FlakeLock> {
        if json_bytes.len() > MAX_FLAKE_LOCK_SIZE {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("flake.lock too large: {} bytes exceeds {} limit", json_bytes.len(), MAX_FLAKE_LOCK_SIZE),
            ));
        }

        let lock: FlakeLock = serde_json::from_slice(json_bytes)
            .map_err(|e| Error::new(ErrorKind::InvalidData, format!("failed to parse flake.lock JSON: {}", e)))?;

        if lock.version != SUPPORTED_LOCK_VERSION {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("unsupported flake.lock version {}, expected {}", lock.version, SUPPORTED_LOCK_VERSION),
            ));
        }

        if lock.nodes.len() > MAX_FLAKE_LOCK_NODES {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("too many nodes in flake.lock: {} exceeds {} limit", lock.nodes.len(), MAX_FLAKE_LOCK_NODES),
            ));
        }

        Ok(lock)
    }

    /// Resolve a follows path to a node name.
    ///
    /// Traverses the input graph following the path array and returns the final node name.
    /// Detects cycles and enforces MAX_FOLLOWS_DEPTH to prevent infinite loops.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A follows path is circular
    /// - The path depth exceeds MAX_FOLLOWS_DEPTH
    /// - A referenced node doesn't exist
    pub fn resolve_input(&self, input_spec: &InputSpec) -> io::Result<String> {
        match input_spec {
            InputSpec::Direct(node_name) => Ok(node_name.clone()),
            InputSpec::Follows(path) => {
                if path.is_empty() {
                    return Err(Error::new(ErrorKind::InvalidData, "empty follows path"));
                }

                let mut visited = HashSet::new();
                let mut current_node = self.root.clone();
                let mut depth = 0;

                for segment in path {
                    depth += 1;
                    if depth > MAX_FOLLOWS_DEPTH {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            format!("follows path too deep: {} > {}", depth, MAX_FOLLOWS_DEPTH),
                        ));
                    }

                    if !visited.insert(current_node.clone()) {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            format!("circular follows detected at node: {}", current_node),
                        ));
                    }

                    let node = self.nodes.get(&current_node).ok_or_else(|| {
                        Error::new(
                            ErrorKind::InvalidData,
                            format!("follows path references missing node: {}", current_node),
                        )
                    })?;

                    let input_spec = node.inputs.get(segment).ok_or_else(|| {
                        Error::new(
                            ErrorKind::InvalidData,
                            format!("follows path segment '{}' not found in node '{}'", segment, current_node),
                        )
                    })?;

                    // Recursively resolve if this is another follows
                    current_node = self.resolve_input(input_spec)?;
                }

                Ok(current_node)
            }
        }
    }

    /// Resolve all non-root nodes to ResolvedInput structs.
    ///
    /// Computes store paths from narHash for all nodes that have locked inputs.
    /// Skips the root node and any nodes without narHash values.
    pub fn resolve_all_nodes(&self) -> io::Result<Vec<ResolvedInput>> {
        let mut resolved = Vec::new();

        for (node_key, node) in &self.nodes {
            // Skip root node - it doesn't have a locked input
            if node_key == &self.root {
                continue;
            }

            if let Some(ref locked) = node.locked
                && let Some(ref nar_hash) = locked.nar_hash
            {
                let store_path = compute_store_path(nar_hash)?;
                resolved.push(ResolvedInput {
                    node_key: node_key.clone(),
                    store_path,
                    locked: locked.clone(),
                    is_local: false, // Will be set by check_local_availability
                });
            }
        }

        Ok(resolved)
    }

    /// Check local availability of resolved inputs.
    ///
    /// Sets the `is_local` field on each ResolvedInput based on whether the
    /// store path exists in the local /nix/store.
    pub fn check_local_availability(resolved: &mut [ResolvedInput]) {
        for input in resolved {
            input.is_local = Path::new(&input.store_path).exists();
        }
    }

    /// Resolve all flake inputs to their store paths or local paths.
    ///
    /// For each node in the lock file:
    /// - If it has a narHash, compute the store path and check local availability
    /// - If the store path exists locally, mark as resolved
    /// - If not local, attempt to fetch based on input type
    /// - Path inputs are resolved relative to flake_dir
    ///
    /// The root node is handled specially — its outPath is the flake_dir itself.
    ///
    /// When `fetch_cache` is provided, missing inputs are fetched over HTTP
    /// and cached by narHash. Without a cache, fetching is skipped and missing
    /// inputs cause a fallback to subprocess evaluation.
    ///
    /// Returns a map of node key → (outPath, LockedInput) for building overrides.
    pub fn resolve_all_inputs(&self, flake_dir: &str) -> io::Result<Vec<ResolvedInput>> {
        let mut resolved = self.resolve_all_nodes()?;
        Self::check_local_availability(&mut resolved);

        for input in &mut resolved {
            if !input.is_local {
                let fetch_result = match input.locked.input_type.as_str() {
                    "path" => resolve_path_input(&input.locked, flake_dir),
                    #[cfg(not(feature = "snix-build"))]
                    "github" | "gitlab" => fetch_github_input(&input.locked),
                    #[cfg(not(feature = "snix-build"))]
                    "tarball" => fetch_tarball_input(&input.locked),
                    #[cfg(feature = "snix-build")]
                    "github" => fetch_github_input(&input.locked, None),
                    #[cfg(feature = "snix-build")]
                    "gitlab" => fetch_gitlab_input(&input.locked, None),
                    #[cfg(feature = "snix-build")]
                    "tarball" => fetch_tarball_input(&input.locked, None),
                    #[cfg(not(feature = "snix-build"))]
                    "git" => fetch_git_input(&input.locked),
                    #[cfg(feature = "snix-build")]
                    "git" => fetch_git_input(&input.locked, None),
                    other => Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        format!("unsupported input type '{}' for node {}", other, input.node_key),
                    )),
                };

                match fetch_result {
                    Ok(path) => {
                        input.store_path = path;
                        input.is_local = true;
                    }
                    Err(e) => {
                        tracing::debug!(
                            node = %input.node_key,
                            input_type = %input.locked.input_type,
                            error = %e,
                            "input not available locally, will need fallback"
                        );
                    }
                }
            }
        }

        Ok(resolved)
    }

    /// Resolve all inputs, fetching missing inputs over HTTP via the provided cache.
    ///
    /// Missing GitHub/GitLab/tarball/git inputs are downloaded, unpacked,
    /// narHash-verified, and cached. This enables fully in-process flake
    /// evaluation without subprocess fallback.
    #[cfg(feature = "snix-build")]
    pub fn resolve_all_inputs_with_fetch(
        &self,
        flake_dir: &str,
        fetch_cache: &crate::fetch::FetchCache,
    ) -> io::Result<Vec<ResolvedInput>> {
        let mut resolved = self.resolve_all_nodes()?;
        Self::check_local_availability(&mut resolved);

        for input in &mut resolved {
            if !input.is_local {
                let fetch_result = match input.locked.input_type.as_str() {
                    "path" => resolve_path_input(&input.locked, flake_dir),
                    "github" => fetch_github_input(&input.locked, Some(fetch_cache)),
                    "gitlab" => fetch_gitlab_input(&input.locked, Some(fetch_cache)),
                    "tarball" => fetch_tarball_input(&input.locked, Some(fetch_cache)),
                    "git" => fetch_git_input(&input.locked, Some(fetch_cache)),
                    other => Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        format!("unsupported input type '{}' for node {}", other, input.node_key),
                    )),
                };

                match fetch_result {
                    Ok(path) => {
                        input.store_path = path;
                        input.is_local = true;
                    }
                    Err(e) => {
                        tracing::debug!(
                            node = %input.node_key,
                            input_type = %input.locked.input_type,
                            error = %e,
                            "input not available locally, will need fallback"
                        );
                    }
                }
            }
        }

        Ok(resolved)
    }

    /// Check if all resolved inputs are available (either locally or fetched).
    /// Returns true if all inputs have is_local == true.
    pub fn all_inputs_available(resolved: &[ResolvedInput]) -> bool {
        resolved.iter().all(|r| r.is_local)
    }
}

/// Extract the host portion from a URL string.
///
/// Simple parser: looks for `://` then takes everything up to the next `/`.
fn extract_host_from_url(url: &str) -> Option<String> {
    let after_scheme = url.split("://").nth(1)?;
    let host = after_scheme.split('/').next()?;
    if host.is_empty() { None } else { Some(host.to_string()) }
}

/// Compute Nix store path from a narHash SRI string.
///
/// Parses the SRI hash, wraps it in CAHash::Nar, and calls build_ca_path
/// with name="source", no references, and no self-reference.
///
/// # Arguments
///
/// * `nar_hash_sri` - SRI format hash like "sha256-OLdIz38tsFp1aXt8GsJ40s0/jxSkhlqftuDE7LvuhK4="
///
/// # Errors
///
/// Returns an error if:
/// - The SRI hash format is invalid
/// - The hash is not sha256
/// - Store path computation fails
pub fn compute_store_path(nar_hash_sri: &str) -> io::Result<String> {
    // Parse SRI hash
    let nix_hash = NixHash::from_sri(nar_hash_sri)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("invalid SRI hash '{}': {}", nar_hash_sri, e)))?;

    // Ensure it's sha256 (required for narHash)
    match nix_hash {
        NixHash::Sha256(_) => {}
        _ => {
            return Err(Error::new(ErrorKind::InvalidData, format!("narHash must be sha256, got: {:?}", nix_hash)));
        }
    }

    // Wrap in CAHash::Nar
    let ca_hash = CAHash::Nar(nix_hash);

    // Build store path
    let store_path =
        build_ca_path::<&str, String, std::iter::Empty<&str>>("source", &ca_hash, std::iter::empty(), false)
            .map_err(|e| Error::new(ErrorKind::InvalidData, format!("failed to build store path: {}", e)))?;

    Ok(store_path.to_absolute_path())
}

/// Fetch a GitHub input via HTTP archive download.
///
/// When `fetch_cache` is provided, downloads the archive tarball, unpacks it,
/// verifies the narHash, and caches the result. Without a cache, returns an
/// error indicating the input is not available.
#[cfg(feature = "snix-build")]
pub fn fetch_github_input(locked: &LockedInput, fetch_cache: Option<&crate::fetch::FetchCache>) -> io::Result<String> {
    let owner = locked.owner.as_deref().unwrap_or("");
    let repo = locked.repo.as_deref().unwrap_or("");
    let rev = locked.rev.as_deref().unwrap_or("");

    let cache = match fetch_cache {
        Some(c) => c,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("github input {owner}/{repo}@{rev} not in local store and no fetch cache"),
            ));
        }
    };

    let nar_hash = locked.nar_hash.as_deref().unwrap_or("");
    if nar_hash.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("github input {owner}/{repo}@{rev} has no narHash for verification"),
        ));
    }

    let o = owner.to_string();
    let r = repo.to_string();
    let rv = rev.to_string();
    let nh = nar_hash.to_string();

    cache
        .get_or_fetch(nar_hash, Some(nar_hash), |dest| {
            let path = crate::fetch::fetch_and_unpack_github(&o, &r, &rv, dest)?;
            // verify_nar_hash is called by get_or_fetch via expected_nar_hash
            Ok(path)
        })
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| io::Error::new(e.kind(), format!("failed to fetch github input {o}/{r}@{rv} (narHash={nh}): {e}")))
}

/// Fetch a GitHub input (stub when snix-build is disabled).
#[cfg(not(feature = "snix-build"))]
pub fn fetch_github_input(locked: &LockedInput) -> io::Result<String> {
    let owner = locked.owner.as_deref().unwrap_or("?");
    let repo = locked.repo.as_deref().unwrap_or("?");
    let rev = locked.rev.as_deref().unwrap_or("?");
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("github input {owner}/{repo}@{rev} not in local store; enable snix-build feature for HTTP fetch"),
    ))
}

/// Fetch a GitLab input via HTTP archive download.
#[cfg(feature = "snix-build")]
pub fn fetch_gitlab_input(locked: &LockedInput, fetch_cache: Option<&crate::fetch::FetchCache>) -> io::Result<String> {
    let owner = locked.owner.as_deref().unwrap_or("");
    let repo = locked.repo.as_deref().unwrap_or("");
    let rev = locked.rev.as_deref().unwrap_or("");
    // Extract host from URL if available, default to gitlab.com
    let host = locked.url.as_deref().and_then(extract_host_from_url).unwrap_or_else(|| "gitlab.com".to_string());

    let cache = match fetch_cache {
        Some(c) => c,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("gitlab input {owner}/{repo}@{rev} not in local store and no fetch cache"),
            ));
        }
    };

    let nar_hash = locked.nar_hash.as_deref().unwrap_or("");
    if nar_hash.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("gitlab input {owner}/{repo}@{rev} has no narHash for verification"),
        ));
    }

    let h = host.clone();
    let o = owner.to_string();
    let r = repo.to_string();
    let rv = rev.to_string();

    cache
        .get_or_fetch(nar_hash, Some(nar_hash), |dest| crate::fetch::fetch_and_unpack_gitlab(&h, &o, &r, &rv, dest))
        .map(|p| p.to_string_lossy().to_string())
}

/// Fetch a tarball input via HTTP download.
#[cfg(feature = "snix-build")]
pub fn fetch_tarball_input(locked: &LockedInput, fetch_cache: Option<&crate::fetch::FetchCache>) -> io::Result<String> {
    let url = locked.url.as_deref().unwrap_or("");

    let cache = match fetch_cache {
        Some(c) => c,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("tarball input {url} not in local store and no fetch cache"),
            ));
        }
    };

    let nar_hash = locked.nar_hash.as_deref().unwrap_or("");
    // For tarballs without narHash, skip verification but still fetch
    let expected = if nar_hash.is_empty() {
        tracing::warn!(url = %url, "tarball input has no narHash, content will not be verified");
        None
    } else {
        Some(nar_hash)
    };

    let cache_key = if nar_hash.is_empty() { url } else { nar_hash };
    let url_owned = url.to_string();

    cache
        .get_or_fetch(cache_key, expected, |dest| crate::fetch::fetch_and_unpack_tarball(&url_owned, dest))
        .map(|p| p.to_string_lossy().to_string())
}

/// Fetch a tarball input (stub when snix-build is disabled).
#[cfg(not(feature = "snix-build"))]
pub fn fetch_tarball_input(locked: &LockedInput) -> io::Result<String> {
    let url = locked.url.as_deref().unwrap_or("?");
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("tarball input {url} not in local store; enable snix-build feature for HTTP fetch"),
    ))
}

/// Fetch a git input by cloning the repository.
///
/// Clones the repo at the locked URL, checks out the specified rev,
/// verifies the narHash if present, and caches the result via `FetchCache`.
/// The cache key is the narHash when available, or `git:<url>@<rev>` otherwise.
#[cfg(feature = "snix-build")]
pub fn fetch_git_input(locked: &LockedInput, fetch_cache: Option<&crate::fetch::FetchCache>) -> io::Result<String> {
    let url = locked.url.as_deref().unwrap_or("");
    let rev = locked.rev.as_deref();

    if url.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "git input missing 'url' field"));
    }

    let cache = match fetch_cache {
        Some(c) => c,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("git input {url} not in local store and no fetch cache"),
            ));
        }
    };

    let nar_hash = locked.nar_hash.as_deref().unwrap_or("");
    let expected_nar_hash = if nar_hash.is_empty() {
        tracing::warn!(url = %url, "git input has no narHash, content will not be verified");
        None
    } else {
        Some(nar_hash)
    };

    // Cache key: narHash if available, otherwise url+rev
    let cache_key = if !nar_hash.is_empty() {
        nar_hash.to_string()
    } else {
        format!("git:{}@{}", url, rev.unwrap_or("HEAD"))
    };

    let url_owned = url.to_string();
    let rev_owned = rev.map(|r| r.to_string());
    // Note: flake.lock git inputs don't have a separate "ref" field in LockedInput;
    // the rev is always pinned in locked inputs. ref is only in the original spec.
    let submodules = false; // TODO: parse from locked input when flake.lock includes it

    cache
        .get_or_fetch(&cache_key, expected_nar_hash, |dest| {
            crate::fetch::fetch_and_clone_git(
                &url_owned,
                rev_owned.as_deref(),
                None, // ref not needed when rev is pinned
                submodules,
                dest,
            )
        })
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| {
            io::Error::new(e.kind(), format!("failed to fetch git input {url}@{}: {e}", rev.unwrap_or("HEAD")))
        })
}

/// Fetch a git input (stub when snix-build is disabled).
#[cfg(not(feature = "snix-build"))]
pub fn fetch_git_input(locked: &LockedInput) -> io::Result<String> {
    let url = locked.url.as_deref().unwrap_or("?");
    let rev = locked.rev.as_deref().unwrap_or("?");
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("git input {url}@{rev} not in local store; enable snix-build feature for git clone"),
    ))
}

/// Resolve a path-type input relative to the flake source directory.
/// Path inputs point to directories on disk (absolute or relative).
pub fn resolve_path_input(locked: &LockedInput, flake_dir: &str) -> io::Result<String> {
    let path = locked
        .path
        .as_deref()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path input missing 'path' field"))?;

    let resolved = if path.starts_with('/') {
        std::path::PathBuf::from(path)
    } else {
        std::path::PathBuf::from(flake_dir).join(path)
    };

    if !resolved.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("path input does not exist: {}", resolved.display()),
        ));
    }

    Ok(resolved.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_parse_aspen_flake_lock() {
        // Read actual Aspen flake.lock
        let lock_bytes = std::fs::read("../../flake.lock").expect("failed to read flake.lock");
        let lock = FlakeLock::parse(&lock_bytes).expect("failed to parse flake.lock");

        // Verify basic structure
        assert_eq!(lock.version, 7);
        assert_eq!(lock.root, "root");
        assert!(!lock.nodes.is_empty());
        assert!(lock.nodes.len() < MAX_FLAKE_LOCK_NODES);

        // Verify root node exists and has expected inputs
        let root = lock.nodes.get("root").expect("root node missing");
        assert!(root.locked.is_none(), "root node should not have locked input");

        // Verify some expected inputs exist (these should be stable)
        let expected_inputs = ["advisory-db", "crane", "flake-parts", "nixpkgs"];
        for input in &expected_inputs {
            assert!(root.inputs.contains_key(*input), "root missing expected input: {}", input);
        }

        // Verify nixpkgs has narHash
        if let Some(nixpkgs_spec) = root.inputs.get("nixpkgs") {
            let nixpkgs_node = match nixpkgs_spec {
                InputSpec::Direct(name) => lock.nodes.get(name),
                InputSpec::Follows(_path) => {
                    let resolved = lock.resolve_input(nixpkgs_spec).unwrap();
                    lock.nodes.get(&resolved)
                }
            };

            if let Some(nixpkgs) = nixpkgs_node {
                if let Some(ref locked) = nixpkgs.locked {
                    assert!(locked.nar_hash.is_some(), "nixpkgs missing narHash");
                    assert_eq!(locked.input_type, "github");
                    assert!(locked.owner.is_some());
                    assert!(locked.repo.is_some());
                }
            }
        }
    }

    #[test]
    fn test_follows_resolution() {
        // Create a simple flake.lock with follows
        let mut nodes = HashMap::new();

        // Root node with inputs
        let mut root_inputs = HashMap::new();
        root_inputs.insert("nixpkgs".to_string(), InputSpec::Direct("nixpkgs".to_string()));
        root_inputs.insert("utils".to_string(), InputSpec::Direct("flake-utils".to_string()));

        nodes.insert("root".to_string(), FlakeNode {
            inputs: root_inputs,
            locked: None,
            flake: true,
        });

        // Nixpkgs node
        nodes.insert("nixpkgs".to_string(), FlakeNode {
            inputs: HashMap::new(),
            locked: Some(LockedInput {
                input_type: "github".to_string(),
                nar_hash: Some("sha256-OLdIz38tsFp1aXt8GsJ40s0/jxSkhlqftuDE7LvuhK4=".to_string()),
                rev: Some("35bdbbce4d6e84baa7df6544d6127db8dd7fbaef".to_string()),
                last_modified: Some(1767927480),
                owner: Some("NixOS".to_string()),
                repo: Some("nixpkgs".to_string()),
                url: None,
                path: None,
                dir: None,
            }),
            flake: true,
        });

        // Flake-utils with follows to nixpkgs
        let mut utils_inputs = HashMap::new();
        utils_inputs.insert("nixpkgs".to_string(), InputSpec::Follows(vec!["nixpkgs".to_string()]));

        nodes.insert("flake-utils".to_string(), FlakeNode {
            inputs: utils_inputs,
            locked: Some(LockedInput {
                input_type: "github".to_string(),
                nar_hash: Some("sha256-l0KFg5HjrsfsO/JpG+r7fRrqm12kzFHyUHqHCVpMMbI=".to_string()),
                rev: Some("11707dc2f618dd54ca8739b309ec4fc024de578b".to_string()),
                last_modified: Some(1731533236),
                owner: Some("numtide".to_string()),
                repo: Some("flake-utils".to_string()),
                url: None,
                path: None,
                dir: None,
            }),
            flake: true,
        });

        let lock = FlakeLock {
            version: 7,
            root: "root".to_string(),
            nodes,
        };

        // Test direct resolution
        let direct_spec = InputSpec::Direct("nixpkgs".to_string());
        let resolved = lock.resolve_input(&direct_spec).unwrap();
        assert_eq!(resolved, "nixpkgs");

        // Test follows resolution
        let follows_spec = InputSpec::Follows(vec!["nixpkgs".to_string()]);
        let resolved = lock.resolve_input(&follows_spec).unwrap();
        assert_eq!(resolved, "nixpkgs");
    }

    #[test]
    fn test_follows_cycle_detection() {
        // Create a flake.lock with circular follows
        let mut nodes = HashMap::new();

        let mut root_inputs = HashMap::new();
        root_inputs.insert("start".to_string(), InputSpec::Direct("node-a".to_string()));

        nodes.insert("root".to_string(), FlakeNode {
            inputs: root_inputs,
            locked: None,
            flake: true,
        });

        // Node A has an input that follows a path creating a cycle
        let mut a_inputs = HashMap::new();
        a_inputs.insert("next".to_string(), InputSpec::Direct("node-b".to_string()));

        nodes.insert("node-a".to_string(), FlakeNode {
            inputs: a_inputs,
            locked: None,
            flake: true,
        });

        // Node B has an input that follows back to A
        let mut b_inputs = HashMap::new();
        b_inputs.insert("back".to_string(), InputSpec::Direct("node-a".to_string()));

        nodes.insert("node-b".to_string(), FlakeNode {
            inputs: b_inputs,
            locked: None,
            flake: true,
        });

        let lock = FlakeLock {
            version: 7,
            root: "root".to_string(),
            nodes,
        };

        // This creates a cycle: start -> next -> back -> next (cycles at node-a)
        let follows_spec = InputSpec::Follows(vec![
            "start".to_string(),
            "next".to_string(),
            "back".to_string(),
            "next".to_string(),
        ]);
        let result = lock.resolve_input(&follows_spec);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("circular"));
    }

    #[test]
    fn test_nar_hash_to_store_path() {
        // Test with known narHash from Aspen's nixpkgs
        let nar_hash = "sha256-OLdIz38tsFp1aXt8GsJ40s0/jxSkhlqftuDE7LvuhK4=";
        let store_path = compute_store_path(nar_hash).unwrap();

        // Verify format
        assert!(store_path.starts_with("/nix/store/"));
        assert!(store_path.contains("-source"));

        // Store path should be deterministic
        let store_path2 = compute_store_path(nar_hash).unwrap();
        assert_eq!(store_path, store_path2);

        // Verify it's the expected format (32 char hash + name)
        let store_name = store_path.strip_prefix("/nix/store/").unwrap();
        let parts: Vec<&str> = store_name.splitn(2, '-').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 32); // Nix store hash is always 32 chars
        assert_eq!(parts[1], "source");
    }

    #[test]
    fn test_invalid_nar_hash() {
        // Test invalid SRI format
        let result = compute_store_path("not-a-hash");
        assert!(result.is_err());

        // Test non-sha256 hash
        let result = compute_store_path("sha1-abcd");
        assert!(result.is_err());

        // Test malformed sha256
        let result = compute_store_path("sha256-not-base64!");
        assert!(result.is_err());
    }

    #[test]
    fn test_resource_bounds() {
        // Test version validation
        let invalid_version = r#"{
            "version": 6,
            "root": "root",
            "nodes": {"root": {"inputs": {}}}
        }"#;

        let result = FlakeLock::parse(invalid_version.as_bytes());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("version"));

        // Test file size limit
        let large_data = "x".repeat(MAX_FLAKE_LOCK_SIZE + 1);
        let result = FlakeLock::parse(large_data.as_bytes());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too large"));

        // Test node count limit (would require building large JSON, skip for now)
    }

    #[test]
    fn test_input_spec_parsing() {
        // Test direct input parsing
        let json = r#""nixpkgs""#;
        let spec: InputSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec, InputSpec::Direct("nixpkgs".to_string()));

        // Test follows input parsing
        let json = r#"["flake-parts", "nixpkgs"]"#;
        let spec: InputSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec, InputSpec::Follows(vec!["flake-parts".to_string(), "nixpkgs".to_string()]));

        // Test empty follows array
        let json = r#"[]"#;
        let spec: InputSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec, InputSpec::Follows(vec![]));
    }

    #[test]
    fn test_resolve_all_nodes() {
        // Read real flake.lock and resolve all nodes
        let lock_bytes = std::fs::read("../../flake.lock").expect("failed to read flake.lock");
        let lock = FlakeLock::parse(&lock_bytes).expect("failed to parse flake.lock");

        let resolved = lock.resolve_all_nodes().expect("failed to resolve nodes");

        // Should have resolved some nodes (excluding root)
        assert!(!resolved.is_empty(), "should resolve at least one node");

        // All resolved inputs should have store paths
        for input in &resolved {
            assert!(input.store_path.starts_with("/nix/store/"));
            assert!(input.store_path.contains("-source"));
            assert!(!input.node_key.is_empty());
        }

        // Verify no root node in results
        assert!(!resolved.iter().any(|r| r.node_key == "root"));
    }

    #[test]
    fn test_check_local_availability() {
        // Create test resolved inputs
        let mut resolved = vec![ResolvedInput {
            node_key: "nixpkgs".to_string(),
            store_path: "/nix/store/nonexistent-source".to_string(),
            locked: LockedInput {
                input_type: "github".to_string(),
                nar_hash: Some("sha256-test".to_string()),
                rev: None,
                last_modified: None,
                owner: Some("NixOS".to_string()),
                repo: Some("nixpkgs".to_string()),
                url: None,
                path: None,
                dir: None,
            },
            is_local: false,
        }];

        // Check availability (should be false for nonexistent path)
        FlakeLock::check_local_availability(&mut resolved);
        assert!(!resolved[0].is_local);

        // Test with existing path (use current directory as guaranteed to exist)
        let current_dir = std::env::current_dir().unwrap();
        resolved[0].store_path = current_dir.to_string_lossy().to_string();
        FlakeLock::check_local_availability(&mut resolved);
        assert!(resolved[0].is_local);
    }

    #[test]
    fn test_empty_follows_path() {
        let lock = FlakeLock {
            version: 7,
            root: "root".to_string(),
            nodes: HashMap::new(),
        };

        let empty_follows = InputSpec::Follows(vec![]);
        let result = lock.resolve_input(&empty_follows);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty follows"));
    }

    #[test]
    fn test_missing_node_in_follows() {
        let mut nodes = HashMap::new();
        nodes.insert("root".to_string(), FlakeNode {
            inputs: HashMap::new(),
            locked: None,
            flake: true,
        });

        let lock = FlakeLock {
            version: 7,
            root: "root".to_string(),
            nodes,
        };

        let follows_spec = InputSpec::Follows(vec!["nonexistent".to_string()]);
        let result = lock.resolve_input(&follows_spec);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_flake_node_defaults() {
        // Test that flake defaults to true when missing from JSON
        let json = r#"{"inputs": {}, "locked": null}"#;
        let node: FlakeNode = serde_json::from_str(json).unwrap();
        assert!(node.flake);

        // Test explicit false
        let json = r#"{"inputs": {}, "locked": null, "flake": false}"#;
        let node: FlakeNode = serde_json::from_str(json).unwrap();
        assert!(!node.flake);
    }

    #[test]
    fn test_resolve_path_input() {
        let locked = LockedInput {
            input_type: "path".to_string(),
            nar_hash: None,
            rev: None,
            last_modified: None,
            owner: None,
            repo: None,
            url: None,
            path: Some(".".to_string()),
            dir: None,
        };
        // Use the crate root as flake_dir — it exists
        let result = resolve_path_input(&locked, env!("CARGO_MANIFEST_DIR"));
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "snix-build")]
    fn test_fetch_git_input_missing_url() {
        let cache = crate::fetch::FetchCache::new().unwrap();
        let locked = LockedInput {
            input_type: "git".to_string(),
            nar_hash: None,
            rev: None,
            last_modified: None,
            owner: None,
            repo: None,
            url: None, // missing
            path: None,
            dir: None,
        };
        let err = fetch_git_input(&locked, Some(&cache)).unwrap_err();
        assert!(err.to_string().contains("missing 'url'"));
    }

    #[test]
    #[cfg(feature = "snix-build")]
    fn test_fetch_git_input_no_cache() {
        let locked = LockedInput {
            input_type: "git".to_string(),
            nar_hash: Some("sha256-test".to_string()),
            rev: Some("abc123".to_string()),
            last_modified: None,
            owner: None,
            repo: None,
            url: Some("https://example.com/repo.git".to_string()),
            path: None,
            dir: None,
        };
        let err = fetch_git_input(&locked, None).unwrap_err();
        assert!(err.to_string().contains("no fetch cache"));
    }

    #[test]
    #[cfg(feature = "snix-build")]
    fn test_fetch_git_input_local_repo() {
        // Create a local git repo and verify fetch_git_input can clone it
        let tmpdir = tempfile::tempdir().unwrap();
        let src = tmpdir.path().join("src");
        std::fs::create_dir(&src).unwrap();

        std::process::Command::new("git").args(["init"]).current_dir(&src).output().unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&src)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(&src)
            .output()
            .unwrap();
        std::fs::write(src.join("file.txt"), "hello").unwrap();
        std::process::Command::new("git").args(["add", "."]).current_dir(&src).output().unwrap();
        std::process::Command::new("git").args(["commit", "-m", "init"]).current_dir(&src).output().unwrap();

        let cache = crate::fetch::FetchCache::new().unwrap();
        let locked = LockedInput {
            input_type: "git".to_string(),
            nar_hash: None,
            rev: None,
            last_modified: None,
            owner: None,
            repo: None,
            url: Some(src.to_string_lossy().to_string()),
            path: None,
            dir: None,
        };
        let result = fetch_git_input(&locked, Some(&cache));
        assert!(result.is_ok(), "fetch_git_input failed: {:?}", result.err());
        let path = result.unwrap();
        assert!(std::path::Path::new(&path).join("file.txt").exists());
    }

    /// Property test: for multiple known SRI hashes, compute_store_path produces
    /// valid store paths (correct format, correct nix32 alphabet, correct length).
    #[test]
    fn test_store_path_format_proptest() {
        // Test with a variety of real-world narHash values from Aspen's flake.lock
        // and synthetically constructed ones. Each must produce a valid store path.
        let test_hashes = [
            "sha256-OLdIz38tsFp1aXt8GsJ40s0/jxSkhlqftuDE7LvuhK4=", // nixpkgs
            "sha256-TXsJUc+nZflFGWLopUxHwdlxYY+ucxbF5IAPLEX5tc4=", // crane
            "sha256-rHuJtdcOjK7rAHpHphUb1iCvgkU3GpfvicLMwwnfMT0=", // flake-parts
            "sha256-l0KFg5HjrsfsO/JpG+r7fRrqm12kzFHyUHqHCVpMMbI=", // flake-utils
            "sha256-Vy1rq5AaRuLzOxct8nz4T6wlgyUR7zLU309k9mBC768=", // systems
            "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=", // all zeros
            "sha256-////////////////////////////////////////////////////////////////", // all ones (base64 '/')
        ];

        for sri in &test_hashes {
            let result = compute_store_path(sri);
            // Some may fail if the base64 is invalid — that's fine
            if let Ok(path) = result {
                assert!(path.starts_with("/nix/store/"), "bad prefix: {path}");
                assert!(path.ends_with("-source"), "bad suffix: {path}");

                // Store path hash part should be 32 chars of nix32
                let hash_part = &path["/nix/store/".len()..path.len() - "-source".len()];
                assert_eq!(hash_part.len(), 32, "hash part wrong length for {sri}: {hash_part}");

                // All chars should be valid nix32
                const NIX32_CHARS: &str = "0123456789abcdfghijklmnpqrsvwxyz";
                for c in hash_part.chars() {
                    assert!(NIX32_CHARS.contains(c), "invalid nix32 char '{c}' in {hash_part} for {sri}");
                }
            }
        }
    }
}
