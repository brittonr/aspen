# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.
Do not rely on chat-only summaries, `/tmp` logs, or memory.

## Implementation Evidence

- Changed file: `Cargo.lock`
- Changed file: `crates/aspen-client/Cargo.toml`
- Changed file: `crates/aspen-cluster-bridges/Cargo.toml`
- Changed file: `crates/aspen-cluster-handler/Cargo.toml`
- Changed file: `crates/aspen-cluster/Cargo.toml`
- Changed file: `crates/aspen-core-essentials-handler/Cargo.toml`
- Changed file: `crates/aspen-docs-handler/Cargo.toml`
- Changed file: `crates/aspen-net/Cargo.toml`
- Changed file: `crates/aspen-nix-handler/Cargo.toml`
- Changed file: `crates/aspen-raft/Cargo.toml`
- Changed file: `crates/aspen-rpc-core/Cargo.toml`
- Changed file: `crates/aspen-rpc-core/src/context.rs`
- Changed file: `crates/aspen-rpc-core/src/context_runtime.rs`
- Changed file: `crates/aspen-rpc-core/src/lib.rs`
- Changed file: `crates/aspen-rpc-core/src/registry.rs`
- Changed file: `crates/aspen-rpc-handlers/Cargo.toml`
- Changed file: `crates/aspen-secrets-handler/Cargo.toml`
- Changed file: `crates/aspen-transport/Cargo.toml`
- Changed file: `crates/aspen-transport/src/constants.rs`
- Changed file: `crates/aspen-transport/src/lib.rs`
- Changed file: `docs/crate-extraction.md`
- Changed file: `docs/crate-extraction/policy.ncl`
- Changed file: `docs/crate-extraction/transport-rpc.md`
- Changed file: `scripts/check-crate-extraction-readiness.rs`
- Changed file: `openspec/changes/split-transport-rpc-core/fixtures/downstream-transport/Cargo.toml`
- Changed file: `openspec/changes/split-transport-rpc-core/fixtures/downstream-transport/src/lib.rs`
- Changed file: `openspec/changes/split-transport-rpc-core/fixtures/downstream-rpc-core/Cargo.toml`
- Changed file: `openspec/changes/split-transport-rpc-core/fixtures/downstream-rpc-core/src/lib.rs`
- Changed file: `openspec/changes/split-transport-rpc-core/evidence/implementation-diff.txt`
- Changed file: `openspec/changes/split-transport-rpc-core/evidence/run-verification.sh`
- Changed file: `openspec/changes/split-transport-rpc-core/evidence/v5-run-negative-mutations.sh`
- Changed file: `openspec/changes/split-transport-rpc-core/tasks.md`
- Changed file: `openspec/changes/split-transport-rpc-core/verification.md`

## Task Coverage

- [x] R1 Capture baseline compile, `cargo tree`, feature, and source-import evidence for `aspen-transport`, `aspen-rpc-core`, and representative consumers, classifying each dependency as generic transport, RPC core, domain context, adapter/runtime, test-only, or forbidden. [covers=transport-rpc-extraction.rpc-core-default-avoids-service-graph,transport-rpc-extraction.transport-default-avoids-runtime-shells,transport-rpc-extraction.transport-default-avoids-runtime-shells.iroh-adapter-is-documented-exception,transport-rpc-extraction.transport-default-avoids-runtime-shells.runtime-concerns-feature-gated,transport-rpc-extraction.rpc-core-default-avoids-service-graph.registry-compiles-without-concrete-contexts]
  - Evidence: `openspec/changes/split-transport-rpc-core/evidence/r1-baseline.md`, `openspec/changes/split-transport-rpc-core/evidence/r1-baseline-logs/transport-cargo-tree.txt`, `openspec/changes/split-transport-rpc-core/evidence/r1-baseline-logs/rpc-core-cargo-tree.txt`, `openspec/changes/split-transport-rpc-core/evidence/r1-baseline-logs/transport-source-imports.txt`, `openspec/changes/split-transport-rpc-core/evidence/r1-baseline-logs/rpc-core-source-imports.txt`
