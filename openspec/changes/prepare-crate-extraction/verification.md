# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.

## Implementation Evidence

- Changed file: `Cargo.lock`
- Changed file: `crates/aspen-raft-kv/Cargo.toml`
- Changed file: `crates/aspen-raft-kv/src/lib.rs`
- Changed file: `docs/crate-extraction/aspen-raft-kv-types.md`
- Changed file: `docs/crate-extraction/aspen-raft-kv.md`
- Changed file: `openspec/changes/prepare-crate-extraction/tasks.md`
- Changed file: `openspec/changes/prepare-crate-extraction/verification.md`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/aspen-raft-kv-facade-check.txt`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/aspen-raft-kv-facade-dependency-tree.txt`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/aspen-raft-kv-types-check.txt`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/aspen-raft-kv-types-dependency-tree.txt`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/feature-matrix.md`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/implementation-diff-i7-raft-kv-types.txt`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/implementation-diff-i8-raft-kv-facade.txt`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/openspec-preflight-i8.txt`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/openspec-gate-tasks-i8.txt`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/human-oracle-escalation-checkpoint.md`

## Task Coverage

- [x] R0 Create `openspec/changes/prepare-crate-extraction/verification.md` before checking any implementation or verification task complete, and update it with each evidence artifact under `openspec/changes/prepare-crate-extraction/evidence/` before the related task is checked. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.extraction-artifacts-have-canonical-locations]
  - Evidence: `openspec/changes/prepare-crate-extraction/verification.md`, `openspec/changes/prepare-crate-extraction/evidence/openspec-validate.txt`

- [x] R1 Capture the workspace extraction baseline: generate a crate dependency/classification inventory from `cargo metadata`, record each crate's direct Aspen dependencies, default features, binary targets, public-purpose summary, owner/manifest status, canonical class, and first extraction blocker, then save it under `openspec/changes/prepare-crate-extraction/evidence/extraction-inventory-baseline.md`. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-extraction-manifest,architecture.modularity.extraction-inventory-is-maintained.inventory-classifies-candidate-families,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners,architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv]
  - Evidence: `openspec/changes/prepare-crate-extraction/evidence/extraction-inventory-baseline.md`, `openspec/changes/prepare-crate-extraction/evidence/cargo-metadata-no-deps.json`

- [x] R2 Capture the Redb Raft KV coupling baseline: save source/module maps and `cargo tree` evidence for `aspen-redb-storage`, `aspen-raft-types`, `aspen-raft`, and `aspen-raft-network`, explicitly identifying storage modules still inside `aspen-raft`, dependencies that pull Aspen app/runtime concerns, current transitive/re-export dependency leak paths, and current single-fsync/chain/snapshot verification rails. [covers=architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layer-map-is-traceable,architecture.modularity.extractable-crate-boundaries-are-explicit.dependency-checks-catch-transitive-and-reexport-leaks,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.storage-layer-has-no-aspen-node-dependency,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.single-fsync-storage-invariant-remains-verifiable]
  - Evidence: `openspec/changes/prepare-crate-extraction/evidence/redb-raft-kv-coupling-baseline.md`, `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-redb-storage.txt`, `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-raft-types.txt`, `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-raft.txt`, `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-raft-network.txt`

- [x] I1 Add `docs/crate-extraction.md` with the reusable-crate readiness contract, candidate taxonomy, owner/manifest traceability rules, canonical candidate classes, Redb Raft KV target layering, and initial assessment of the high-value candidate families named in the design. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-extraction-manifest,architecture.modularity.extractable-crate-boundaries-are-explicit.extraction-artifacts-have-canonical-locations,architecture.modularity.extraction-inventory-is-maintained.inventory-classifies-candidate-families,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners,architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv]
  - Evidence: `docs/crate-extraction.md`, `openspec/changes/prepare-crate-extraction/evidence/extraction-inventory-baseline.md`

