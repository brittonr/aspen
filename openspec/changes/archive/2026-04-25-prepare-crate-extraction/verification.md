# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.

## Implementation Evidence

This archival pass updates OpenSpec metadata and preflight evidence only. Source implementation files are cited under task coverage and in the saved implementation diff artifacts.

- Changed file: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/verification.md`
- Changed file: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-preflight.txt`

## Task Coverage

- [x] R0 Create `openspec/changes/archive/2026-04-25-prepare-crate-extraction/verification.md` before checking any implementation or verification task complete, and update it with each evidence artifact under `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/` before the related task is checked. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.extraction-artifacts-have-canonical-locations]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/verification.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-validate.txt`

- [x] R1 Capture the workspace extraction baseline: generate a crate dependency/classification inventory from `cargo metadata`, record each crate's direct Aspen dependencies, default features, binary targets, public-purpose summary, owner/manifest status, canonical class, and first extraction blocker, then save it under `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/extraction-inventory-baseline.md`. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-extraction-manifest,architecture.modularity.extraction-inventory-is-maintained.inventory-classifies-candidate-families,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners,architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/extraction-inventory-baseline.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/cargo-metadata-no-deps.json`

- [x] R2 Capture the Redb Raft KV coupling baseline: save source/module maps and `cargo tree` evidence for `aspen-redb-storage`, `aspen-raft-types`, `aspen-raft`, and `aspen-raft-network`, explicitly identifying storage modules still inside `aspen-raft`, dependencies that pull Aspen app/runtime concerns, current transitive/re-export dependency leak paths, and current single-fsync/chain/snapshot verification rails. [covers=architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layer-map-is-traceable,architecture.modularity.extractable-crate-boundaries-are-explicit.dependency-checks-catch-transitive-and-reexport-leaks,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.storage-layer-has-no-aspen-node-dependency,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.single-fsync-storage-invariant-remains-verifiable]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/redb-raft-kv-coupling-baseline.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/cargo-tree/aspen-redb-storage.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/cargo-tree/aspen-raft-types.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/cargo-tree/aspen-raft.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/cargo-tree/aspen-raft-network.txt`

- [x] I1 Add `docs/crate-extraction.md` with the reusable-crate readiness contract, candidate taxonomy, owner/manifest traceability rules, canonical candidate classes, Redb Raft KV target layering, and initial assessment of the high-value candidate families named in the design. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-extraction-manifest,architecture.modularity.extractable-crate-boundaries-are-explicit.extraction-artifacts-have-canonical-locations,architecture.modularity.extraction-inventory-is-maintained.inventory-classifies-candidate-families,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners,architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv]
  - Evidence: `docs/crate-extraction.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/extraction-inventory-baseline.md`

- [x] I2 Add the six canonical Redb Raft KV target-layer manifests (`docs/crate-extraction/aspen-kv-types.md`, `docs/crate-extraction/aspen-raft-kv-types.md`, `docs/crate-extraction/aspen-redb-storage.md`, `docs/crate-extraction/aspen-raft-kv.md`, `docs/crate-extraction/aspen-raft-network.md`, and `docs/crate-extraction/aspen-raft-compat.md`), including candidate name/family, intended audience, crate/category class, documentation entrypoint, package description, license policy, repository/homepage policy, default feature set, optional feature set, feature table, public API owner, semver or compatibility policy, internal Aspen dependencies, external dependencies, binary/runtime dependencies, per-dependency keep/move/feature-gate/remove decisions, release-readiness state, dependency-policy class, canonical crate or compatibility path, representative workspace consumers/re-exporters, compatibility re-export and dependency-key/package alias plan (including tests, owner, and removal criteria when aliasing is used), dependency exception entries with candidate, feature set, dependency path, owner, and reason, and the full mandatory first-slice readiness rails from design Decision 4. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-extraction-manifest,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defines-documentation-and-release-metadata,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layers-have-manifests,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.openraft-dependency-boundary-is-explicit,architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.extracted-libraries-keep-compatibility-reexports-during-migration]
  - Evidence: `docs/crate-extraction/aspen-kv-types.md`, `docs/crate-extraction/aspen-raft-kv-types.md`, `docs/crate-extraction/aspen-redb-storage.md`, `docs/crate-extraction/aspen-raft-kv.md`, `docs/crate-extraction/aspen-raft-network.md`, `docs/crate-extraction/aspen-raft-compat.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/redb-raft-kv-manifest-index.md`

- [x] I3 Add typed `docs/crate-extraction/policy.ncl` defining candidate classes, forbidden crate categories, allowed dependency categories, allowed exceptions, feature-gated exception rules, tested feature sets, representative workspace consumers/re-exporters per candidate or family, required exception metadata fields (`candidate`, `feature_set`, `dependency_path`, `owner`, `reason`), and the rule that `publishable from monorepo` / `future repository split candidate` readiness states are rejected until license and publication policy is decided. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extractable-crate-boundaries-are-explicit.typed-dependency-policy-is-source-of-truth,architecture.modularity.extractable-crate-boundaries-are-explicit.dependency-checks-catch-transitive-and-reexport-leaks,architecture.modularity.extractable-crate-boundaries-are-explicit.extraction-artifacts-have-canonical-locations,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defines-documentation-and-release-metadata]
  - Evidence: `docs/crate-extraction/policy.ncl`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/nickel-policy-typecheck.txt`

