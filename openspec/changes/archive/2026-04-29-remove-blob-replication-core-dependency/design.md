## Context

`crates/aspen-blob/Cargo.toml` currently makes `aspen-core` optional and enables it through the `replication` feature. The default blob graph already depends on `aspen-traits` and `aspen-kv-types`, and `BlobAwareKeyValueStore` uses those leaf contracts. The remaining broad edge is `crates/aspen-blob/src/replication/adapters.rs`, where `KvReplicaMetadataStore` imports `aspen_core::traits::KeyValueStore`, `aspen_core::kv::*`, and `aspen_core::error::*` for replica metadata persistence.

The replication adapter still legitimately needs `aspen-client-api` for `BlobReplicatePull` RPC wire messages. This change narrows only the KV persistence dependency.

## Goals / Non-Goals

**Goals:**

- Remove `aspen-core` from `aspen-blob` dependencies and `replication` feature wiring.
- Move `KvReplicaMetadataStore` imports and trait bounds to `aspen-traits` and `aspen-kv-types`.
- Preserve replica metadata key format, JSON payload format, status filtering, and error context.
- Update extraction docs/policy so `aspen-blob -> aspen-core` is no longer accepted as a normal exception.
- Save positive and negative evidence proving the boundary.

**Non-Goals:**

- Remove `aspen-client-api` from the replication feature.
- Split iroh transport out of `aspen-blob`.
- Refactor `aspen-snix` or other consumers that still use `aspen_core::*`.
- Raise blob/castore/cache readiness above `workspace-internal`.

## Decisions

### 1. Use existing leaf KV contracts directly

**Choice:** Change `KvReplicaMetadataStore` to use `aspen_traits::KeyValueStore` and `aspen_kv_types::{ReadRequest, WriteRequest, WriteCommand, DeleteRequest, ScanRequest, KeyValueStoreError, ...}`.

**Rationale:** This is the smallest change that removes the root-core edge while preserving the existing full-store contract. Narrower capability bounds can follow later if the adapter is decomposed further, but the immediate ROI is eliminating the broad package dependency.

**Alternative:** Introduce a new `ReplicaMetadataKv` trait now. Rejected for this slice because the existing adapter methods need read/write/delete/scan and the current composite trait already represents that behavior; adding a new port would increase implementation and compatibility work without being needed to remove `aspen-core`.

**Implementation:** Update imports, trait bounds, and test fixture impls in `replication/adapters.rs`; remove the optional manifest dependency and feature edge.

### 2. Keep client RPC schema under `replication`

**Choice:** Keep `aspen-client-api` in the `replication` feature for transfer RPC construction.

**Rationale:** The ROI target is the root core dependency. Client RPC schemas are an explicit protocol/wire adapter dependency already documented by the blob/castore/cache manifest.

**Alternative:** Split RPC transfer into a separate crate/feature. Deferred because it is higher-effort and not required for the current boundary win.

### 3. Make policy fail on stale `aspen-core` exception

**Choice:** Remove the `aspen-blob -> aspen-core` exception from `docs/crate-extraction/policy.ncl` and update manifest text that called it a later seam.

**Rationale:** Without policy cleanup, future checks can silently accept the regression this change removes.

**Alternative:** Leave policy unchanged until broader readiness work. Rejected because stale exceptions reduce the value of this seam.

## Risks / Trade-offs

**[Trait path mismatch]** → Existing compatibility consumers may still pass `aspen_core::KeyValueStore` implementors. Mitigation: `aspen-core` re-exports the same leaf trait, so implementors should satisfy `aspen_traits::KeyValueStore` when dependency unification is correct; verify with `cargo check -p aspen-blob --features replication` and representative consumer checks if needed.

**[Error type import drift]** → Tests and pattern matches may reference `aspen_core::error::KeyValueStoreError`. Mitigation: update test fixture imports to `aspen_kv_types::KeyValueStoreError` and preserve negative tests for missing/malformed metadata.

**[False cargo tree positives]** → `aspen-client-api` may pull key-only `iroh-base`; iroh/iroh-blobs remain backend exceptions. Mitigation: boundary evidence checks exact `aspen-core` absence, not all backend dependencies.

## Validation Plan

- `cargo check -p aspen-blob --features replication`
- Focused adapter tests for replica metadata get/save/delete/scan, including missing and malformed metadata cases if present.
- `cargo tree -p aspen-blob --features replication -e normal` with a saved grep/audit proving root `aspen-core` is absent.
- `scripts/check-crate-extraction-readiness.rs --candidate-family blob-castore-cache` after policy/docs update, plus a negative mutation proving an `aspen-blob -> aspen-core` policy exception or dependency is rejected.
- `scripts/openspec-preflight.sh remove-blob-replication-core-dependency` before marking tasks complete.
