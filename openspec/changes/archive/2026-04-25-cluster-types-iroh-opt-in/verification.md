# Verification Evidence

This file maps the `cluster-types-iroh-opt-in` tasks to durable repo evidence.
Baseline context for the review failure is captured in `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/baseline-feature-leak.md`.

## Implementation Evidence

This archival pass updates OpenSpec metadata, delta-spec format, and preflight evidence only. Source implementation files are cited under task coverage and in the saved implementation diff artifact.

- Changed file: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/specs/architecture-modularity/spec.md`
- Changed file: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/verification.md`
- Changed file: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/openspec-preflight.txt`

## Task Coverage

- [x] I1 Record the current `aspen-cluster-types` feature leak and direct-consumer baseline, then remove implicit workspace-level `iroh` enablement and update every direct `aspen-cluster-types` dependency stanza so only crates that call runtime helper APIs opt into `features = ["iroh"]`. [covers=architecture.modularity.feature-bundles-are-explicit-and-bounded.cluster-types-iroh-opt-in-is-explicit]
  - Evidence: `Cargo.toml`, `crates/aspen-cluster-handler/Cargo.toml`, `crates/aspen-jobs/Cargo.toml`, `crates/aspen-raft/Cargo.toml`, `crates/aspen-testing-core/Cargo.toml`, `crates/aspen-testing-fixtures/Cargo.toml`, `crates/aspen-testing-madsim/Cargo.toml`, `crates/aspen-testing-patchbay/Cargo.toml`, `crates/aspen-testing/Cargo.toml`, `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/baseline-feature-leak.md`, `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/direct-consumer-audit.md`, `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/workspace-dependency-proof.txt`, `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/implementation-diff.txt`

- [x] I2 Keep `crates/aspen-cluster-types` alloc-safe by default (`default = []`, `#![cfg_attr(not(test), no_std)]`, alloc-safe error/dependency settings) while preserving runtime helper behavior behind the explicit `iroh` feature. [covers=architecture.modularity.acyclic-no-std-core-boundary.cluster-types-default-to-alloc-safe-builds,architecture.modularity.acyclic-no-std-core-boundary.cluster-types-verification-is-reviewable]
  - Evidence: `crates/aspen-cluster-types/Cargo.toml`, `crates/aspen-cluster-types/src/lib.rs`, `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/cluster-types-validation.md`, `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/implementation-diff.txt`

- [x] V1 Save reviewable evidence for the bare/default graph, explicit-`iroh` graph, the alloc-safe consumer-path proof for `cargo tree -p aspen-traits -e normal --depth 2`, the root-workspace stanza proof under `evidence/workspace-dependency-proof.txt`, the deterministic direct consumer audit, exact command/results for every direct consumer whose manifest classification changed in this seam, and `scripts/openspec-preflight.sh cluster-types-iroh-opt-in`, then map those artifacts in `verification.md`. [covers=architecture.modularity.acyclic-no-std-core-boundary.cluster-types-default-to-alloc-safe-builds,architecture.modularity.acyclic-no-std-core-boundary.cluster-types-verification-is-reviewable,architecture.modularity.feature-bundles-are-explicit-and-bounded.cluster-types-iroh-opt-in-is-explicit] [evidence=openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/cluster-types-validation.md]
  - Evidence: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/verification.md`, `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/cluster-types-validation.md`, `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/direct-consumer-audit.md`, `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/runtime-consumers.md`, `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/workspace-dependency-proof.txt`, `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/openspec-preflight.txt`, `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/implementation-diff.txt`

## Review Scope Snapshot

### `git diff --cached -- Cargo.toml crates/aspen-cluster-handler/Cargo.toml crates/aspen-cluster-types/Cargo.toml crates/aspen-cluster-types/src/lib.rs crates/aspen-jobs/Cargo.toml crates/aspen-raft/Cargo.toml crates/aspen-testing-core/Cargo.toml crates/aspen-testing-fixtures/Cargo.toml crates/aspen-testing-madsim/Cargo.toml crates/aspen-testing-patchbay/Cargo.toml crates/aspen-testing/Cargo.toml openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in`

- Status: captured
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/implementation-diff.txt`

## Verification Commands

### `cargo check -p aspen-cluster-types --no-default-features`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/cluster-types-validation.md`

### `cargo check -p aspen-cluster-types --no-default-features --target wasm32-unknown-unknown`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/cluster-types-validation.md`

### `cargo test -p aspen-cluster-types`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/cluster-types-validation.md`

### `cargo test -p aspen-cluster-types --features iroh`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/cluster-types-validation.md`

### `cargo tree -p aspen-cluster-types -e normal --depth 2`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/cluster-types-validation.md`

### `cargo tree -p aspen-cluster-types --features iroh -e normal --depth 2`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/cluster-types-validation.md`

### `cargo tree -p aspen-traits -e normal --depth 2`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/cluster-types-validation.md`

### `rg -n 'aspen-cluster-types\s*=\s*\{' . -g 'Cargo.toml'`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/direct-consumer-audit.md`

### `rg -n 'NodeAddress::new|ClusterNode::with_iroh_addr|\.iroh_addr\(|try_into_iroh\(' . -g '*.rs'`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/direct-consumer-audit.md`

### `cargo check -p aspen-jobs`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/runtime-consumers.md`

### `cargo check -p aspen-testing-core`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/runtime-consumers.md`

### `cargo check -p aspen-testing-fixtures`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/runtime-consumers.md`

### `cargo check -p aspen-testing`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/runtime-consumers.md`

### `cargo check -p aspen-testing-madsim`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/runtime-consumers.md`

### `cargo check -p aspen-raft`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/runtime-consumers.md`

### `cargo check -p aspen-cluster-handler --tests`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/runtime-consumers.md`

### `cargo check -p aspen-testing-patchbay`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/runtime-consumers.md`

### `scripts/openspec-preflight.sh cluster-types-iroh-opt-in`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/openspec-preflight.txt`

## Notes

- `openspec/changes/archive/2026-04-25-cluster-types-iroh-opt-in/evidence/baseline-feature-leak.md` records the original workspace-level failure mode that hid the leak.
- The bare/default proof for `aspen-cluster-types` is the tree in `cluster-types-validation.md`, not the earlier incorrect claim from chat.