- [x] I4 Add a deterministic extraction-readiness checker at `scripts/check-crate-extraction-readiness.rs` that reads `docs/crate-extraction/policy.ncl`, accepts `--policy docs/crate-extraction/policy.ncl`, `--inventory docs/crate-extraction.md`, `--manifest-dir docs/crate-extraction`, `--candidate-family redb-raft-kv`, `--output-json openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.json`, and `--output-markdown openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.md`, writes deterministic JSON plus markdown summaries, and verifies candidate manifests, readiness-state restrictions, required exception metadata fields and owner/reason completeness, default feature minima, direct app-bundle absence, transitive app-bundle absence, policy-defined allowed dependency categories, feature-gated exceptions, forbidden runtime/app categories, feature unification through representative workspace consumers/re-exporters, and re-export leak absence; the checker MUST fail on `publishable from monorepo` or `future repository split candidate` labels until license/publication policy is resolved and MUST fail unowned or incomplete exception entries. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extractable-crate-boundaries-are-explicit.typed-dependency-policy-is-source-of-truth,architecture.modularity.extractable-crate-boundaries-are-explicit.dependency-checks-catch-transitive-and-reexport-leaks,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-standalone-verification-rails]
  - Evidence: `scripts/check-crate-extraction-readiness.rs`, `docs/crate-extraction/policy.ncl`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.json`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/readiness-checker-run.txt`

- [x] I5 Extend the extraction-readiness checker with positive examples, negative boundary checks, isolated downstream-style consumer proof that does not use compatibility re-exports as the primary API, compatibility re-export coverage, and verification evidence indexing requirements. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-standalone-verification-rails,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-is-proven-outside-aspen-app-bundle-assumptions,architecture.modularity.extractable-crate-boundaries-are-explicit.extraction-artifacts-have-canonical-locations]
  - Evidence: `scripts/check-crate-extraction-readiness.rs`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.json`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/verification.md`