- [x] I2 Add the six canonical Redb Raft KV target-layer manifests (`docs/crate-extraction/aspen-kv-types.md`, `docs/crate-extraction/aspen-raft-kv-types.md`, `docs/crate-extraction/aspen-redb-storage.md`, `docs/crate-extraction/aspen-raft-kv.md`, `docs/crate-extraction/aspen-raft-network.md`, and `docs/crate-extraction/aspen-raft-compat.md`), including candidate name/family, intended audience, crate/category class, documentation entrypoint, package description, license policy, repository/homepage policy, default feature set, optional feature set, feature table, public API owner, semver or compatibility policy, internal Aspen dependencies, external dependencies, binary/runtime dependencies, per-dependency keep/move/feature-gate/remove decisions, release-readiness state, dependency-policy class, canonical crate or compatibility path, representative workspace consumers/re-exporters, compatibility re-export and dependency-key/package alias plan (including tests, owner, and removal criteria when aliasing is used), dependency exception entries with candidate, feature set, dependency path, owner, and reason, and the full mandatory first-slice readiness rails from design Decision 4. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-extraction-manifest,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defines-documentation-and-release-metadata,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layers-have-manifests,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.openraft-dependency-boundary-is-explicit,architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.extracted-libraries-keep-compatibility-reexports-during-migration]
  - Evidence: `docs/crate-extraction/aspen-kv-types.md`, `docs/crate-extraction/aspen-raft-kv-types.md`, `docs/crate-extraction/aspen-redb-storage.md`, `docs/crate-extraction/aspen-raft-kv.md`, `docs/crate-extraction/aspen-raft-network.md`, `docs/crate-extraction/aspen-raft-compat.md`, `openspec/changes/prepare-crate-extraction/evidence/redb-raft-kv-manifest-index.md`

- [x] I3 Add typed `docs/crate-extraction/policy.ncl` defining candidate classes, forbidden crate categories, allowed exceptions, tested feature sets, representative workspace consumers/re-exporters per candidate or family, required exception metadata fields (`candidate`, `feature_set`, `dependency_path`, `owner`, `reason`), and the rule that `publishable from monorepo` / `future repository split candidate` readiness states are rejected until license and publication policy is decided. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extractable-crate-boundaries-are-explicit.typed-dependency-policy-is-source-of-truth,architecture.modularity.extractable-crate-boundaries-are-explicit.dependency-checks-catch-transitive-and-reexport-leaks,architecture.modularity.extractable-crate-boundaries-are-explicit.extraction-artifacts-have-canonical-locations,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defines-documentation-and-release-metadata]
  - Evidence: `docs/crate-extraction/policy.ncl`, `openspec/changes/prepare-crate-extraction/evidence/nickel-policy-typecheck.txt`

- [x] I4 Add a deterministic extraction-readiness checker at `scripts/check-crate-extraction-readiness.rs` that reads `docs/crate-extraction/policy.ncl`, accepts `--policy docs/crate-extraction/policy.ncl`, `--inventory docs/crate-extraction.md`, `--manifest-dir docs/crate-extraction`, `--candidate-family redb-raft-kv`, `--output-json openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.json`, and `--output-markdown openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.md`, writes deterministic JSON plus markdown summaries, and verifies candidate manifests, readiness-state restrictions, required exception metadata fields and owner/reason completeness, default feature minima, direct app-bundle absence, transitive app-bundle absence, feature unification through representative workspace consumers/re-exporters, and re-export leak absence; the checker MUST fail on `publishable from monorepo` or `future repository split candidate` labels until license/publication policy is resolved and MUST fail unowned or incomplete exception entries. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extractable-crate-boundaries-are-explicit.typed-dependency-policy-is-source-of-truth,architecture.modularity.extractable-crate-boundaries-are-explicit.dependency-checks-catch-transitive-and-reexport-leaks,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-standalone-verification-rails]
  - Evidence: `scripts/check-crate-extraction-readiness.rs`, `docs/crate-extraction/policy.ncl`, `openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.json`, `openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.md`, `openspec/changes/prepare-crate-extraction/evidence/readiness-checker-run.txt`

