## Why

The snix native build pipeline (eval → materialize → build → upload) works end-to-end for trivial derivations but has structural fragilities: placeholder nodes with zeroed digests mask missing PathInfoService entries, input materialization from upstream caches runs sequentially despite a concurrency limit of 8, missing inputs are silently skipped leading to cryptic sandbox failures, and no pre-flight check verifies all required store paths exist before starting the build. These issues compound for real-world builds with large dependency closures.

## What Changes

- Replace placeholder `B3Digest::from(&[0u8; 32])` nodes in `resolve_single_input` with proper local-store ingestion that computes real digests from filesystem content
- Make `populate_closure` in `UpstreamCacheClient` actually fetch NARs concurrently using the existing `MAX_CONCURRENT_FETCHES` semaphore
- Add pre-build input verification that checks all required store paths exist on disk after materialization, with a clear error listing every missing path
- Make `collect_input_store_paths` report unresolved input derivations as warnings (not silent debug skips) and propagate partial resolution metadata
- Add unit tests for `resolve_single_input`, `collect_input_store_paths`, placeholder-to-build path, and partial input resolution error cascade

## Capabilities

### New Capabilities

- `native-build-input-verification`: Pre-build check that all required store paths are present on disk, run between materialization and sandbox execution, with structured error reporting listing every missing path
- `concurrent-upstream-fetch`: Parallel NAR fetching from upstream binary caches during closure population, bounded by `MAX_CONCURRENT_FETCHES`

### Modified Capabilities

## Impact

- `crates/aspen-ci-executor-nix/src/build_service.rs`: `resolve_single_input`, `resolve_build_inputs`, `collect_input_store_paths`
- `crates/aspen-ci-executor-nix/src/upstream_cache.rs`: `populate_closure`
- `crates/aspen-ci-executor-nix/src/executor.rs`: `try_native_build` (insert verification step)
- `crates/aspen-ci-executor-nix/src/materialize.rs`: add ingestion helper for local filesystem paths
- Unit tests across all four modules
