## Why

The first `aspen-core` no-std split established the top-level shell boundary, but the alloc-only foundation still leaks runtime baggage through a few leaf and wire crates. `aspen-storage-types` still drags `redb` and `libc` into the foundational graph for a table definition, `aspen-traits` still depends on `std::sync::Arc`, and `aspen-client-api` plus the protocol crates still carry avoidable std-only collection and serialization baggage.

This is next highest-ROI seam work because it removes the fattest remaining fake "type crate" boundary first, then hardens the portable trait and wire layers before the larger `aspen-raft-types` / transport surgery.

## What Changes

- Split the storage seam so alloc-safe record types stay in `crates/aspen-storage-types` while Redb-specific table definitions move behind `aspen-core-shell`; shell consumers keep the existing `aspen_core::storage::SM_KV_TABLE` import path through the established `aspen-core-shell` alias pattern.
- Make `crates/aspen-traits` genuinely alloc-safe, keeping trait contracts portable; this cut expects `alloc::sync::Arc` to preserve the supported convenience behavior so no separate shell-side trait shim is planned.
- Make `crates/aspen-client-api` alloc-safe in production code by replacing `std` collections and std-only postcard helpers with alloc-safe equivalents while preserving postcard discriminant stability, and rewrite the crate's postcard tests to use alloc-safe serializers rather than relying on production std helpers.
- Trim `crates/aspen-{forge,jobs,coordination}-protocol` so std-only JSON helpers remain test-only instead of production dependencies.
- Update `aspen-core` / `aspen-core-shell` export inventories and no-std documentation to reflect the narrower alloc-safe surface.
- Add deterministic compile, dependency, and negative-regression rails for the leaf and wire crates touched by this split, including explicit alloc-only / `no_std` compile proofs for `aspen-storage-types`, `aspen-traits`, `aspen-client-api`, and the protocol crates plus failure-triggering source audits for forbidden runtime helpers.
- Add a deterministic postcard-compatibility artifact for `ClientRpcRequest` and `ClientRpcResponse` so discriminant stability is reviewable instead of inferred from chat or memory; the artifact must record at least one variant-keyed default-production encoding for every request/response enum variant.
- Explicitly defer `aspen-raft-types` transport-address surgery to a later change; this proposal only prepares that path by cleaning lower-cost seams first.

## Non-Goals

- Reworking `aspen-raft-types` to eliminate Iroh transport types in this change.
- Changing `ClientRpcRequest` / `ClientRpcResponse` feature defaults or reordering any wire enum variants.
- Introducing a new public storage shell crate when the existing `aspen-core-shell` alias path is sufficient.
- Changing runtime behavior outside the listed leaf, core-shell, and wire-crate seam cleanup.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities
- `core`: narrow the alloc-only core inventory so storage record types stay portable while Redb-specific table definitions move behind an explicit shell boundary.
- `architecture-modularity`: extend the alloc-safe modularity rules from `aspen-core` into the remaining foundational leaf and wire crates so shared traits and protocol crates do not silently pull std/runtime baggage.

## Impact

- **Files**: `crates/aspen-storage-types/`, `crates/aspen-core/`, `crates/aspen-core-shell/`, `crates/aspen-traits/`, `crates/aspen-client-api/`, `crates/aspen-{forge,jobs,coordination}-protocol/`, workspace `Cargo.toml`, docs, and OpenSpec delta specs.
- **APIs**: alloc-only consumers lose direct access to Redb table-definition exports from the core surface; shell consumers keep `aspen_core::storage::SM_KV_TABLE` through the existing `aspen-core-shell` alias path. `ClientRpcRequest` / `ClientRpcResponse` wire behavior stays compatible; protocol crates keep their intended schemas while this change focuses on alloc-safe production surfaces and dependency cleanup rather than separate protocol baseline artifacts.
- **Dependencies**: `redb`/`libc` leave the alloc-only foundation path; protocol crates stop treating `serde_json` as a production dependency; `aspen-client-api` stops depending on std-only collection helpers in production code.
- **Testing**: compile-slice, dependency-boundary, protocol-dependency, alloc-only target-build, negative source-audit, and wire-compatibility rails will expand to cover storage, traits, client-api, and protocol crates in both positive and negative cases, with fresh saved evidence under `openspec/changes/extend-no-std-foundation-and-wire/evidence/`, a deterministic postcard baseline artifact, and representative consumer slices for `cargo check -p aspen-cluster`, `cargo check -p aspen-client`, `cargo check -p aspen-cli`, `cargo check -p aspen-rpc-handlers`, and `cargo check -p aspen --no-default-features --features node-runtime`, plus traceability in `verification.md`. Archived evidence from older no-std changes is baseline context only, not final proof for this change.