- [x] I5 Extend the extraction-readiness checker with positive examples, negative boundary checks, isolated downstream-style consumer proof that does not use compatibility re-exports as the primary API, compatibility re-export coverage, and verification evidence indexing requirements. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-standalone-verification-rails,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-is-proven-outside-aspen-app-bundle-assumptions,architecture.modularity.extractable-crate-boundaries-are-explicit.extraction-artifacts-have-canonical-locations]
  - Evidence: `scripts/check-crate-extraction-readiness.rs`, `openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.json`, `openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.md`, `openspec/changes/prepare-crate-extraction/verification.md`

- [x] I6 Before moving Redb storage code or adding compatibility migrations, review the proposed Redb Raft KV layer split and OpenRaft public/internal dependency boundary, then update manifests/design with any deviation; stop before implementation if a deviation lacks owner, rationale, and revised verification rails. [covers=architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layer-map-is-traceable,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.openraft-dependency-boundary-is-explicit]
  - Evidence: `openspec/changes/prepare-crate-extraction/evidence/redb-raft-kv-layer-review.md`, `openspec/changes/prepare-crate-extraction/design.md`, `docs/crate-extraction/aspen-kv-types.md`, `docs/crate-extraction/aspen-raft-kv-types.md`, `docs/crate-extraction/aspen-redb-storage.md`, `docs/crate-extraction/aspen-raft-kv.md`, `docs/crate-extraction/aspen-raft-network.md`, `docs/crate-extraction/aspen-raft-compat.md`

- [x] I7 Keep reusable KV command and response types in `aspen-kv-types`, create `aspen-raft-kv-types` only for reusable OpenRaft app types, membership metadata, Raft app data, and storage errors, and explicitly document the `aspen-raft-types` package/API transition plan for direct consumers (migrate, dependency-key alias, temporary compatibility crate/re-export, owner, tests, and removal criteria) while keeping Aspen bootstrap, cluster orchestration, trust, secrets, SQL, coordination, client API, handler registries, dogfood defaults, binaries, and root app bundles out of default reusable features. [covers=architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layer-map-is-traceable,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layers-have-manifests,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.consensus-node-layer-has-reusable-boundary,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.consensus-kv-facade-hides-aspen-app-identity,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.extracted-libraries-keep-compatibility-reexports-during-migration]
  - Evidence: `crates/aspen-raft-kv-types/src/lib.rs`, `docs/crate-extraction/aspen-raft-kv-types.md`, `openspec/changes/prepare-crate-extraction/evidence/implementation-diff-i7-raft-kv-types.txt`, `openspec/changes/prepare-crate-extraction/evidence/aspen-raft-kv-types-check.txt`, `openspec/changes/prepare-crate-extraction/evidence/aspen-raft-kv-types-dependency-tree.txt`

- [x] I8 Create a pre-storage `aspen-raft-kv` facade skeleton that exposes node configuration, membership setup, storage path configuration, bounded resource settings, and `KeyValueStore` / `ClusterController` style operations without requiring Aspen binary configuration or binding to the old `aspen-raft` storage surface; final Redb-backed execution must wait for the `I10` storage migration. [covers=architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layer-map-is-traceable,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.consensus-node-layer-has-reusable-boundary,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.consensus-kv-facade-hides-aspen-app-identity,architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.binary-only-code-does-not-own-reusable-behavior]
  - Evidence: `crates/aspen-raft-kv/src/lib.rs`, `docs/crate-extraction/aspen-raft-kv.md`, `openspec/changes/prepare-crate-extraction/evidence/implementation-diff-i8-raft-kv-facade.txt`, `openspec/changes/prepare-crate-extraction/evidence/aspen-raft-kv-facade-check.txt`, `openspec/changes/prepare-crate-extraction/evidence/aspen-raft-kv-facade-dependency-tree.txt`

<!-- Pending task evidence; I9 remains unchecked until V1. -->