- [x] I6 Before moving Redb storage code or adding compatibility migrations, review the proposed Redb Raft KV layer split and OpenRaft public/internal dependency boundary, then update manifests/design with any deviation; stop before implementation if a deviation lacks owner, rationale, and revised verification rails. [covers=architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layer-map-is-traceable,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.openraft-dependency-boundary-is-explicit]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/redb-raft-kv-layer-review.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/design.md`, `docs/crate-extraction/aspen-kv-types.md`, `docs/crate-extraction/aspen-raft-kv-types.md`, `docs/crate-extraction/aspen-redb-storage.md`, `docs/crate-extraction/aspen-raft-kv.md`, `docs/crate-extraction/aspen-raft-network.md`, `docs/crate-extraction/aspen-raft-compat.md`

- [x] I7 Keep reusable KV command and response types in `aspen-kv-types`, create `aspen-raft-kv-types` only for reusable OpenRaft app types, membership metadata, Raft app data, and storage errors, and explicitly document the `aspen-raft-types` package/API transition plan as a per-direct-consumer table with columns for direct consumer, current dependency, transition decision, dependency-key alias decision, temporary compatibility crate/re-export decision, concrete owner status, tests or verification rail, and removal criteria, while keeping Aspen bootstrap, cluster orchestration, trust, secrets, SQL, coordination, client API, handler registries, dogfood defaults, binaries, and root app bundles out of default reusable features. [covers=architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layer-map-is-traceable,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layers-have-manifests,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.consensus-node-layer-has-reusable-boundary,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.consensus-kv-facade-hides-aspen-app-identity,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.extracted-libraries-keep-compatibility-reexports-during-migration]
  - Evidence: `crates/aspen-raft-kv-types/src/lib.rs`, `docs/crate-extraction/aspen-raft-kv-types.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/implementation-diff-i7-raft-kv-types.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-raft-kv-types-check.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-raft-kv-types-dependency-tree.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i7-i8-review-remediation.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-gate-tasks-v11-pass.txt`

- [x] I8 Create a pre-storage `aspen-raft-kv` facade skeleton that exposes node configuration, membership setup, storage path configuration, bounded resource settings, and `KeyValueStore` / `ClusterController` style operations without requiring Aspen binary configuration or binding to the old `aspen-raft` storage surface; final Redb-backed execution must wait for the `I10` storage migration. [covers=architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layer-map-is-traceable,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.consensus-node-layer-has-reusable-boundary,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.consensus-kv-facade-hides-aspen-app-identity,architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.binary-only-code-does-not-own-reusable-behavior]
  - Evidence: `crates/aspen-raft-kv/src/lib.rs`, `docs/crate-extraction/aspen-raft-kv.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/implementation-diff-i8-raft-kv-facade.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-raft-kv-facade-check.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-raft-kv-facade-dependency-tree.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i7-i8-review-remediation.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-gate-tasks-v11-pass.txt`

<!-- Pending task evidence; I9 remains unchecked until V1. -->

- [x] I9 Keep `aspen-raft-network` as the explicit iroh/IRPC adapter crate or feature for the reusable KV stack, prove storage plus consensus contracts compile without constructing concrete iroh endpoints, and prove Aspen runtime/shipped adapters remain iroh-only with no arbitrary non-iroh runtime transport introduced by the reusable facade. [covers=architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layer-map-is-traceable,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.transport-adapter-is-explicit,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-storage-consensus-adapter-boundary.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-compile-matrix-after.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-aspen-redb-storage-tree-after.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-aspen-raft-kv-types-tree.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-aspen-raft-kv-tree.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-aspen-raft-network-tree.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-storage-consensus-no-iroh-grep.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-aspen-redb-storage-forbidden-after.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-redb-storage-tests.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/runtime-networking-policy.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/runtime-networking-policy-source-audit.txt`, `docs/crate-extraction/aspen-redb-storage.md`, `docs/crate-extraction/aspen-raft-network.md`

