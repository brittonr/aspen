Evidence-ID: extend-no-std-foundation-and-wire.root-verification
Artifact-Type: claim-to-artifact-index

# Verification Evidence

This file is the root claim-to-artifact index for `extend-no-std-foundation-and-wire`.
Durable proof for this change must live under `openspec/changes/extend-no-std-foundation-and-wire/evidence/`.

## Evidence Freshness Rule

- Only artifacts generated under `openspec/changes/extend-no-std-foundation-and-wire/evidence/` and indexed from this file satisfy this change's verification claims.
- Archived evidence from earlier no-std changes may be cited as baseline context only.
- Archived evidence MUST NOT substitute for final proof of the storage, trait, client-api, or protocol seams changed here.

## Evidence Naming Convention

Planned artifact families for this change:

- storage seam and UI/boundary artifacts: `evidence/storage-seam-*.txt`, `evidence/storage-seam-*.md`, saved stderr snapshots, and source-audit outputs
- core foundation and dependency-boundary artifacts: `evidence/core-foundation-*.txt`, `evidence/core-foundation-*.md`, dependency graphs, feature graphs, and target-build transcripts
- wire dependency artifacts: `evidence/wire-dependency-*.txt`, `evidence/wire-dependency-*.md`, dependency graphs, feature graphs, and target-build transcripts
- wire compatibility artifacts: `evidence/wire-compatibility-*.txt`, `evidence/wire-compatibility-*.md`, `evidence/client-rpc-postcard-baseline.json`, and test transcripts comparing current encodings against that baseline
- traceability plans: `evidence/verification-plan.md`, `evidence/*-verification.md`

## Implementation Evidence

- Changed file: `Cargo.lock`
- Changed file: `crates/aspen-client-api/Cargo.toml`
- Changed file: `crates/aspen-client-api/tests/client_rpc_postcard_baseline.rs`
- Changed file: `openspec/changes/extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json`
- Changed file: `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-compatibility-verification.md`
- Changed file: `openspec/changes/extend-no-std-foundation-and-wire/evidence/implementation-diff.txt`
- Changed file: `openspec/changes/extend-no-std-foundation-and-wire/evidence/openspec-preflight.txt`
- Changed file: `openspec/changes/extend-no-std-foundation-and-wire/tasks.md`
- Changed file: `openspec/changes/extend-no-std-foundation-and-wire/verification.md`

## Task Coverage

Completed tasks indexed here:

- [x] I3 Add crate-level `#![cfg_attr(not(test), no_std)]` plus `extern crate alloc` scaffolding to `crates/aspen-client-api`, update the shared wire verification rails (`python3 scripts/check-foundation-wire-deps.py --mode wire` and `python3 scripts/check-foundation-wire-source-audits.py --mode wire`) needed by `V3`, then convert its production modules to alloc-safe collections/codecs while preserving feature defaults, append-only postcard discriminants, alloc-safe postcard regression tests, and the all-variants postcard baseline harness. That harness must derive canonical payloads from fixed rules only (stable variant-name-derived strings, fixed integers, fixed bytes, deterministic single-entry collections, recursively canonical nested values) with no randomness, clock reads, or environment-dependent inputs, write variant-name-keyed baseline entries for every `ClientRpcRequest` and `ClientRpcResponse` variant, and deterministically enumerate the live request/response variant sets so the test fails before comparison if any current variant lacks a baseline entry. [covers=architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.client-api-production-modules-avoid-std-only-helpers,architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.client-api-postcard-tests-use-alloc-safe-serializers,architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-crates-reject-forbidden-std-helpers,architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-compatibility-survives-alloc-safe-refactor,architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-compatibility-verification-is-reviewable]
  - Evidence: `crates/aspen-client-api/Cargo.toml`, `crates/aspen-client-api/tests/client_rpc_postcard_baseline.rs`, `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-dependency-verification.md`, `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-compatibility-verification.md`, `openspec/changes/extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json`

