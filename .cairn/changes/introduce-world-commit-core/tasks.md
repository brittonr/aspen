## Phase 1: Baseline and pure core

- [ ] [serial] Record baseline focused tests for content refs, Preserves canonicalization, replay, scheduler, tasks, effects, runtime profiles, and root inventories. r[molten.world_commit.verification]
- [ ] [serial] Define closed world-commit, parent, snapshot-profile, typed-root, revision-fence, completeness, and diagnostic DTOs in the pure core. r[molten.world_commit.core] r[molten.world_commit.typed_roots]
- [ ] [depends:world-commit-dtos] Implement domain-separated BLAKE3 identity over canonical Preserves bytes with deterministic parent and root ordering. r[molten.world_commit.core]
- [ ] [depends:world-commit-dtos] Implement pure capture planning, bound checks, duplicate rejection, domain checks, and revision-recheck comparison. r[molten.world_commit.capture]
- [ ] [parallel] Implement pure closure validation, restore ordering, replay classification, and first-missing-root diagnostics. r[molten.world_commit.restore]

## Phase 2: Schemas and shell

- [ ] [depends:world-commit-identity] Add versioned Preserves schemas and exact Rust codec parity fixtures for commits, capture receipts, closure reports, and restore plans. r[molten.world_commit.core] r[molten.world_commit.verification]
- [ ] [depends:world-commit-capture-plan] Add narrow root-observation, immutable-object, revision-recheck, commit-publication, and restore ports. r[molten.world_commit.capture] r[molten.world_commit.restore]
- [ ] [depends:world-commit-ports] Implement the local shell that persists roots, rechecks fences, and publishes the commit object last. r[molten.world_commit.capture]
- [ ] [depends:world-commit-ports] Add detached Valence evidence and artifact-auth statement projections without placing signatures or attestations in the commit hash. r[molten.world_commit.detached_evidence]
- [ ] [depends:world-commit-local-shell] Add operator commands to inspect, validate, explain, and plan restore for one explicit commit identity. r[molten.world_commit.restore] r[molten.world_commit.detached_evidence]

## Phase 3: Verification and documentation

- [ ] [parallel] Add positive logical captures, stable repeated identities, complete restores, and cohort-bound opaque-reference fixtures. r[molten.world_commit.verification]
- [ ] [parallel] Add negative drift, incomplete inventory, missing root, wrong domain, duplicate parent, cycle, malformed canonical bytes, stale schema, secret disclosure, authority confusion, and evidence-inside-core fixtures. r[molten.world_commit.verification]
- [ ] [serial] Document ownership, root profiles, detached envelopes, restore limits, and the absence of external `RealmCommit` compatibility claims. r[molten.world_commit.detached_evidence]
- [ ] [depends:world-commit-verification] Run focused tests, Octet, Clippy with warnings denied, Cairn validation and gates, lifecycle checks, and relevant Nix checks. r[molten.world_commit.verification]