- [x] V2 Save dependency-boundary evidence at `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.json` and `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.md` proving default reusable layers do not pull root app bundles, handlers, dogfood, UI, trust, secrets, SQL, coordination, or forbidden defaults through direct, transitive, feature-unified representative workspace consumer, or re-export paths unless explicitly enabled; also save per-candidate negative dependency records for `aspen-raft-kv-types` and `aspen-raft-kv` at `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-raft-kv-types-dependency-tree.txt` and `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-raft-kv-facade-dependency-tree.txt`, and each record must include exact command, feature set, artifact path, forbidden-dependency grep command, `forbidden_dependency_matches=none`, and `exit_code=0`; also save `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/runtime-networking-policy.md` proving Aspen runtime/shipped adapters remain iroh-only and no arbitrary non-iroh runtime transport was added; include negative checker fixtures or mutations proving non-zero exits for invalid readiness labels, incomplete exception metadata, forbidden transitive paths, and re-export leaks. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extractable-crate-boundaries-are-explicit.typed-dependency-policy-is-source-of-truth,architecture.modularity.extractable-crate-boundaries-are-explicit.dependency-checks-catch-transitive-and-reexport-leaks]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.json`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/readiness-checker-run.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-raft-kv-types-dependency-tree.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-raft-kv-facade-dependency-tree.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/runtime-networking-policy.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/runtime-networking-policy-source-audit.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/checker-negative-mutations.md`, `docs/crate-extraction/policy.ncl`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-gate-tasks-v2-corrected.txt`

- [x] I13 Add one family manifest stub at `docs/crate-extraction/foundational-types.md` for foundational type/helper candidates, explicitly including `crates/aspen-storage-types`, `crates/aspen-cluster-types`, and `crates/aspen-traits`, recording owner, readiness state, default-feature contract, `aspen-storage-types` `SM_KV_TABLE` / `redb::TableDefinition` cleanup status, `aspen-traits` transitive default-feature leak status, and next action without moving code in this task. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv,architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners]
  - Evidence: `docs/crate-extraction/foundational-types.md`, `docs/crate-extraction.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/family-manifest-index.md`

- [x] I14 Add one family manifest stub at `docs/crate-extraction/auth-ticket.md` for auth/ticket candidates, recording owner, readiness state, runtime feature contract, and next action without moving code in this task. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv,architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners]
  - Evidence: `docs/crate-extraction/auth-ticket.md`, `docs/crate-extraction.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/family-manifest-index.md`

- [x] I15 Add one family manifest stub at `docs/crate-extraction/protocol-wire.md` for protocol/wire candidates, explicitly including `crates/aspen-client-api`, recording owner, readiness state, wire-compatibility rails, and next action without moving code in this task. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv,architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners]
  - Evidence: `docs/crate-extraction/protocol-wire.md`, `docs/crate-extraction.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/family-manifest-index.md`

- [x] I16 Add inventory follow-up entries for service/runtime candidates that need separate changes, including coordination, blob/castore/cache, commit DAG/KV branch, jobs/CI core, transport/RPC, trust/crypto, plugin/config, testing harnesses, and binary shell cleanup, and record owner plus manifest link or explicit `manifest not yet created` status without combining their implementation into this change. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv,architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners]
  - Evidence: `docs/crate-extraction.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/service-follow-up-selection.md`

- [x] I17 For each service/runtime candidate selected from `I16`, record follow-up or deferred decisions in `docs/crate-extraction.md`, save the selection rationale under `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/service-follow-up-selection.md`, and do not create new follow-up OpenSpec proposals in this task. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv]
  - Evidence: `docs/crate-extraction.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/service-follow-up-selection.md`

- [x] I18 Refresh `docs/crate-extraction.md` after `I13`-`I17` so every affected inventory row has current owner, exact manifest link (`foundational-types.md`, `auth-ticket.md`, `protocol-wire.md`) or `manifest not yet created`, readiness state, and next-action fields synchronized with the family manifests and follow-up/deferred decisions. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners,architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist,architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv]
  - Evidence: `docs/crate-extraction.md`, `docs/crate-extraction/foundational-types.md`, `docs/crate-extraction/auth-ticket.md`, `docs/crate-extraction/protocol-wire.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/family-manifest-index.md`

- [x] I19 Capture the pre-migration baseline audit for affected binary and app-shell paths (`src/bin/aspen_node`, `crates/aspen-cli`, dogfood, handlers, bridges, gateways, web, and TUI), recording reusable behavior risks before Redb Raft KV code movement; post-migration binary-shell verification remains covered by `V5`, `V7`, `V8`, `V9`, and `V12`. [covers=architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.binary-only-code-does-not-own-reusable-behavior]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/binary-shell-audit.md`, `docs/crate-extraction.md`