- [x] I1 Create `docs/crate-extraction/transport-rpc.md` with staged layer map, feature contracts, dependency exceptions, blocked reasons, compatibility plan, representative consumers, and verification rails. [covers=transport-rpc-extraction.inventory-and-policy,transport-rpc-extraction.inventory-and-policy.checker-verifies-staged-split]
  - Evidence: `docs/crate-extraction/transport-rpc.md`, `openspec/changes/split-transport-rpc-core/evidence/implementation-diff.txt`
- [x] I2 Add transport/RPC candidates to `docs/crate-extraction/policy.ncl` and update `docs/crate-extraction.md` with readiness state and next action. [covers=transport-rpc-extraction.inventory-and-policy.checker-verifies-staged-split]
  - Evidence: `docs/crate-extraction.md`, `docs/crate-extraction/policy.ncl`, `scripts/check-crate-extraction-readiness.rs`, `openspec/changes/split-transport-rpc-core/evidence/v5-readiness.md`
- [x] I3 Introduce or isolate the reusable transport surface so protocol identifiers, protocol handler traits, stream/connection helpers, and iroh/irpc adapter-purpose APIs compile without trust, sharding, auth runtime, Raft compatibility, handler registries, cluster bootstrap, or root Aspen by default. [covers=transport-rpc-extraction.transport-default-avoids-runtime-shells.iroh-adapter-is-documented-exception,transport-rpc-extraction.transport-default-avoids-runtime-shells.runtime-concerns-feature-gated]
  - Evidence: `crates/aspen-transport/Cargo.toml`, `crates/aspen-transport/src/lib.rs`, `crates/aspen-transport/src/constants.rs`, `openspec/changes/split-transport-rpc-core/evidence/v1-transport-default-cargo-check.txt`, `openspec/changes/split-transport-rpc-core/evidence/v1-transport-default-cargo-tree.txt`
- [x] I4 Move OpenRaft, sharding, trust, and auth runtime transport integration behind named features or adapter crates, preserving existing Aspen import paths through compatibility features where needed. [covers=transport-rpc-extraction.compatibility-adapters-preserve-runtime,transport-rpc-extraction.transport-default-avoids-runtime-shells.runtime-concerns-feature-gated,transport-rpc-extraction.compatibility-adapters-preserve-runtime.runtime-consumers-compile-through-features]
  - Evidence: `crates/aspen-transport/Cargo.toml`, `crates/aspen-raft/Cargo.toml`, `crates/aspen-cluster/Cargo.toml`, `crates/aspen-client/Cargo.toml`, `crates/aspen-cluster-bridges/Cargo.toml`, `openspec/changes/split-transport-rpc-core/evidence/v1-transport-runtime-cargo-check.txt`, `openspec/changes/split-transport-rpc-core/evidence/v4-compatibility-summary.txt`
- [x] I5 Introduce or isolate the reusable RPC core surface so handler registry/dispatch abstractions compile without concrete Raft, cluster, coordination, jobs, Forge, CI, hooks, blob, transport, testing, or root Aspen service contexts by default. [covers=transport-rpc-extraction.rpc-core-default-avoids-service-graph.registry-compiles-without-concrete-contexts]
  - Evidence: `crates/aspen-rpc-core/src/context.rs`, `crates/aspen-rpc-core/src/context_runtime.rs`, `crates/aspen-rpc-core/src/registry.rs`, `crates/aspen-rpc-core/src/lib.rs`, `openspec/changes/split-transport-rpc-core/evidence/v2-rpc-core-default-cargo-check.txt`, `openspec/changes/split-transport-rpc-core/evidence/v2-rpc-core-default-cargo-tree.txt`
- [x] I6 Move domain-specific RPC contexts and service bundles behind documented named features or adapter crates while preserving current Aspen runtime behavior. [covers=transport-rpc-extraction.rpc-core-default-avoids-service-graph.concrete-domains-are-explicit-opt-ins,transport-rpc-extraction.compatibility-adapters-preserve-runtime.runtime-consumers-compile-through-features]
  - Evidence: `crates/aspen-rpc-core/Cargo.toml`, `crates/aspen-rpc-core/src/context.rs`, `crates/aspen-rpc-core/src/context_runtime.rs`, `crates/aspen-rpc-handlers/Cargo.toml`, `crates/aspen-core-essentials-handler/Cargo.toml`, `crates/aspen-cluster-handler/Cargo.toml`, `openspec/changes/split-transport-rpc-core/evidence/v2-rpc-core-runtime-cargo-check.txt`, `openspec/changes/split-transport-rpc-core/evidence/v4-compatibility-summary.txt`