- [ ] I9 Keep `aspen-raft-network` as the explicit iroh/IRPC adapter crate or feature for the reusable KV stack, prove storage plus consensus contracts compile without constructing concrete iroh endpoints, and prove Aspen runtime/shipped adapters remain iroh-only with no arbitrary non-iroh runtime transport introduced by the reusable facade. [covers=architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layer-map-is-traceable,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.transport-adapter-is-explicit,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles]
  - Evidence prepared but task left unchecked until the broader `V1` compile and feature-topology rail is checked: `openspec/changes/prepare-crate-extraction/evidence/i9-storage-consensus-adapter-boundary.md`, `openspec/changes/prepare-crate-extraction/evidence/i9-compile-matrix-after.txt`, `openspec/changes/prepare-crate-extraction/evidence/i9-aspen-redb-storage-tree-after.txt`, `openspec/changes/prepare-crate-extraction/evidence/i9-aspen-raft-kv-types-tree.txt`, `openspec/changes/prepare-crate-extraction/evidence/i9-aspen-raft-kv-tree.txt`, `openspec/changes/prepare-crate-extraction/evidence/i9-aspen-raft-network-tree.txt`, `openspec/changes/prepare-crate-extraction/evidence/i9-storage-consensus-no-iroh-grep.txt`, `openspec/changes/prepare-crate-extraction/evidence/i9-aspen-redb-storage-forbidden-after.txt`, `openspec/changes/prepare-crate-extraction/evidence/aspen-redb-storage-tests.txt`, `docs/crate-extraction/aspen-redb-storage.md`, `docs/crate-extraction/aspen-raft-network.md`

- [x] I13 Add one family manifest stub at `docs/crate-extraction/foundational-types.md` for foundational type/helper candidates, explicitly including `crates/aspen-storage-types`, `crates/aspen-cluster-types`, and `crates/aspen-traits`, recording owner, readiness state, default-feature contract, `aspen-storage-types` `SM_KV_TABLE` / `redb::TableDefinition` cleanup status, `aspen-traits` transitive default-feature leak status, and next action without moving code in this task. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv,architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners]
  - Evidence: `docs/crate-extraction/foundational-types.md`, `docs/crate-extraction.md`, `openspec/changes/prepare-crate-extraction/evidence/family-manifest-index.md`

- [x] I14 Add one family manifest stub at `docs/crate-extraction/auth-ticket.md` for auth/ticket candidates, recording owner, readiness state, runtime feature contract, and next action without moving code in this task. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv,architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners]
  - Evidence: `docs/crate-extraction/auth-ticket.md`, `docs/crate-extraction.md`, `openspec/changes/prepare-crate-extraction/evidence/family-manifest-index.md`

- [x] I15 Add one family manifest stub at `docs/crate-extraction/protocol-wire.md` for protocol/wire candidates, explicitly including `crates/aspen-client-api`, recording owner, readiness state, wire-compatibility rails, and next action without moving code in this task. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv,architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners]
  - Evidence: `docs/crate-extraction/protocol-wire.md`, `docs/crate-extraction.md`, `openspec/changes/prepare-crate-extraction/evidence/family-manifest-index.md`

- [x] I16 Add inventory follow-up entries for service/runtime candidates that need separate changes, including coordination, blob/castore/cache, commit DAG/KV branch, jobs/CI core, transport/RPC, trust/crypto, plugin/config, testing harnesses, and binary shell cleanup, and record owner plus manifest link or explicit `manifest not yet created` status without combining their implementation into this change. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv,architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners]
  - Evidence: `docs/crate-extraction.md`, `openspec/changes/prepare-crate-extraction/evidence/service-follow-up-selection.md`

- [x] I17 For each service/runtime candidate selected from `I16`, record follow-up or deferred decisions in `docs/crate-extraction.md`, save the selection rationale under `openspec/changes/prepare-crate-extraction/evidence/service-follow-up-selection.md`, and do not create new follow-up OpenSpec proposals in this task. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv]
  - Evidence: `docs/crate-extraction.md`, `openspec/changes/prepare-crate-extraction/evidence/service-follow-up-selection.md`

