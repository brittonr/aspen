## Context

The first no-std split moved obvious runtime helpers out of `aspen-core`, but several lower-tier crates still blur the seam:

- `crates/aspen-storage-types` mixes alloc-safe `KvEntry` with Redb-specific `SM_KV_TABLE`, which still pulls `redb` and `libc` into the alloc-only core graph.
- `crates/aspen-traits` still uses `std::sync::Arc`, so one of the core leaf crates is not actually alloc-safe.
- `crates/aspen-client-api` still imports `std::collections::BTreeMap` in production modules and enables `postcard`'s `use-std` feature only because tests use `to_stdvec`.
- `crates/aspen-{forge,jobs,coordination}-protocol` are nearly alloc-only already, but they still carry `serde_json` as a production dependency even though JSON helpers are only used in tests.

These seams matter because they are shared widely: they affect the `aspen-core` boundary, the portable trait layer, and the public wire surface used by CLI, handlers, and future constrained consumers.

## Goals / Non-Goals

**Goals:**
- Remove Redb-specific definitions from the alloc-only foundation path while keeping alloc-safe storage record types portable.
- Make `aspen-traits` genuinely alloc-safe without losing ergonomic shared-pointer usage.
- Make `aspen-client-api` and the protocol crates alloc-safe in production code.
- Preserve existing wire behavior: enum discriminants, feature defaults, and runtime consumer compatibility.
- Add deterministic verification rails for the new seams.

**Non-Goals:**
- Reworking `aspen-raft-types` to eliminate Iroh transport types in this change.
- Changing `ClientRpcRequest` / `ClientRpcResponse` feature defaults or reordering any wire enum variants.
- Making the entire workspace `no_std`.
- Replacing Redb, postcard, or the existing RPC schema.

## Decisions

### 1. Move Redb table definitions to the shell side, not the alloc leaf crate

**Choice:** keep `crates/aspen-storage-types` as the alloc-safe home for `KvEntry` and any other pure storage records/codecs, enforce an alloc-safe production surface there with `#![cfg_attr(not(test), no_std)]`, and move `SM_KV_TABLE` into the default shell base of `aspen-core-shell`. `aspen-core` will re-export only the alloc-safe record surface; `aspen-core-shell` keeps the shell-visible table-definition path.

**Rationale:** `SM_KV_TABLE` is the only reason `redb` and `libc` stay in the alloc-only graph. Moving the table definition to the shell side removes the heavy dependency leak without changing the data model. Keeping the shell-visible path in `aspen-core-shell` also preserves the current import experience for shell consumers that alias `aspen-core-shell` as `aspen-core`.

**Alternative:** create a brand-new public leaf crate for one Redb constant. Rejected for now because the shell crate already exists and can preserve current shell paths with less churn.

### 2. Make `aspen-traits` alloc-safe by using `alloc::sync::Arc`

**Choice:** convert `crates/aspen-traits` to `#![cfg_attr(not(test), no_std)]`, import `alloc::sync::Arc`, and keep the trait signatures plus blanket impl ergonomics in the leaf crate.

**Rationale:** `Arc` itself is alloc-safe, so there is no reason for `aspen-traits` to stay std-bound. This preserves existing trait ergonomics while removing a fake std boundary from the core dependency path.

**Alternative:** move all `Arc<T>` blanket impls to `aspen-core-shell`. Rejected because it would create unnecessary split-brain ergonomics in a crate whose core behavior is otherwise already portable.

### 3. Make wire crates alloc-first in production code while keeping existing behavior

**Choice:** convert `aspen-client-api` and the protocol crates to alloc-first production modules, with explicit crate-level no-std enforcement for every touched wire crate:

- `#![cfg_attr(not(test), no_std)]` on `aspen-client-api`, `aspen-coordination-protocol`, `aspen-jobs-protocol`, and `aspen-forge-protocol`
- `extern crate alloc`
- replace `std::collections::BTreeMap` with `alloc::collections::BTreeMap`
- remove production dependence on `postcard::to_stdvec` by switching tests/helpers to alloc-safe postcard APIs
- move `serde_json` to dev-dependencies where JSON is only used in tests
- add a deterministic postcard baseline artifact for representative `ClientRpcRequest` / `ClientRpcResponse` encodings so discriminant stability is compared against saved bytes, not inferred from memory
- make the baseline harness derive canonical per-variant payloads from fixed rules only: stable variant-name-derived strings, fixed integers, fixed byte sequences, deterministic single-entry collections, and recursively canonical nested values with no randomness, clock reads, or environment-dependent inputs

Feature defaults and enum ordering stay unchanged.

**Rationale:** these crates are mostly pure message definitions already. The remaining std-only helpers are accidental, not architectural. Fixing them yields a portable wire surface without changing transport semantics.

**Alternative:** leave wire crates std-bound until `aspen-raft-types` is refactored too. Rejected because the leaf/wire cleanup is lower risk and independently valuable.

### 4. Preserve shell-facing import paths, narrow alloc-only inventory

**Choice:** alloc-only consumers of `aspen-core` lose direct access to Redb table definitions, but shell consumers keep the existing shell-facing path through `aspen-core-shell` aliasing. The documented alloc-only inventory changes from `storage::{KvEntry, SM_KV_TABLE}` to alloc-safe storage record exports only.

**Rationale:** the alloc-only contract should describe what truly compiles without runtime baggage. The shell alias path already exists precisely to preserve existing shell imports.

**Alternative:** keep re-exporting `SM_KV_TABLE` from alloc-only `aspen-core` and tolerate `redb` in the graph. Rejected because it defeats the seam.

### 5. Verification expands from core-only rails to leaf and wire rails

**Choice:** verification for this change will add deterministic rails for:

