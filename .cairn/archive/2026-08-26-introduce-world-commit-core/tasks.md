## Phase 1: Baseline and pure core

- [x] [serial] Record baseline focused tests for content refs, Preserves canonicalization, replay, scheduler, tasks, effects, runtime profiles, and root inventories. r[molten.world_commit.verification]
- [x] [serial] Define closed world-commit, parent, snapshot-profile, typed-root, revision-fence, completeness, and diagnostic DTOs in the pure core. r[molten.world_commit.core] r[molten.world_commit.typed_roots]
- [x] [depends:world-commit-dtos] Implement domain-separated BLAKE3 identity over canonical Preserves bytes with deterministic parent and root ordering. r[molten.world_commit.core]
- [x] [depends:world-commit-dtos] Implement pure capture planning, bound checks, duplicate rejection, domain checks, and revision-recheck comparison. r[molten.world_commit.capture]
- [x] [parallel] Implement pure closure validation, restore ordering, replay classification, and first-missing-root diagnostics. r[molten.world_commit.restore]

## Phase 2: Schemas and shell

- [x] [depends:world-commit-identity] Add versioned Preserves schemas and exact Rust codec parity fixtures for commits, capture receipts, closure reports, and restore plans. r[molten.world_commit.core] r[molten.world_commit.verification]
- [x] [depends:world-commit-capture-plan] Add narrow root-observation, immutable-object, revision-recheck, commit-publication, and restore ports. r[molten.world_commit.capture] r[molten.world_commit.restore]
- [x] [depends:world-commit-ports] Implement the local shell that persists roots, rechecks fences, and publishes the commit object last. r[molten.world_commit.capture]
- [x] [depends:world-commit-ports] Add detached Valence evidence and artifact-auth statement projections without placing signatures or attestations in the commit hash. r[molten.world_commit.detached_evidence]
- [x] [depends:world-commit-local-shell] Add operator commands to inspect, validate, explain, and plan restore for one explicit commit identity. r[molten.world_commit.restore] r[molten.world_commit.detached_evidence]

## Phase 3: Verification and documentation

- [x] [parallel] Add positive logical captures, stable repeated identities, complete restores, and cohort-bound opaque-reference fixtures. r[molten.world_commit.verification]
- [x] [parallel] Add negative drift, incomplete inventory, missing root, wrong domain, duplicate parent, cycle, malformed canonical bytes, stale schema, secret disclosure, authority confusion, and evidence-inside-core fixtures. r[molten.world_commit.verification]
- [x] [serial] Document ownership, root profiles, detached envelopes, restore limits, and the absence of external `RealmCommit` compatibility claims. r[molten.world_commit.detached_evidence]
- [x] [depends:world-commit-verification] Run focused tests, Octet, Clippy with warnings denied, Cairn validation and gates, lifecycle checks, and relevant Nix checks. r[molten.world_commit.verification]
  - Passed: the dedicated full-catalog `world-commit-octet-deny-all` check over the real core source reported zero findings, warnings, and errors.
  - Passed: focused core and shell tests, Clippy with warnings denied, Cairn strict validation and gates, `nix flake check --no-build`, and `checks.x86_64-linux.verified-node-replication-pilot`.
  - Bounded: inherited repository-wide findings remain outside this focused completion claim.