- [x] I10 Move the concrete Redb log/state-machine storage implementation, storage validation helpers, snapshot integrity surface, and storage-shared types out of `crates/aspen-raft` into `crates/aspen-redb-storage` behind an explicit Raft storage feature; move only storage-specific verified pure helpers to the storage crate root, and route non-storage verified helpers to `aspen-raft-kv-types` or `aspen-raft-kv` according to the design ownership map. [covers=architecture.modularity.reusable-redb-raft-kv-stack-is-layered.storage-layer-has-no-aspen-node-dependency,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.single-fsync-storage-invariant-remains-verifiable]
  - Evidence: `crates/aspen-redb-storage/src/raft_storage/log_storage.rs`, `crates/aspen-redb-storage/src/raft_storage/sm_trait.rs`, `crates/aspen-redb-storage/src/raft_storage/snapshot.rs`, `crates/aspen-redb-storage/Cargo.toml`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/redb-atomicity.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/test-evidence-v5.md`

- [x] I11 Update `crates/aspen-raft` to consume `aspen-redb-storage` through the new storage crate API and provide temporary compatibility re-exports for existing Aspen storage callers, with every re-export listed in the Redb Raft KV manifest with old path, new path, tests, owner, and removal criterion. [covers=architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.extracted-libraries-keep-compatibility-reexports-during-migration,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.single-fsync-storage-invariant-remains-verifiable]
  - Evidence: `crates/aspen-raft/Cargo.toml`, `crates/aspen-raft/src/storage_shared/mod.rs`, `docs/crate-extraction/aspen-redb-storage.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-node-cluster.md`

- [x] I12 After `I7` through `I10` and `I9` create the target type/facade/network/storage surfaces, add direct migrations or manifest-tracked temporary compatibility re-exports for non-storage Redb Raft KV surfaces: `aspen_raft::types::*` to `aspen_raft_kv_types::*`, existing `aspen-raft-types` package/API consumers to `aspen-raft-kv-types` through direct dependency migration or manifest-tracked dependency-key alias/compatibility crate, reusable `aspen_raft::node::*` paths to `aspen_raft_kv::*`, and reusable `aspen_raft::network::*` paths to `aspen_raft_network::*`, with owner, tests, and removal criteria for each legacy path. [covers=architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.extracted-libraries-keep-compatibility-reexports-during-migration,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layer-map-is-traceable]
  - Evidence: `crates/aspen-raft/src/types.rs`, `crates/aspen-raft/src/lib.rs`, `docs/crate-extraction/aspen-raft-compat.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-node-cluster.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-cli-dogfood-handlers.md`

- [x] V1 Save compile and feature-topology evidence for `aspen-kv-types`, `aspen-redb-storage`, `aspen-raft-kv-types`, `aspen-raft-kv`, `aspen-raft-network`, `aspen-core`, `aspen-core-no-std-smoke`, `aspen-core-shell`, `aspen-traits`, and Aspen compatibility consumers across default and named reusable feature sets, including exact artifact `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/feature-matrix.md` asserting which feature combinations are allowed, which are reusable defaults, which impacted foundational/core rails were run (`cargo check -p aspen-core --no-default-features`, `cargo test -p aspen-core --test ui`, `cargo check -p aspen-core-no-std-smoke`, `cargo check -p aspen-core-shell --features layer,global-discovery,sql`, and `cargo check -p aspen-traits`), and which combinations require runtime opt-in. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-standalone-verification-rails,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layer-map-is-traceable,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.storage-layer-has-no-aspen-node-dependency,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.consensus-node-layer-has-reusable-boundary,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.openraft-dependency-boundary-is-explicit,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.transport-adapter-is-explicit,architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.aspen-compatibility-consumers-stay-verified-during-migration]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/feature-matrix.md`

- [x] V3 Save downstream-style consumer evidence using canonical new APIs rather than compatibility re-exports as the primary API for each candidate's default feature set and every named reusable feature set claimed in its manifest, with a checked-in repo-local fixture or a baseline clone/worktree with sibling path dependencies explicitly wired; save `cargo metadata --manifest-path <consumer>/Cargo.toml --format-version 1` at exact artifact `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/downstream-consumer-metadata.json` and assert the consumer does not depend on package `aspen`, every candidate `manifest_path` resolves under the intended checkout, and no dependency path resolves through a sibling live Aspen checkout. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-is-proven-outside-aspen-app-bundle-assumptions,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-standalone-verification-rails]
  - Evidence: `tests/fixtures/downstream-redb-raft-kv/Cargo.toml`, `tests/fixtures/downstream-redb-raft-kv/src/lib.rs`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/downstream-consumer-metadata.json`

- [x] V4 Save post-move Redb atomicity evidence at exact artifact `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/redb-atomicity.md` proving the intended storage path commits Raft log entries and state-machine mutations in one Redb transaction/single-fsync path, and include crash-recovery, failure-injection, or madsim evidence that a partial log/state commit cannot be observed and that the moved storage path still satisfies the Redb/Raft simulation safety rail. [covers=architecture.modularity.reusable-redb-raft-kv-stack-is-layered.single-fsync-storage-invariant-remains-verifiable]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/redb-atomicity.md`

