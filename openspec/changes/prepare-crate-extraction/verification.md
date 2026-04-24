# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.

## Implementation Evidence

- Changed file: `docs/crate-extraction.md`
- Changed file: `docs/crate-extraction/aspen-kv-types.md`
- Changed file: `docs/crate-extraction/aspen-raft-kv-types.md`
- Changed file: `docs/crate-extraction/aspen-redb-storage.md`
- Changed file: `docs/crate-extraction/aspen-raft-kv.md`
- Changed file: `docs/crate-extraction/aspen-raft-network.md`
- Changed file: `docs/crate-extraction/aspen-raft-compat.md`
- Changed file: `docs/crate-extraction/foundational-types.md`
- Changed file: `docs/crate-extraction/auth-ticket.md`
- Changed file: `docs/crate-extraction/protocol-wire.md`
- Changed file: `docs/crate-extraction/policy.ncl`
- Changed file: `scripts/check-crate-extraction-readiness.rs`
- Changed file: `openspec/changes/prepare-crate-extraction/proposal.md`
- Changed file: `openspec/changes/prepare-crate-extraction/design.md`
- Changed file: `openspec/changes/prepare-crate-extraction/specs/architecture-modularity/spec.md`
- Changed file: `openspec/changes/prepare-crate-extraction/tasks.md`
- Changed file: `openspec/changes/prepare-crate-extraction/verification.md`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/openspec-validate.txt`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/cargo-metadata-no-deps.json`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/extraction-inventory-baseline.md`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/redb-raft-kv-coupling-baseline.md`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/nickel-policy-typecheck.txt`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.json`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.md`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/readiness-checker-run.txt`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/redb-raft-kv-layer-review.md`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/service-follow-up-selection.md`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/binary-shell-audit.md`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-redb-storage.txt`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-raft-types.txt`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-raft.txt`
- Changed file: `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-raft-network.txt`

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
  - Evidence: `docs/crate-extraction/aspen-kv-types.md`, `docs/crate-extraction/aspen-raft-kv-types.md`, `docs/crate-extraction/aspen-redb-storage.md`, `docs/crate-extraction/aspen-raft-kv.md`, `docs/crate-extraction/aspen-raft-network.md`, `docs/crate-extraction/aspen-raft-compat.md`

- [x] I3 Add typed `docs/crate-extraction/policy.ncl` defining candidate classes, forbidden crate categories, allowed exceptions, tested feature sets, representative workspace consumers/re-exporters per candidate or family, required exception metadata fields (`candidate`, `feature_set`, `dependency_path`, `owner`, `reason`), and the rule that `publishable from monorepo` / `future repository split candidate` readiness states are rejected until license and publication policy is decided. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extractable-crate-boundaries-are-explicit.typed-dependency-policy-is-source-of-truth,architecture.modularity.extractable-crate-boundaries-are-explicit.dependency-checks-catch-transitive-and-reexport-leaks,architecture.modularity.extractable-crate-boundaries-are-explicit.extraction-artifacts-have-canonical-locations,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defines-documentation-and-release-metadata]
  - Evidence: `docs/crate-extraction/policy.ncl`, `openspec/changes/prepare-crate-extraction/evidence/nickel-policy-typecheck.txt`

- [x] I4 Add a deterministic extraction-readiness checker that reads `docs/crate-extraction/policy.ncl` and verifies candidate manifests, readiness-state restrictions, required exception metadata fields and owner/reason completeness, default feature minima, direct app-bundle absence, transitive app-bundle absence, feature unification through representative workspace consumers/re-exporters, and re-export leak absence; the checker MUST fail on `publishable from monorepo` or `future repository split candidate` labels until license/publication policy is resolved and MUST fail unowned or incomplete exception entries. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extractable-crate-boundaries-are-explicit.typed-dependency-policy-is-source-of-truth,architecture.modularity.extractable-crate-boundaries-are-explicit.dependency-checks-catch-transitive-and-reexport-leaks,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-standalone-verification-rails]
  - Evidence: `scripts/check-crate-extraction-readiness.rs`, `docs/crate-extraction/policy.ncl`, `openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.json`, `openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.md`, `openspec/changes/prepare-crate-extraction/evidence/readiness-checker-run.txt`

- [x] I5 Extend the extraction-readiness checker with positive examples, negative boundary checks, isolated downstream-style consumer proof that does not use compatibility re-exports as the primary API, compatibility re-export coverage, and verification evidence indexing requirements. [covers=architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-standalone-verification-rails,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-is-proven-outside-aspen-app-bundle-assumptions,architecture.modularity.extractable-crate-boundaries-are-explicit.extraction-artifacts-have-canonical-locations]
  - Evidence: `scripts/check-crate-extraction-readiness.rs`, `openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.json`, `openspec/changes/prepare-crate-extraction/evidence/dependency-boundary.md`, `openspec/changes/prepare-crate-extraction/verification.md`

