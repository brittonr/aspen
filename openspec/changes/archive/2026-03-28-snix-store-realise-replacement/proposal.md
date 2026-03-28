## Why

The native build pipeline's `try_native_build` still falls back to `nix-store --realise` subprocesses when input store paths are missing from `/nix/store`. This is the last subprocess escape on the build path — the zero-subprocess pipeline (snix-eval → snix-build) skips it when all inputs are present, but the fallback remains for cases where eval didn't materialize all dependencies (e.g. derivation came from `nix eval` subprocess). Replacing this with in-process fetching from Aspen's PathInfoService/BlobService or upstream substituters via snix-store eliminates the fallback entirely.

## What Changes

- Add in-process store path materialization: given a set of missing store paths, fetch their content from Aspen's `PathInfoService` + `BlobService` (cluster store) or upstream Nix binary caches via snix-store's substituter infrastructure, then write NAR archives to `/nix/store` using `nix_compat::nar`
- Replace the two `nix-store --realise` subprocess calls in `executor.rs` with the new in-process materializer
- Keep `nix-store --realise` as a gated fallback behind `nix-cli-fallback` feature for environments without snix services configured
- Add integration tests that verify the in-process path works with an in-memory PathInfoService populated with test store paths

## Capabilities

### New Capabilities

- `store-path-materializer`: In-process materialization of missing `/nix/store` paths from Aspen's distributed store or upstream Nix binary caches, replacing `nix-store --realise` subprocess

### Modified Capabilities

## Impact

- `crates/aspen-ci-executor-nix/src/executor.rs` — `try_native_build` Step 2 (realise input closure)
- `crates/aspen-ci-executor-nix/src/build_service.rs` — may need new helper functions for NAR-to-filesystem unpacking
- `crates/aspen-snix/` — may provide PathInfoService/BlobService access patterns for content retrieval
- Dependencies: `snix-store` (substituter/NarCalculationService), `nix-compat` (NAR reader/writer), `aspen-cache` (existing NAR utilities)
- No API changes — this is internal to the build executor