- [x] I18 Refresh `docs/crate-extraction.md` after `I13`-`I17` so every affected inventory row has current owner, exact manifest link (`foundational-types.md`, `auth-ticket.md`, `protocol-wire.md`) or `manifest not yet created`, readiness state, and next-action fields synchronized with the family manifests and follow-up/deferred decisions. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners,architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist,architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv]
  - Evidence: `docs/crate-extraction.md`, `docs/crate-extraction/foundational-types.md`, `docs/crate-extraction/auth-ticket.md`, `docs/crate-extraction/protocol-wire.md`, `openspec/changes/prepare-crate-extraction/evidence/family-manifest-index.md`

- [x] I19 Capture the pre-migration baseline audit for affected binary and app-shell paths (`src/bin/aspen_node`, `crates/aspen-cli`, dogfood, handlers, bridges, gateways, web, and TUI), recording reusable behavior risks before Redb Raft KV code movement; post-migration binary-shell verification remains covered by `V5`, `V7`, `V8`, `V9`, and `V12`. [covers=architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.binary-only-code-does-not-own-reusable-behavior]
  - Evidence: `openspec/changes/prepare-crate-extraction/evidence/binary-shell-audit.md`, `docs/crate-extraction.md`

<!-- Pending compatibility evidence; V7/V8/V9 remain unchecked until the post-migration Redb Raft KV move tasks they verify are complete. -->

- [ ] V7 Save node and cluster compatibility evidence for `cargo check -p aspen --no-default-features --features node-runtime`, `cargo check -p aspen-cluster`, and `cargo check -p aspen-rpc-handlers`, with artifacts under `openspec/changes/prepare-crate-extraction/evidence/compat-node-cluster.md`. [covers=architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.binary-only-code-does-not-own-reusable-behavior,architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.aspen-compatibility-consumers-stay-verified-during-migration]
  - Evidence prepared but task left unchecked until post-migration verification is valid: `openspec/changes/prepare-crate-extraction/evidence/compat-node-cluster.md`, `openspec/changes/prepare-crate-extraction/evidence/feature-matrix.md`

- [ ] V8 Save CLI, dogfood, and handler compatibility evidence for `cargo check -p aspen-cli`, `cargo check -p aspen-dogfood`, and the exact handler package list `aspen-rpc-handlers`, `aspen-core-essentials-handler`, `aspen-cluster-handler`, `aspen-blob-handler`, `aspen-forge-handler`, `aspen-docs-handler`, `aspen-secrets-handler`, `aspen-ci-handler`, `aspen-job-handler`, and `aspen-nix-handler`, with artifacts under `openspec/changes/prepare-crate-extraction/evidence/compat-cli-dogfood-handlers.md`. [covers=architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.binary-only-code-does-not-own-reusable-behavior,architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.aspen-compatibility-consumers-stay-verified-during-migration]
  - Evidence prepared but task left unchecked until post-migration verification is valid: `openspec/changes/prepare-crate-extraction/evidence/compat-cli-dogfood-handlers.md`, `openspec/changes/prepare-crate-extraction/evidence/aspen-jobs-worker-shell-alias-check.txt`, `openspec/changes/prepare-crate-extraction/evidence/feature-matrix.md`

- [ ] V9 Save bridge, gateway, web, and TUI compatibility evidence for the exact bridge/gateway package list `aspen-cluster-bridges`, `aspen-snix-bridge`, `aspen-nix-cache-gateway`, and `aspen-h3-proxy`, plus `cargo check -p aspen-forge-web` and `cargo check -p aspen-tui`, with artifacts under `openspec/changes/prepare-crate-extraction/evidence/compat-bridges-web-tui.md`. [covers=architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.binary-only-code-does-not-own-reusable-behavior,architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.aspen-compatibility-consumers-stay-verified-during-migration]
  - Evidence prepared but task left unchecked until post-migration verification is valid: `openspec/changes/prepare-crate-extraction/evidence/compat-bridges-web-tui.md`, `openspec/changes/prepare-crate-extraction/evidence/aspen-snix-circuit-breaker-time-check.txt`, `openspec/changes/prepare-crate-extraction/evidence/feature-matrix.md`

## Review Scope Snapshot