- [x] I7 Add downstream fixtures for generic transport protocol helpers and RPC handler registry usage without Aspen service contexts. [covers=transport-rpc-extraction.downstream-fixtures,transport-rpc-extraction.downstream-fixtures.transport-fixture-uses-generic-helpers,transport-rpc-extraction.downstream-fixtures.rpc-fixture-uses-registry-without-services]
  - Evidence: `openspec/changes/split-transport-rpc-core/fixtures/downstream-transport/Cargo.toml`, `openspec/changes/split-transport-rpc-core/fixtures/downstream-transport/src/lib.rs`, `openspec/changes/split-transport-rpc-core/fixtures/downstream-rpc-core/Cargo.toml`, `openspec/changes/split-transport-rpc-core/fixtures/downstream-rpc-core/src/lib.rs`, `openspec/changes/split-transport-rpc-core/evidence/i7-downstream-transport-tests.txt`, `openspec/changes/split-transport-rpc-core/evidence/i7-downstream-rpc-tests.txt`
- [x] V1 Save compile and dependency-boundary evidence for reusable transport feature sets, proving allowed iroh/irpc dependencies are documented and forbidden Aspen runtime shells are absent. [covers=transport-rpc-extraction.transport-default-avoids-runtime-shells.iroh-adapter-is-documented-exception,transport-rpc-extraction.transport-default-avoids-runtime-shells.runtime-concerns-feature-gated]
  - Evidence: `openspec/changes/split-transport-rpc-core/evidence/v1-transport-default-cargo-check.txt`, `openspec/changes/split-transport-rpc-core/evidence/v1-transport-runtime-cargo-check.txt`, `openspec/changes/split-transport-rpc-core/evidence/v1-transport-default-cargo-tree.txt`, `openspec/changes/split-transport-rpc-core/evidence/v1-transport-source-audit.txt`
- [x] V2 Save compile and dependency-boundary evidence for reusable RPC core feature sets, proving concrete service contexts and app shells are absent by default. [covers=transport-rpc-extraction.rpc-core-default-avoids-service-graph.registry-compiles-without-concrete-contexts,transport-rpc-extraction.rpc-core-default-avoids-service-graph.concrete-domains-are-explicit-opt-ins]
  - Evidence: `openspec/changes/split-transport-rpc-core/evidence/v2-rpc-core-default-cargo-check.txt`, `openspec/changes/split-transport-rpc-core/evidence/v2-rpc-core-runtime-cargo-check.txt`, `openspec/changes/split-transport-rpc-core/evidence/v2-rpc-core-default-cargo-tree.txt`, `openspec/changes/split-transport-rpc-core/evidence/v2-rpc-core-source-audit.txt`
- [x] V3 Save downstream fixture compile/metadata evidence for transport and RPC fixtures. [covers=transport-rpc-extraction.downstream-fixtures.transport-fixture-uses-generic-helpers,transport-rpc-extraction.downstream-fixtures.rpc-fixture-uses-registry-without-services]
  - Evidence: `openspec/changes/split-transport-rpc-core/evidence/i7-downstream-transport-tests.txt`, `openspec/changes/split-transport-rpc-core/evidence/i7-downstream-transport-metadata.json`, `openspec/changes/split-transport-rpc-core/evidence/i7-downstream-transport-forbidden-grep.txt`, `openspec/changes/split-transport-rpc-core/evidence/i7-downstream-rpc-tests.txt`, `openspec/changes/split-transport-rpc-core/evidence/i7-downstream-rpc-metadata.json`, `openspec/changes/split-transport-rpc-core/evidence/i7-downstream-rpc-forbidden-grep.txt`
- [x] V4 Save compatibility compile/test evidence for `aspen-raft-network`, `aspen-raft`, `aspen-cluster`, `aspen-client`, `aspen-rpc-handlers`, and root node runtime feature bundles. [covers=transport-rpc-extraction.compatibility-adapters-preserve-runtime.runtime-consumers-compile-through-features]
  - Evidence: `openspec/changes/split-transport-rpc-core/evidence/v4-compatibility-summary.txt`, `openspec/changes/split-transport-rpc-core/evidence/v4-root-node-runtime-lib-check.txt`, `openspec/changes/split-transport-rpc-core/evidence/v4-root-node-runtime-all-targets-preexisting-failure.txt`