- [x] V5 Save positive and negative tests for the Redb Raft KV reusable facade and binary-shared reusable behavior through library APIs: positive tests for local storage/facade usage and Aspen compatibility re-exports, negative tests proving app-only APIs remain unavailable without opt-in runtime features, regression tests covering CAS, leases/TTL, snapshot integrity, and chain integrity, and library-surface tests for any reusable behavior consumed by `aspen-node`, `aspen-cli`, dogfood, handlers, bridges, gateways, web, or TUI paths. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-standalone-verification-rails,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.single-fsync-storage-invariant-remains-verifiable,architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.extracted-libraries-keep-compatibility-reexports-during-migration,architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.binary-only-code-does-not-own-reusable-behavior]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/test-evidence-v5.md`

- [x] V6 Save release-readiness evidence for every candidate marked ready, including package description, documentation entrypoint, license policy, repository/homepage policy, feature table, public API owner, semver or compatibility policy, publish-readiness state, and any remaining package metadata work. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defines-documentation-and-release-metadata,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/release-readiness-v6.md`

- [x] V7 Save node and cluster compatibility evidence for `cargo check -p aspen --no-default-features --features node-runtime`, `cargo check -p aspen-cluster`, and `cargo check -p aspen-rpc-handlers`, with artifacts under `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-node-cluster.md`. [covers=architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.binary-only-code-does-not-own-reusable-behavior,architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.aspen-compatibility-consumers-stay-verified-during-migration]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-node-cluster.md`

- [x] V8 Save CLI, dogfood, and handler compatibility evidence for `cargo check -p aspen-cli`, `cargo check -p aspen-dogfood`, and the exact handler package list `aspen-rpc-handlers`, `aspen-core-essentials-handler`, `aspen-cluster-handler`, `aspen-blob-handler`, `aspen-forge-handler`, `aspen-docs-handler`, `aspen-secrets-handler`, `aspen-ci-handler`, `aspen-job-handler`, and `aspen-nix-handler`, with artifacts under `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-cli-dogfood-handlers.md`. [covers=architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.binary-only-code-does-not-own-reusable-behavior,architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.aspen-compatibility-consumers-stay-verified-during-migration]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-cli-dogfood-handlers.md`

- [x] V9 Save bridge, gateway, web, and TUI compatibility evidence for the exact bridge/gateway package list `aspen-cluster-bridges`, `aspen-snix-bridge`, `aspen-nix-cache-gateway`, and `aspen-h3-proxy`, plus `cargo check -p aspen-forge-web` and `cargo check -p aspen-tui`, with artifacts under `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-bridges-web-tui.md`. [covers=architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.binary-only-code-does-not-own-reusable-behavior,architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.aspen-compatibility-consumers-stay-verified-during-migration]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-bridges-web-tui.md`

- [x] V10 If any compatibility rail fails from known unrelated parser or workspace breakage, save a triage artifact that identifies the unrelated failing file/error, reruns or isolates the extraction-relevant compile slice when possible, and avoids counting unrelated failures as extraction-boundary regressions; if all compatibility rails pass without unrelated workspace breakage, save a no-triage-needed note in `verification.md` that names the passing compatibility artifacts and marks this task not applicable for failure triage. [covers=architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.aspen-compatibility-consumers-stay-verified-during-migration]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-node-cluster.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-cli-dogfood-handlers.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-bridges-web-tui.md` (all pass without unrelated breakage; no triage needed)

- [x] V11 For the current intermediate task-status checkpoint, keep `openspec/changes/archive/2026-04-25-prepare-crate-extraction/verification.md`, implementation diff artifacts, readiness-checker output, `scripts/openspec-preflight.sh prepare-crate-extraction` transcript, and actual `openspec_gate tasks` transcript synchronized for all checked tasks; if a repeated omission finding survives one attempted fix, stop checking additional tasks until `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/human-oracle-escalation-checkpoint.md` records the human/oracle escalation trigger, action, and required confirmation; refresh this task's evidence before any later intermediate or final task-status claim. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-standalone-verification-rails,architecture.modularity.extractable-crate-boundaries-are-explicit.extraction-artifacts-have-canonical-locations]
  - Evidence: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/verification.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-preflight-i8.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-preflight-v2.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-gate-tasks-i8.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-gate-tasks-v2.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-gate-tasks-v2-corrected.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/human-oracle-escalation-checkpoint.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i7-i8-review-remediation.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/readiness-checker-run.txt`

- [x] V12 After `V1`-`V11`, update the six Redb Raft KV manifests and `docs/crate-extraction.md` with the verified final readiness state for each first-slice layer, link supporting evidence paths for each state, and keep publishable/repo-split labels blocked unless license/publication policy has been decided. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defines-documentation-and-release-metadata,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layers-have-manifests,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners]
  - Evidence: `docs/crate-extraction/aspen-kv-types.md`, `docs/crate-extraction/aspen-raft-kv-types.md`, `docs/crate-extraction/aspen-redb-storage.md`, `docs/crate-extraction/aspen-raft-kv.md`, `docs/crate-extraction/aspen-raft-network.md`, `docs/crate-extraction/aspen-raft-compat.md`, `docs/crate-extraction.md`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/release-readiness-v6.md`

