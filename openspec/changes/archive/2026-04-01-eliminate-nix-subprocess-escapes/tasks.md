## 1. In-process NAR export (replace nix nar dump-path and nix path-info)

- [x] 1.1 Add `pathinfo_lookup` helper in `cache.rs` that queries PathInfoService by store path digest and returns NAR size, NAR hash, references, deriver
- [x] 1.2 Rewrite `check_store_path_size` to use `pathinfo_lookup` when snix services are configured, falling back to `nix path-info --json` only with `nix-cli-fallback`
- [x] 1.3 Rewrite `upload_store_paths` to use castore ingestion + SimpleRenderer NAR hash instead of `nix nar dump-path` pipe
- [x] 1.4 Gate existing `nix path-info` and `nix nar dump-path` calls behind `#[cfg(feature = "nix-cli-fallback")]`
- [x] 1.5 Unit tests: `pathinfo_lookup` returns correct metadata, `upload_store_paths` round-trips through castore

## 2. In-process closure walk (replace nix-store -qR)

- [x] 2.1 Extract `compute_input_closure_via_pathinfo` into its own public function in `build_service.rs` with clear return type (full closure or partial with unresolved list)
- [x] 2.2 Add BFS reference walking that queries PathInfoService for each path's references, with visited set and `MAX_CLOSURE_PATHS` bound
- [x] 2.3 Gate `compute_input_closure` (nix-store -qR version) behind `#[cfg(feature = "nix-cli-fallback")]`
- [x] 2.4 Wire the PathInfoService closure walk as the primary path in `derivation_to_build_request`
- [x] 2.5 Unit tests: complete closure from mock PathInfoService, self-referencing paths, cycle detection, exceeds limit error

## 3. Replace curl with reqwest in fetch.rs

- [x] 3.1 Add `reqwest` to `aspen-ci-executor-nix` Cargo.toml (gated on `snix-build` feature since tarball fetching is only used by the native eval path)
- [x] 3.2 Rewrite `fetch_tarball_url` to use `reqwest::get()` with streaming body → `flate2::read::GzDecoder` → `tar::Archive` pipeline
- [x] 3.3 Implement download timeout via `reqwest::Client::builder().timeout()` matching `FETCH_TIMEOUT_SECS`
- [x] 3.4 Implement download size limit by tracking bytes read and aborting if exceeding `MAX_DOWNLOAD_SIZE`
- [x] 3.5 Gate `curl` subprocess path behind `#[cfg(feature = "nix-cli-fallback")]` as fallback
- [x] 3.6 Unit test: fetch small tarball from local HTTP server, verify unpacked content matches
- [x] 3.7 Rewrite `fetch_git_repo_via_archive` to use reqwest for the HTTP archive download (keep `git clone` for actual git operations)

## 4. Upstream binary cache client

- [x] 4.1 Create `upstream_cache.rs` module with `UpstreamCacheClient` struct holding `reqwest::Client`, cache URLs, and snix service references
- [x] 4.2 Implement `fetch_narinfo(store_path_hash)` — HTTP GET `<cache>/<hash>.narinfo`, parse via `nix_compat::narinfo::NarInfo`
- [x] 4.3 Implement `fetch_nar(nar_url, expected_hash, compression)` — download NAR, decompress (xz/zstd/bzip2/none), verify hash
- [x] 4.4 Implement `ingest_nar(nar_bytes)` — feed NAR into snix-castore ingestion, register PathInfo in PathInfoService
- [x] 4.5 Implement `populate_closure(store_path_hashes)` — BFS over references, fetch_narinfo + fetch_nar + ingest for each missing path, bounded by `MAX_CLOSURE_PATHS`
- [x] 4.6 Add `UpstreamCacheConfig` to `NixBuildWorkerConfig` with default cache.nixos.org URL and key
- [x] 4.7 Unit tests: narinfo parsing, NAR hash verification, closure BFS termination
- [x] 4.8 Integration test: populate a single store path from a mock HTTP cache server

## 5. Wire upstream cache into build pipeline

- [x] 5.1 In `executor.rs` `try_native_build`, after castore materialization reports unresolved paths, try `UpstreamCacheClient::populate_closure` before falling back to `nix-store --realise`
- [x] 5.2 In `build_service.rs`, when `compute_input_closure_via_pathinfo` encounters unresolved paths, try fetching their narinfo from upstream cache to populate PathInfoService, then retry closure walk
- [x] 5.3 Gate all `nix-store --realise` calls in executor.rs behind `#[cfg(feature = "nix-cli-fallback")]`
- [x] 5.4 Gate `resolve_drv_path` (nix eval --raw), `spawn_nix_build`, and `ensure_flake_lock` behind `#[cfg(feature = "nix-cli-fallback")]`

## 6. Feature flag cleanup

- [x] 6.1 Audit all `Command::new("nix*")` calls in aspen-ci-executor-nix — every one must be inside `#[cfg(feature = "nix-cli-fallback")]`
- [x] 6.2 Update Cargo.toml feature documentation to reflect new semantics: `nix-cli-fallback` is opt-in for all subprocess calls
- [x] 6.3 Ensure `snix-build` without `nix-cli-fallback` compiles cleanly with no dead code warnings

## 7. NixOS VM tests

- [ ] 7.1 Update `snix-pure-build-test` to build a derivation with actual transitive dependencies (bash, coreutils) and verify zero subprocess calls
- [ ] 7.2 New VM test `upstream-cache-bootstrap-test`: cluster starts with empty PathInfoService, submits a build job, upstream cache client populates inputs from cache.nixos.org, build succeeds
- [ ] 7.3 Verify existing `snix-flake-native-build-test` and `npins-native-eval-test` still pass

## 8. Documentation

- [x] 8.1 Update `docs/nix-integration.md` data flow diagrams to reflect the upstream cache client path
- [x] 8.2 Update AGENTS.md feature flag table if any feature semantics changed