I7 now records the reusable `aspen-raft-kv-types` boundary and the explicit legacy `aspen-raft-types` package/API transition plan. I8 now exposes and tests pre-storage `aspen-raft-kv` facade operations for KV and cluster-control trait surfaces without concrete Redb or iroh endpoint construction. I9 evidence proves the default storage/type/facade graphs do not reach concrete iroh or transport dependencies while `aspen-raft-network` remains the explicit adapter; I9 remains unchecked until the broader `V1` compile and feature-topology rail can be checked. The partial V1 feature matrix records reusable/core compile rails plus prepared V7/V8/V9 compatibility consumer rails, but those compatibility tasks remain unchecked because they are post-migration verification rails for I10-I12. V1 remains unchecked until the newly required core UI rail, future named storage/facade features from I10-I12, and dependency-boundary owners are covered or explicitly triaged.

## Verification Commands

### `openspec validate prepare-crate-extraction`

- Status: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/openspec-validate.txt`

### `cargo metadata --format-version 1 --no-deps > openspec/changes/prepare-crate-extraction/evidence/cargo-metadata-no-deps.json`

- Status: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/cargo-metadata-no-deps.json`

### `cargo tree -p {aspen-redb-storage,aspen-raft-types,aspen-raft,aspen-raft-network} -e features`

- Status: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-redb-storage.txt`
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-raft-types.txt`
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-raft.txt`
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-raft-network.txt`

### `nix run nixpkgs#nickel -- typecheck docs/crate-extraction/policy.ncl`

- Status: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/nickel-policy-typecheck.txt`

### `scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family redb-raft-kv --output-json openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.json --output-markdown openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.md`

- Status: expected fail: owner-needed exceptions remain until human assignment
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/readiness-checker-run.txt`
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.json`
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.md`

### `cargo check -p aspen-kv-types -p aspen-raft-types`

- Status: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/baseline-raft-kv-types-check.txt`

### `cargo check -p aspen-raft-kv-types --no-default-features && cargo test -p aspen-raft-kv-types`

- Status: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/aspen-raft-kv-types-check.txt`

### `cargo tree -p aspen-raft-kv-types --no-default-features`

- Status: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/aspen-raft-kv-types-dependency-tree.txt`

### `rustfmt crates/aspen-raft-kv/src/lib.rs && cargo check -p aspen-raft-kv --no-default-features && cargo test -p aspen-raft-kv`

- Status: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/aspen-raft-kv-facade-check.txt`

### `cargo tree -p aspen-raft-kv --no-default-features`

- Status: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/aspen-raft-kv-facade-dependency-tree.txt`

### `cargo check` partial V1 reusable/core feature matrix

- Status: pass for reusable/default and impacted foundational/core compile slice; V1 remains unchecked pending compatibility consumer rails.
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/feature-matrix.md`
- Transcript: `openspec/changes/prepare-crate-extraction/evidence/feature-matrix-core-slice.txt`

### `cargo test -p aspen-redb-storage`

- Baseline status: failed before this session on stale `LeaseExpirationInput` doctest.
- Baseline artifact: `openspec/changes/prepare-crate-extraction/evidence/baseline-redb-storage-doctest-failure.txt`
- Post-change status: pass
- Post-change artifact: `openspec/changes/prepare-crate-extraction/evidence/aspen-redb-storage-tests.txt`

### `cargo check -p aspen-redb-storage --no-default-features && cargo check -p aspen-raft-kv-types --no-default-features && cargo check -p aspen-raft-kv --no-default-features && cargo check -p aspen-raft-network --no-default-features`