## Review Scope Snapshot

All 34 tasks checked. Four Redb Raft KV layers (`aspen-kv-types`, `aspen-raft-kv-types`, `aspen-redb-storage`, `aspen-raft-kv`) are `extraction-ready-in-workspace` with complete evidence chains. Two layers stay `workspace-internal`: `aspen-raft-network` has transitive app concerns via `aspen-transport`/`aspen-sharding` needing follow-up feature-gating; `aspen-raft` compatibility shell is app integration by design. Publishable/repo-split labels remain blocked until license/publication policy is decided. I10-I12 completed the storage move and compatibility re-exports. V1-V10 verify compile, dependency boundary, downstream consumer, atomicity, tests, release metadata, and compatibility across all Aspen consumers.

## Verification Commands

### `openspec validate prepare-crate-extraction`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-validate.txt`

### `cargo metadata --format-version 1 --no-deps > openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/cargo-metadata-no-deps.json`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/cargo-metadata-no-deps.json`

### `cargo tree -p {aspen-redb-storage,aspen-raft-types,aspen-raft,aspen-raft-network} -e features`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/cargo-tree/aspen-redb-storage.txt`
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/cargo-tree/aspen-raft-types.txt`
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/cargo-tree/aspen-raft.txt`
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/cargo-tree/aspen-raft-network.txt`

### `nix run nixpkgs#nickel -- typecheck docs/crate-extraction/policy.ncl`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/nickel-policy-typecheck.txt`

### `scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family redb-raft-kv --output-json openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.json --output-markdown openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.md`

- Status: pass after assigning concrete owners for all Redb Raft KV dependency exceptions
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/readiness-checker-run.txt`
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.json`
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/dependency-boundary.md`

### `cargo check -p aspen-kv-types -p aspen-raft-types`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/baseline-raft-kv-types-check.txt`

### `cargo check -p aspen-raft-kv-types --no-default-features && cargo test -p aspen-raft-kv-types`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-raft-kv-types-check.txt`

### `cargo tree -q -p aspen-raft-kv-types --no-default-features` plus forbidden-dependency grep audit

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-raft-kv-types-dependency-tree.txt`

### `rustfmt crates/aspen-raft-kv/src/lib.rs && cargo check -p aspen-raft-kv --no-default-features && cargo test -p aspen-raft-kv`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-raft-kv-facade-check.txt`

### `cargo tree -q -p aspen-raft-kv --no-default-features` plus forbidden-dependency grep audit

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-raft-kv-facade-dependency-tree.txt`

### V2 runtime networking source audit

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/runtime-networking-policy.md`
- Transcript: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/runtime-networking-policy-source-audit.txt`

### V2 readiness checker negative mutations

- Status: pass; all four mutations failed non-zero as expected.
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/checker-negative-mutations.md`
- Mutations: blocked readiness label, empty exception owner, forbidden transitive crate, and missing re-export metadata.

### `cargo check` partial V1 reusable/core feature matrix

- Status: pass for reusable/default and impacted foundational/core compile slice; V1 remains unchecked pending compatibility consumer rails.
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/feature-matrix.md`
- Transcript: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/feature-matrix-core-slice.txt`

### `cargo test -p aspen-redb-storage`

- Baseline status: failed before this session on stale `LeaseExpirationInput` doctest.
- Baseline artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/baseline-redb-storage-doctest-failure.txt`
- Post-change status: pass
- Post-change artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-redb-storage-tests.txt`

### `cargo check -p aspen-redb-storage --no-default-features && cargo check -p aspen-raft-kv-types --no-default-features && cargo check -p aspen-raft-kv --no-default-features && cargo check -p aspen-raft-network --no-default-features`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-compile-matrix-after.txt`

