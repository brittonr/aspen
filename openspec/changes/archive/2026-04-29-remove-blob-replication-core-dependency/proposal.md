## Why

`aspen-blob` already exposes reusable default APIs through `aspen-traits` and `aspen-kv-types`, but its `replication` feature still keeps an optional dependency on root `aspen-core`. The only current production need is `KvReplicaMetadataStore`, which reads, writes, deletes, and scans replica metadata through the KV interface. That broad dependency weakens the blob/castore/cache extraction boundary and leaves a stale policy exception that lets root core contracts leak into a reusable adapter feature.

Removing this dependency is a small, high-confidence seam hardening step before broader crate extraction work.

## What Changes

- **Blob replication KV adapter**: Migrate `KvReplicaMetadataStore` from `aspen_core::*` imports to `aspen-traits` capability/composite traits and `aspen-kv-types` request/response/error types.
- **Manifest feature topology**: Remove the optional `aspen-core` dependency from `crates/aspen-blob/Cargo.toml` and keep `replication` tied only to client-RPC schema needs plus existing blob backend dependencies.
- **Extraction policy/docs**: Update blob/castore/cache extraction metadata so `aspen-blob -> aspen-core` is no longer an allowed exception, and record the new replication adapter proof.
- **Verification rails**: Prove `cargo check -p aspen-blob --features replication` still works and `cargo tree -p aspen-blob --features replication -e normal` no longer reaches `aspen-core` except through unrelated dev/test-only paths if any.

## Capabilities

### Modified Capabilities

- `blob-castore-cache-extraction`: Tightens the blob replication adapter contract so replication may use client RPC schemas but must not import root `aspen-core` when leaf KV traits/types are sufficient.
- `architecture-modularity`: Reinforces the focused blob replication adapter rule that KV-only metadata persistence should choose leaf KV contracts before reaching for broad root core packages.

## Impact

- **Files**: `crates/aspen-blob/Cargo.toml`, `crates/aspen-blob/src/replication/adapters.rs`, `docs/crate-extraction/blob-castore-cache.md`, `docs/crate-extraction/policy.ncl`, and evidence under this change.
- **APIs**: No intended public behavior change. Generic bounds remain a KV store requirement, but the dependency source changes to leaf trait/type crates.
- **Dependencies**: Removes optional `aspen-core` from `aspen-blob`; keeps `aspen-client-api` behind `replication` for `BlobReplicatePull` wire messages.
- **Testing**: Focused cargo check, focused adapter tests, cargo tree boundary proof, extraction readiness checker, negative mutation proof, and OpenSpec preflight evidence.

## Out of Scope

- Splitting iroh/iroh-blobs out of `aspen-blob`.
- Making `aspen-snix` independent from `aspen-core`.
- Raising blob/castore/cache readiness state above `workspace-internal`.
- Changing replication wire schemas or behavior.
