## 1. Dependencies and Module Structure

- [x] 1.1 Add `ureq`, `flate2`, `tar` deps to `aspen-ci-executor-nix/Cargo.toml` (flate2/tar from workspace, ureq new with version pin), gated behind `snix-build` feature
- [x] 1.2 Create `crates/aspen-ci-executor-nix/src/fetch.rs` module with public API: `FetchCache`, `fetch_and_unpack_github`, `fetch_and_unpack_tarball`, `verify_nar_hash`
- [x] 1.3 Register `fetch` module in `lib.rs` behind `#[cfg(feature = "snix-build")]`

## 2. Core Fetch Implementation

- [x] 2.1 Implement `fetch_and_unpack_tarball(url, dest_dir)` — HTTP GET with ureq, stream through `GzDecoder` + `tar::Archive`, unpack to dest_dir, return unpacked path. Enforce 2GB download limit and 300s timeout.
- [x] 2.2 Implement tarball prefix stripping — detect single top-level directory in unpacked tarball, return its path instead of the parent. Handle multi-entry tarballs by returning the parent directly.
- [x] 2.3 Implement `fetch_and_unpack_github(owner, repo, rev, dest_dir)` — construct `https://github.com/{owner}/{repo}/archive/{rev}.tar.gz` URL, delegate to `fetch_and_unpack_tarball`, strip the `{repo}-{rev}/` prefix.
- [x] 2.4 Implement `fetch_and_unpack_gitlab(host, owner, repo, rev, dest_dir)` — construct GitLab archive URL, delegate to `fetch_and_unpack_tarball`.

## 3. narHash Verification

- [x] 3.1 Implement `compute_nar_hash(dir_path)` — use `snix_store::nar` to serialize the directory to NAR format and compute SHA256 hash. Return SRI-formatted hash string.
- [x] 3.2 Implement `verify_nar_hash(dir_path, expected_sri)` — compute NAR hash, compare against expected, return Ok or error with both hashes. Clean up directory on mismatch.
- [x] 3.3 Add unit test: compute NAR hash of a known directory, verify deterministic output
- [x] 3.4 Add unit test: verify_nar_hash rejects tampered directory

## 4. Fetch Cache

- [x] 4.1 Implement `FetchCache` struct — `HashMap<String, PathBuf>` keyed by narHash SRI string, with a `tempdir` root for all cached directories. Concurrent access via `Mutex`.
- [x] 4.2 Implement `FetchCache::get_or_fetch(nar_hash, fetch_fn)` — return cached path if present, otherwise call fetch_fn, verify narHash, store in cache.
- [x] 4.3 Add `FetchCache` to `NixEvaluator` struct, pass through to `resolve_all_inputs`.

## 5. Wire Into flake_lock.rs

- [x] 5.1 Replace `fetch_github_input()` stub with call to `fetch_and_unpack_github` via `FetchCache`
- [x] 5.2 Replace `fetch_tarball_input()` stub with call to `fetch_and_unpack_tarball` via `FetchCache`
- [x] 5.3 Add `fetch_gitlab_input()` implementation
- [x] 5.4 Update `resolve_all_inputs()` signature to accept `Option<&FetchCache>` parameter
- [x] 5.5 Update `evaluate_flake_derivation()` in `eval.rs` to create and pass `FetchCache` to `resolve_all_inputs()`

## 6. Resource Bounds

- [x] 6.1 Add constants to `fetch.rs`: `MAX_DOWNLOAD_SIZE = 2GB`, `FETCH_TIMEOUT_SECS = 300`, `MAX_CONCURRENT_FETCHES = 4`
- [x] 6.2 Enforce download size limit via ureq response body `read` with byte counter
- [x] 6.3 Enforce timeout via ureq agent timeout configuration
- [x] 6.4 Add semaphore for concurrent fetch limiting (acquire before fetch, release after)

## 7. Tests

- [x] 7.1 Unit test: `fetch_and_unpack_tarball` with a small in-memory `.tar.gz` (no network)
- [x] 7.2 Unit test: prefix stripping for single-directory and multi-entry tarballs
- [x] 7.3 Unit test: `FetchCache` returns cached path on second call, doesn't call fetch_fn twice
- [x] 7.4 Unit test: `fetch_and_unpack_github` constructs correct URL from owner/repo/rev
- [x] 7.5 Integration test (network profile): fetch a small real GitHub archive (e.g., `nix-systems/default`), verify narHash matches flake.lock value
- [x] 7.6 Integration test (network profile): fetch + narHash verification with FetchCache caching (nix-systems/default from Aspen's flake.lock)
- [x] 7.7 Unit test: verify_nar_hash rejects mismatched hash
- [x] 7.8 Unit test: download size limit enforcement (abort on oversized response)