- [x] V5 Run `scripts/check-crate-extraction-readiness.rs --candidate-family transport-rpc` and save output plus negative mutations for unowned runtime dependency, missing owner, invalid readiness state, missing downstream fixture, and missing compatibility evidence. [covers=transport-rpc-extraction.inventory-and-policy.checker-verifies-staged-split]
  - Evidence: `openspec/changes/split-transport-rpc-core/evidence/v5-readiness.md`, `openspec/changes/split-transport-rpc-core/evidence/v5-readiness.json`, `openspec/changes/split-transport-rpc-core/evidence/v5-run-negative-mutations.sh`, `openspec/changes/split-transport-rpc-core/evidence/v5-negative-unowned-runtime-summary.txt`, `openspec/changes/split-transport-rpc-core/evidence/v5-negative-missing-owner-summary.txt`, `openspec/changes/split-transport-rpc-core/evidence/v5-negative-invalid-readiness-summary.txt`, `openspec/changes/split-transport-rpc-core/evidence/v5-negative-missing-downstream-summary.txt`, `openspec/changes/split-transport-rpc-core/evidence/v5-negative-missing-compatibility-summary.txt`

## Review Scope Snapshot

### `git diff HEAD -- transport/RPC split implementation paths`

- Status: captured
- Artifact: `openspec/changes/split-transport-rpc-core/evidence/implementation-diff.txt`

## Verification Commands

### `openspec/changes/split-transport-rpc-core/evidence/r1-capture-baseline.sh`

- Status: pass
- Artifact: `openspec/changes/split-transport-rpc-core/evidence/r1-baseline.md`

### `openspec/changes/split-transport-rpc-core/evidence/run-verification.sh`

- Status: pass
- Artifact: `openspec/changes/split-transport-rpc-core/evidence/verification-summary.md`

### `cargo check -p aspen-raft-network && cargo check -p aspen-raft && cargo check -p aspen-cluster --no-default-features --features bootstrap && cargo check -p aspen-client && cargo check -p aspen-rpc-handlers`

- Status: pass
- Artifact: `openspec/changes/split-transport-rpc-core/evidence/v4-compatibility-summary.txt`

### `cargo check -p aspen --lib --no-default-features --features node-runtime`

- Status: pass
- Artifact: `openspec/changes/split-transport-rpc-core/evidence/v4-root-node-runtime-lib-check.txt`

### `cargo check -p aspen --all-targets --no-default-features --features node-runtime`

- Status: fail: pre-existing all-target test import errors unrelated to transport/RPC split; lib check above covers node-runtime bundle
- Artifact: `openspec/changes/split-transport-rpc-core/evidence/v4-root-node-runtime-all-targets-preexisting-failure.txt`

### `scripts/check-crate-extraction-readiness.rs --candidate-family transport-rpc`

- Status: pass
- Artifact: `openspec/changes/split-transport-rpc-core/evidence/v5-readiness.md`

### `openspec/changes/split-transport-rpc-core/evidence/v5-run-negative-mutations.sh`

- Status: pass
- Artifact: `openspec/changes/split-transport-rpc-core/evidence/v5-negative-mutations-summary.txt`

### `openspec validate split-transport-rpc-core --strict`

- Status: pass
- Artifact: `openspec/changes/split-transport-rpc-core/evidence/openspec-validate-strict.txt`

### `rustfmt --check transport/RPC touched Rust files`

- Status: pass
- Artifact: `openspec/changes/split-transport-rpc-core/evidence/rustfmt-check.txt`

### `scripts/openspec-preflight.sh split-transport-rpc-core`

- Status: pass
- Artifact: `openspec/changes/split-transport-rpc-core/evidence/v1-v5-openspec-preflight.txt`

### `cargo check -p aspen-rpc-core --all-features`

- Status: pass
- Artifact: `openspec/changes/split-transport-rpc-core/evidence/rpc-core-all-features-check.txt`

## Notes

- Root `--all-targets` node-runtime check still hits pre-existing test API import/type errors; the `--lib` node-runtime check passed after the split and the failing transcript is saved for review.