- [x] V4 Save wire-compatibility evidence proving `cargo test -p aspen-client-api` and `cargo test -p aspen-client-api client_rpc_postcard_baseline -- --nocapture`, alloc-safe postcard test serializers, deterministic `openspec/changes/extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json` generation plus comparison via `client_rpc_postcard_baseline`, at least one variant-keyed default-production encoding for every `ClientRpcRequest` and `ClientRpcResponse` enum variant, a completeness check against the live enum set that fails on any missing baseline entry, canonical payload generation from fixed rules only (stable variant-name-derived strings, fixed integers, fixed bytes, deterministic single-entry collections, recursively canonical nested values) with no randomness, clock reads, or environment-dependent inputs, postcard discriminants, default feature behavior, and explicit references in `verification.md` to the representative consumer build artifacts saved by `V3`. [covers=architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.client-api-postcard-tests-use-alloc-safe-serializers,architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-compatibility-survives-alloc-safe-refactor,architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-compatibility-verification-is-reviewable] [evidence=openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-compatibility-verification.md]
  - Evidence: `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-compatibility-verification.md`, `openspec/changes/extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json`, `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-dependency-verification.md`

Remaining verification-plan anchors:

- `V1` → `openspec/changes/extend-no-std-foundation-and-wire/evidence/storage-seam-verification.md`
- `V2` → `openspec/changes/extend-no-std-foundation-and-wire/evidence/core-foundation-verification.md`
- `V3` → `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-dependency-verification.md`
- `V5` → `openspec/changes/extend-no-std-foundation-and-wire/verification.md` with supporting plan context in `openspec/changes/extend-no-std-foundation-and-wire/evidence/verification-plan.md`

## Review Scope Snapshot

### `git diff HEAD -- Cargo.lock crates/aspen-client-api/Cargo.toml crates/aspen-client-api/tests/client_rpc_postcard_baseline.rs openspec/changes/extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-compatibility-verification.md openspec/changes/extend-no-std-foundation-and-wire/tasks.md openspec/changes/extend-no-std-foundation-and-wire/verification.md`

- Status: captured
- Artifact: `openspec/changes/extend-no-std-foundation-and-wire/evidence/implementation-diff.txt`

## Verification Commands

### `cargo test -p aspen-client-api`

- Status: pass
- Artifact: `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-compatibility-verification.md`

### `cargo test -p aspen-client-api client_rpc_postcard_baseline -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-compatibility-verification.md`

### `scripts/openspec-preflight.sh extend-no-std-foundation-and-wire`

- Status: pass
- Artifact: `openspec/changes/extend-no-std-foundation-and-wire/evidence/openspec-preflight.txt`

## Scenario Coverage

Scenarios covered by the newly completed wire-baseline work:

- `architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.client-api-production-modules-avoid-std-only-helpers`
  - `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-dependency-verification.md`
  - `crates/aspen-client-api/Cargo.toml`
- `architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.client-api-postcard-tests-use-alloc-safe-serializers`
  - `crates/aspen-client-api/tests/client_rpc_postcard_baseline.rs`
  - `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-compatibility-verification.md`
- `architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-crates-reject-forbidden-std-helpers`
  - `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-dependency-verification.md`
- `architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-compatibility-survives-alloc-safe-refactor`
  - `openspec/changes/extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json`
  - `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-compatibility-verification.md`
- `architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-compatibility-verification-is-reviewable`
  - `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-compatibility-verification.md`
  - `openspec/changes/extend-no-std-foundation-and-wire/evidence/wire-dependency-verification.md`

Other scenarios from `openspec/changes/extend-no-std-foundation-and-wire/specs/core/spec.md` and `openspec/changes/extend-no-std-foundation-and-wire/specs/architecture-modularity/spec.md` remain tracked by their existing evidence files under this change's `evidence/` directory.

## Notes

Before any task is checked complete, update this file from plan-only status to a real claim-to-artifact index and keep it synchronized with the per-topic evidence files under `evidence/`.
