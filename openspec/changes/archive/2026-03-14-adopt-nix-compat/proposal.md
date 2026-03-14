## Why

The codebase reimplements Nix primitives in two places: `aspen-cache` (nixbase32, narinfo, store paths, signing) and the CI executors (`nix nar dump-path` subprocess calls). The snix project provides battle-tested, spec-compliant implementations of all of these across several crates — `nix-compat` for format types, `snix-store` for NAR ingestion/writing, and `snix/nar-bridge` as a reference Nix binary cache server. We already depend on nix-compat and snix-store via the snix integration. Maintaining parallel implementations creates divergence risk, and shelling out to `nix nar dump-path` spawns a subprocess per store path output, adding process overhead and a hard dependency on the `nix` CLI binary being present at build-output upload time.

The snix ecosystem also provides `snix/nix-daemon` — a full `NixDaemonIO` implementation backed by snix services (blob, directory, pathinfo). This is the same daemon protocol rio-build reimplemented from scratch in `rio-nix`. For our immediate needs, we don't need the daemon protocol (we still use `nix build` for builds), but the NAR writer and format types are drop-in replacements for our hand-rolled code.

## What Changes

- **Replace `aspen-cache::nix32` with `nix_compat::nixbase32`**: Drop the hand-rolled nix32 encoder (~60 lines) in favor of the upstream implementation that also provides `decode`, `decode_fixed`, and proper error types.
- **Replace `aspen-cache::signing` with `nix_compat::narinfo::{SigningKey, VerifyingKey, parse_keypair}`**: Drop ~280 lines of custom Ed25519 signing. nix-compat provides identical functionality with the same `name:base64(key)` format.
- **Replace `aspen-cache::narinfo::CacheEntry::to_narinfo()` and `fingerprint()` with `nix_compat::narinfo::NarInfo` and `nix_compat::narinfo::fingerprint()`**: Drop the manual narinfo string builder and fingerprint formatter. Gains narinfo parsing for free.
- **Replace `aspen-cache::index::parse_store_path()` with `nix_compat::store_path::StorePath::from_absolute_path()`**: Drop the manual parser (~30 lines) for the upstream `StorePath` type with proper hash validation.
- **Replace `nix nar dump-path` subprocess calls with `nix_compat::nar::writer`**: Eliminate 3 subprocess call sites across `aspen-ci-executor-nix` (cache.rs, snix.rs) and `aspen-ci-executor-shell` (snix.rs). Use nix-compat's sync NAR writer in a `spawn_blocking` to walk store paths and produce byte-identical NAR archives in pure Rust. Compute SHA-256 in the same pass via a hashing writer wrapper.
- **Rewrite `aspen-nix-cache-gateway` narinfo rendering to match snix's `nar-bridge` pattern**: snix's `nar-bridge` crate demonstrates exactly how to construct `NarInfo` from a `PathInfo` and render it via `Display`. Our gateway does the same thing from `CacheEntry`. Adopt the same pattern: construct `NarInfo` with proper `StorePathRef`, `[u8; 32]` nar_hash, and `SignatureRef` types.

## Capabilities

### New Capabilities

- `nix-compat-adoption`: Replace hand-rolled Nix primitives in `aspen-cache`, `aspen-nix-cache-gateway`, and CI executors with `nix_compat` crate types for narinfo, nixbase32, store paths, signing, and NAR serialization.

### Modified Capabilities

(none — no spec-level behavioral changes, only implementation replacement)

## Impact

- **Crates modified**: `aspen-cache` (narinfo/nix32/signing/store paths), `aspen-nix-cache-gateway` (narinfo rendering), `aspen-ci-executor-nix` (NAR dump + nix32 in cache.rs and snix.rs), `aspen-ci-executor-shell` (NAR dump in snix.rs), `aspen-nix-handler` (consumer), `aspen-rpc-handlers` (tests)
- **Crates unaffected**: `aspen-snix`, `aspen-castore` — already use nix-compat and snix-store directly
- **Dependencies**: `nix-compat` added to `aspen-cache/Cargo.toml` and `aspen-ci-executor-shell/Cargo.toml`. Removes `ed25519-dalek`, `rand_core_06`, `base64`, `hex` direct deps from `aspen-cache`.
- **Subprocess elimination**: Removes 3 `Command::new("nix").args(["nar", "dump-path", ...])` call sites. The `nix` binary is still required for `nix build`, `nix path-info`, and `nix flake archive` — those are operational commands. Future work could use the `nix-daemon` protocol (as rio-build and snix/nix-daemon do) to replace `nix build` itself, but that's out of scope.
- **API surface**: `CacheEntry` struct stays (KV storage format). Signing types get thin wrappers. Public functions get replaced with re-exports or adapters.