- the alloc-only `aspen-core` feature-topology matrix: `cargo check -p aspen-core`, `cargo check -p aspen-core --no-default-features`, `cargo check -p aspen-core --no-default-features --features sql`, and `python3 scripts/check-aspen-core-feature-claims.py` proving `aspen-core` keeps `default = []` and the documented package/feature claims remain accurate
- the `aspen-core-shell` shell feature matrix: `cargo check -p aspen-core-shell`, `cargo check -p aspen-core-shell --features layer`, `cargo check -p aspen-core-shell --features global-discovery`, and `cargo check -p aspen-core-shell --features sql`
- the alloc-only direct prerequisite assertion for `aspen-core`: `cargo tree -p aspen-core --no-default-features -e normal --depth 1`, plus `cargo tree -p aspen-core --no-default-features -e normal` and `cargo tree -p aspen-core --no-default-features -e features`, compared against the exact prerequisite set named in the architecture-modularity spec and the allowlist/feature-leak contract
- `cargo check -p aspen-core-no-std-smoke`
- `cargo check -p aspen-storage-types`, `cargo check -p aspen-storage-types --no-default-features`, `cargo check -p aspen-storage-types --no-default-features --target wasm32-unknown-unknown`, `cargo tree -p aspen-storage-types -e normal`, `cargo tree -p aspen-storage-types -e features`, `python3 scripts/check-foundation-wire-deps.py --mode leaf`, and deterministic source-audit artifacts from `python3 scripts/check-foundation-wire-source-audits.py --mode leaf`, which must treat `tests/` trees and `#[cfg(test)]` code as test-only while proving `redb::TableDefinition` stays out of production modules and `redb` / `libc` stay out of the production graph
- `cargo check -p aspen-traits`, `cargo check -p aspen-traits --no-default-features`, `cargo check -p aspen-traits --no-default-features --target wasm32-unknown-unknown`, `cargo tree -p aspen-traits -e normal`, `cargo tree -p aspen-traits -e features`, `cargo tree -p aspen-traits -e features -i aspen-cluster-types`, `python3 scripts/check-foundation-wire-deps.py --mode leaf`, and deterministic source-audit artifacts from `python3 scripts/check-foundation-wire-source-audits.py --mode leaf`, which must treat `tests/` trees and `#[cfg(test)]` code as test-only while proving `std::sync::Arc` stays out of production modules and the `aspen-traits` → `aspen-cluster-types` path keeps alloc-only feature settings without leaking `iroh` / `iroh-base`
- `cargo check -p aspen-client-api`, `cargo check -p aspen-client-api --no-default-features`, `cargo check -p aspen-client-api --no-default-features --target wasm32-unknown-unknown`, `cargo tree -p aspen-client-api -e normal`, `cargo tree -p aspen-client-api -e features`, `python3 scripts/check-foundation-wire-deps.py --mode wire`, and deterministic source-audit artifacts from `python3 scripts/check-foundation-wire-source-audits.py --mode wire`, which must treat `tests/` trees and `#[cfg(test)]` code as test-only while proving `std::collections` / `postcard::to_stdvec` stay out of production modules and forbidden production std/runtime dependencies stay out of the wire graph
- `cargo check -p aspen-coordination-protocol`, `cargo check -p aspen-coordination-protocol --no-default-features`, `cargo check -p aspen-coordination-protocol --no-default-features --target wasm32-unknown-unknown`, `cargo check -p aspen-jobs-protocol`, `cargo check -p aspen-jobs-protocol --no-default-features`, `cargo check -p aspen-jobs-protocol --no-default-features --target wasm32-unknown-unknown`, `cargo check -p aspen-forge-protocol`, `cargo check -p aspen-forge-protocol --no-default-features`, and `cargo check -p aspen-forge-protocol --no-default-features --target wasm32-unknown-unknown`, paired with `cargo tree -p <crate> -e normal`, `cargo tree -p <crate> -e features`, `python3 scripts/check-foundation-wire-deps.py --mode wire`, and deterministic source-audit artifacts from `python3 scripts/check-foundation-wire-source-audits.py --mode wire`, which must treat `tests/` trees and `#[cfg(test)]` code as test-only while proving `serde_json` stays test-only and forbidden production std/runtime dependencies stay out of the wire graph while the crates actually build in alloc-only target mode
- existing `aspen-core --no-default-features` dependency rails proving `redb` / `libc` are gone, using the approved transitive allowlist at `scripts/aspen-core-no-std-transitives.txt`
- `python3 scripts/check-aspen-core-no-std-surface.py` and `python3 scripts/check-aspen-core-no-std-boundary.py` as the boundary-proof rails, producing inventory/export-map/source-audit outputs plus dependency full-graph / allowlist outputs
- representative shell/runtime consumers (`aspen-cluster`, `aspen-client`, `aspen-cli`, `aspen-rpc-handlers`, the root `aspen` bundle via `cargo check -p aspen --no-default-features --features node-runtime`, and `aspen-core-shell`), plus `cargo check --manifest-path crates/aspen-core/tests/fixtures/shell-alias-smoke/Cargo.toml` for the dedicated shell-path smoke fixture whose manifest uses `aspen-core = { package = "aspen-core-shell", ... }` and imports `aspen_core::storage::SM_KV_TABLE`
- boundary inventory artifacts that explicitly enumerate the exact alloc-only families (`circuit_breaker`, `cluster`, `constants`, `crypto`, `error`, `hlc`, `kv`, alloc-safe `prelude`, `protocol`, `spec`, alloc-safe `storage`, `traits`, `types`, `vault`, `verified`, optional alloc-safe `sql`) and the exact shell families (`app_registry`, `context`, `simulation`, `transport`, Redb-specific storage helpers, `utils`, `test_support`, runtime-only prelude additions, duration convenience exports, plus `layer` and `global-discovery`) named in the core spec
- `cargo test -p aspen-core --test ui` as the deterministic compile-fail rail, with trybuild fixtures for alloc-only access to `AppRegistry`, `NetworkTransport`, `SimulationArtifact`, `ContentDiscovery`, `DirectoryLayer`, and `SM_KV_TABLE`, plus saved stderr snapshots and a fixture index
- `cargo test -p aspen-client-api` plus `cargo test -p aspen-client-api client_rpc_postcard_baseline -- --nocapture` as the deterministic wire-compatibility rails, saving alloc-safe serializer proof, default-feature regression output, and a saved baseline artifact `openspec/changes/extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json` that contains at least one variant-keyed default-production encoding for every `ClientRpcRequest` / `ClientRpcResponse` enum variant, derives canonical payloads from fixed rules only, deterministically enumerates the live variant set, and fails if any current variant lacks a baseline entry before comparing current request/response encodings against reviewed golden bytes