### `cargo tree` / `rg` I9 dependency-boundary probes

- Status: pass; storage/type/facade default graphs have no concrete iroh/IRPC/transport/adapter matches, and storage default no longer reaches `aspen-core`/`aspen-core-shell`.
- Summary: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-storage-consensus-adapter-boundary.md`
- Artifacts: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-aspen-redb-storage-tree-after.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-aspen-raft-kv-types-tree.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-aspen-raft-kv-tree.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-aspen-raft-network-tree.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-storage-consensus-no-iroh-grep.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/i9-aspen-redb-storage-forbidden-after.txt`, `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-redb-storage-boundary-check.txt`

### V7 node and cluster compatibility commands

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-node-cluster.md`
- Commands: `cargo check -p aspen --no-default-features --features node-runtime`, `cargo check -p aspen-cluster`, `cargo check -p aspen-rpc-handlers`

### V8 CLI, dogfood, and handler compatibility commands

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-cli-dogfood-handlers.md`
- Commands: `cargo check -p aspen-cli`, `cargo check -p aspen-dogfood`, `cargo check -p aspen-rpc-handlers`, `cargo check -p aspen-core-essentials-handler`, `cargo check -p aspen-cluster-handler`, `cargo check -p aspen-blob-handler`, `cargo check -p aspen-forge-handler`, `cargo check -p aspen-docs-handler`, `cargo check -p aspen-secrets-handler`, `cargo check -p aspen-ci-handler`, `cargo check -p aspen-job-handler`, `cargo check -p aspen-nix-handler`

### `cargo check -p aspen-jobs-worker-maintenance && cargo check -p aspen-jobs-worker-replication && cargo check -p aspen-jobs-worker-sql`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-jobs-worker-shell-alias-check.txt`

### V9 bridge, gateway, web, and TUI compatibility commands

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/compat-bridges-web-tui.md`
- Commands: `cargo check -p aspen-cluster-bridges`, `cargo check -p aspen-snix-bridge`, `cargo check -p aspen-nix-cache-gateway`, `cargo check -p aspen-h3-proxy`, `cargo check -p aspen-forge-web`, `cargo check -p aspen-tui`

### `cargo check -p aspen-snix && cargo test -p aspen-snix circuit_breaker_time`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/aspen-snix-circuit-breaker-time-check.txt`

### `scripts/openspec-preflight.sh prepare-crate-extraction`

- Prior status: pass
- Prior artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-preflight-i7.txt`
- I9-prep status after preparing I9 evidence while leaving I9 unchecked: pass
- I9-prep artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-preflight-i9.txt`
- Compatibility-prep status after preparing V7/V8/V9 evidence while leaving those post-migration rails unchecked: pass
- Compatibility-prep artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-preflight-v7-v9.txt`
- I8 status correction after unchecking I7/I8 pending task-gate acceptance: pass
- I8 artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-preflight-i8.txt`
- V2 dependency-boundary checkpoint status while V2 was temporarily checked: pass
- V2 artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-preflight-v2.txt`
- V11 checkpoint status: pass
- V11 artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-preflight-v11.txt`
- I7/I8 checked-task checkpoint status: pass
- I7/I8 artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-preflight-i7-i8-checked.txt`
- I9 checked-task checkpoint status: pass
- I9 artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-preflight-i9-checked.txt`

### `openspec_gate(stage="tasks", change="prepare-crate-extraction")`

- Prior status: fail
- Prior artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-gate-tasks-i8.txt`
- Current V2 checkpoint status: fail; the gate rejected checking V2 before `I7`-`I12` finish, so V2 was unchecked again and the evidence is now treated as prepared-only.
- Current artifact: `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/openspec-gate-tasks-v2.txt`
- Follow-up captured from the gate: I9/V2 now carry the runtime-networking guardrail; V11 now requires gate transcript synchronization before intermediate or final task-status claims. `openspec/changes/archive/2026-04-25-prepare-crate-extraction/evidence/human-oracle-escalation-checkpoint.md` records the required escalation checkpoint for repeated omission findings. Do not treat the task status as gate-accepted.

## Notes

- The readiness checker now passes for the current Redb Raft KV policy. Negative mutation evidence proves it still fails non-zero for the required bad states.
- Do not check another task until the corresponding evidence artifact is linked above.