- Status: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/i9-compile-matrix-after.txt`

### `cargo tree` / `rg` I9 dependency-boundary probes

- Status: pass; storage/type/facade default graphs have no concrete iroh/IRPC/transport/adapter matches, and storage default no longer reaches `aspen-core`/`aspen-core-shell`.
- Summary: `openspec/changes/prepare-crate-extraction/evidence/i9-storage-consensus-adapter-boundary.md`
- Artifacts: `openspec/changes/prepare-crate-extraction/evidence/i9-aspen-redb-storage-tree-after.txt`, `openspec/changes/prepare-crate-extraction/evidence/i9-aspen-raft-kv-types-tree.txt`, `openspec/changes/prepare-crate-extraction/evidence/i9-aspen-raft-kv-tree.txt`, `openspec/changes/prepare-crate-extraction/evidence/i9-aspen-raft-network-tree.txt`, `openspec/changes/prepare-crate-extraction/evidence/i9-storage-consensus-no-iroh-grep.txt`, `openspec/changes/prepare-crate-extraction/evidence/i9-aspen-redb-storage-forbidden-after.txt`, `openspec/changes/prepare-crate-extraction/evidence/aspen-redb-storage-boundary-check.txt`

### V7 node and cluster compatibility commands

- Status: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/compat-node-cluster.md`
- Commands: `cargo check -p aspen --no-default-features --features node-runtime`, `cargo check -p aspen-cluster`, `cargo check -p aspen-rpc-handlers`

### V8 CLI, dogfood, and handler compatibility commands

- Status: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/compat-cli-dogfood-handlers.md`
- Commands: `cargo check -p aspen-cli`, `cargo check -p aspen-dogfood`, `cargo check -p aspen-rpc-handlers`, `cargo check -p aspen-core-essentials-handler`, `cargo check -p aspen-cluster-handler`, `cargo check -p aspen-blob-handler`, `cargo check -p aspen-forge-handler`, `cargo check -p aspen-docs-handler`, `cargo check -p aspen-secrets-handler`, `cargo check -p aspen-ci-handler`, `cargo check -p aspen-job-handler`, `cargo check -p aspen-nix-handler`

### `cargo check -p aspen-jobs-worker-maintenance && cargo check -p aspen-jobs-worker-replication && cargo check -p aspen-jobs-worker-sql`

- Status: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/aspen-jobs-worker-shell-alias-check.txt`

### V9 bridge, gateway, web, and TUI compatibility commands

- Status: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/compat-bridges-web-tui.md`
- Commands: `cargo check -p aspen-cluster-bridges`, `cargo check -p aspen-snix-bridge`, `cargo check -p aspen-nix-cache-gateway`, `cargo check -p aspen-h3-proxy`, `cargo check -p aspen-forge-web`, `cargo check -p aspen-tui`

### `cargo check -p aspen-snix && cargo test -p aspen-snix circuit_breaker_time`

- Status: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/aspen-snix-circuit-breaker-time-check.txt`

### `scripts/openspec-preflight.sh prepare-crate-extraction`

- Prior status: pass
- Prior artifact: `openspec/changes/prepare-crate-extraction/evidence/openspec-preflight-i7.txt`
- I9-prep status after preparing I9 evidence while leaving I9 unchecked: pass
- I9-prep artifact: `openspec/changes/prepare-crate-extraction/evidence/openspec-preflight-i9.txt`
- Compatibility-prep status after preparing V7/V8/V9 evidence while leaving those post-migration rails unchecked: pass
- Compatibility-prep artifact: `openspec/changes/prepare-crate-extraction/evidence/openspec-preflight-v7-v9.txt`
- Current status after checking I7/I8: pass
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/openspec-preflight-i8.txt`

### `openspec_gate(stage="tasks", change="prepare-crate-extraction")`

- Status: fail
- Artifact: `openspec/changes/prepare-crate-extraction/evidence/openspec-gate-tasks-i8.txt`
- Follow-up captured from the gate: I9/V2 now carry the runtime-networking guardrail; V11 now requires gate transcript synchronization before intermediate or final task-status claims. `openspec/changes/prepare-crate-extraction/evidence/human-oracle-escalation-checkpoint.md` records the required escalation checkpoint for repeated omission findings. The final captured gate still reports review-scope blockers because the gate invocation did not receive the full `verification.md`/evidence bundle as supplied artifacts; do not treat the task status as gate-accepted.

## Notes

- Readiness checker failure is expected at this stage because exception owners remain `owner needed`; this proves the checker fails unowned exception entries before readiness is claimed.
- Do not check another task until the corresponding evidence artifact is linked above.