All of these rails save artifacts under `openspec/changes/extend-no-std-foundation-and-wire/evidence/`, including a dedicated docs review artifact `openspec/changes/extend-no-std-foundation-and-wire/evidence/docs-no-std-core-review.md`, and `verification.md` is the required claim-to-artifact index for the spec scenarios changed here. Verification assumes the `wasm32-unknown-unknown` target is available; if it is missing, install it via `rustup target add wasm32-unknown-unknown` or record the equivalent environment-provided setup in `verification.md` before running the target-build rails.

**Rationale:** this seam can regress silently if only one crate is checked. The affected surface spans leaf crates, public wire crates, shell consumers, and the previously established `aspen-core` feature/boundary contract.

### 6. Fresh evidence for this change is mandatory

**Choice:** only artifacts generated under `openspec/changes/extend-no-std-foundation-and-wire/evidence/` and indexed from this change's `verification.md` satisfy this change's verification claims. Archived evidence from earlier no-std changes may be cited as baseline context, but never as final proof for this change.

**Rationale:** this seam extends prior boundary work, so old artifacts are useful history but not sufficient evidence. Review must be able to distinguish fresh proof for the new storage/trait/wire seams from earlier `aspen-core`-only proof.

## Risks / Trade-offs

- **Shell storage path confusion** → document clearly that alloc-only `aspen-core::storage` no longer includes Redb table definitions, while shell consumers keep the path through `aspen-core-shell` aliasing.
- **Wide consumer churn from `aspen-traits` / storage changes** → stage the work by updating re-exports first, then leaf consumers, and keep representative compile rails running before and after each core edit.
- **Wire compatibility regression** → treat postcard discriminants as append-only compatibility contracts and add snapshot / roundtrip regression checks during the refactor.
- **Over-scoping into `aspen-raft-types`** → keep Iroh transport-type cleanup explicitly out of this change.

## Migration Plan

1. Record baseline compile and dependency rails for `aspen-core`, `aspen-core-no-std-smoke`, `aspen-storage-types`, `aspen-traits`, `aspen-client-api`, and the protocol crates.
2. Move Redb table definitions to the shell side and update `aspen-core` / `aspen-core-shell` storage re-exports.
3. Convert `aspen-traits` to alloc-safe `Arc` usage and rerun foundation rails.
4. Convert `aspen-client-api` and protocol crates to alloc-first production code while preserving feature defaults and discriminant order.
5. Update downstream consumers and `docs/no-std-core.md` so the visible contract explicitly documents what alloc-only `aspen-core` provides, what `aspen-core-shell` default / `layer` / `global-discovery` / shell `sql` provide, how shell consumers retain `aspen_core::storage::SM_KV_TABLE` through the `aspen-core-shell` alias path, and how the exact module-family inventory from the core spec maps onto the saved boundary artifacts.
6. Re-run positive and negative verification rails, save durable evidence under `openspec/changes/extend-no-std-foundation-and-wire/evidence/`, and map each changed scenario to concrete artifact paths in `verification.md`.

Rollback is straightforward at each step because the changes are crate-local seam moves: restore the previous re-export or dependency if a downstream compatibility issue appears.

## Open Questions

- None for the planning stage. If `SM_KV_TABLE` grows into a larger Redb helper surface during implementation, a dedicated shell crate can be reconsidered without changing the alloc-only contract defined here.