- [x] I6 Before moving Redb storage code or adding compatibility migrations, review the proposed Redb Raft KV layer split and OpenRaft public/internal dependency boundary, then update manifests/design with any deviation; stop before implementation if a deviation lacks owner, rationale, and revised verification rails. [covers=architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layer-map-is-traceable,architecture.modularity.reusable-redb-raft-kv-stack-is-layered.openraft-dependency-boundary-is-explicit]
  - Evidence: `openspec/changes/prepare-crate-extraction/evidence/redb-raft-kv-layer-review.md`, `openspec/changes/prepare-crate-extraction/design.md`, `docs/crate-extraction/aspen-kv-types.md`, `docs/crate-extraction/aspen-raft-kv-types.md`, `docs/crate-extraction/aspen-redb-storage.md`, `docs/crate-extraction/aspen-raft-kv.md`, `docs/crate-extraction/aspen-raft-network.md`, `docs/crate-extraction/aspen-raft-compat.md`

- [x] I13 Add one family manifest stub at `docs/crate-extraction/foundational-types.md` for foundational type/helper candidates, explicitly including `crates/aspen-storage-types`, `crates/aspen-cluster-types`, and `crates/aspen-traits`, recording owner, readiness state, default-feature contract, `aspen-storage-types` `SM_KV_TABLE` / `redb::TableDefinition` cleanup status, `aspen-traits` transitive default-feature leak status, and next action without moving code in this task. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv,architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners]
  - Evidence: `docs/crate-extraction/foundational-types.md`, `docs/crate-extraction.md`

- [x] I14 Add one family manifest stub at `docs/crate-extraction/auth-ticket.md` for auth/ticket candidates, recording owner, readiness state, runtime feature contract, and next action without moving code in this task. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv,architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners]
  - Evidence: `docs/crate-extraction/auth-ticket.md`, `docs/crate-extraction.md`

- [x] I15 Add one family manifest stub at `docs/crate-extraction/protocol-wire.md` for protocol/wire candidates, explicitly including `crates/aspen-client-api`, recording owner, readiness state, wire-compatibility rails, and next action without moving code in this task. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv,architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist,architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners]
  - Evidence: `docs/crate-extraction/protocol-wire.md`, `docs/crate-extraction.md`

- [x] I16 Add inventory follow-up entries for service/runtime candidates that need separate changes, including coordination, blob/castore/cache, commit DAG/KV branch, jobs/CI core, transport/RPC, trust/crypto, plugin/config, testing harnesses, and binary shell cleanup, and record owner plus manifest link or explicit `manifest not yet created` status without combining their implementation into this change. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv,architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist,architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners]
  - Evidence: `docs/crate-extraction.md`, `openspec/changes/prepare-crate-extraction/evidence/service-follow-up-selection.md`

- [x] I17 For each service/runtime candidate selected from `I16`, record follow-up or deferred decisions in `docs/crate-extraction.md`, save the selection rationale under `openspec/changes/prepare-crate-extraction/evidence/service-follow-up-selection.md`, and do not create new follow-up OpenSpec proposals in this task. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv]
  - Evidence: `docs/crate-extraction.md`, `openspec/changes/prepare-crate-extraction/evidence/service-follow-up-selection.md`

- [x] I18 Refresh `docs/crate-extraction.md` after `I13`-`I17` so every affected inventory row has current owner, exact manifest link (`foundational-types.md`, `auth-ticket.md`, `protocol-wire.md`) or `manifest not yet created`, readiness state, and next-action fields synchronized with the family manifests and follow-up/deferred decisions. [covers=architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners,architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist,architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv]
  - Evidence: `docs/crate-extraction.md`, `docs/crate-extraction/foundational-types.md`, `docs/crate-extraction/auth-ticket.md`, `docs/crate-extraction/protocol-wire.md`

- [x] I19 Audit affected binary and app-shell paths (`src/bin/aspen_node`, `crates/aspen-cli`, dogfood, handlers, bridges, gateways, web, and TUI) for reusable behavior touched by this change; migrate any shared logic into library crates or save evidence that no migration is required for the touched paths. [covers=architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.binary-only-code-does-not-own-reusable-behavior]
  - Evidence: `openspec/changes/prepare-crate-extraction/evidence/binary-shell-audit.md`, `docs/crate-extraction.md`

## Review Scope Snapshot

No implementation diff artifact yet; checked tasks currently cover OpenSpec planning artifacts, extraction docs, baseline evidence, binary-shell audit, and the readiness checker.

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

## Notes

- Readiness checker failure is expected at this stage because exception owners remain `owner needed`; this proves the checker fails unowned exception entries before readiness is claimed.
- Do not check another task until the corresponding evidence artifact is linked above.
