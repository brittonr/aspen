# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.

## Implementation Evidence

- Changed file: `openspec/changes/decompose-next-five-crate-families/tasks.md`
- Changed file: `openspec/changes/decompose-next-five-crate-families/verification.md`

## Task Coverage

- [x] R1 Capture current extraction status for completed families and candidate inventory, including Redb Raft KV, coordination, protocol/wire, transport/RPC, blob/castore/cache, KV branch/commit DAG, foundational types, auth/tickets, jobs/CI, trust/crypto/secrets, testing harness, config/plugin, and binary shells; save selection evidence under `openspec/changes/decompose-next-five-crate-families/evidence/selection-baseline.md`. [covers=architecture.modularity.next-decomposition-wave-is-selected] ✅ 1m (started: 2026-04-29T14:12:15Z → completed: 2026-04-29T14:13:29Z)
  - Evidence: `openspec/changes/decompose-next-five-crate-families/evidence/selection-baseline.md`, `openspec/changes/decompose-next-five-crate-families/evidence/r1-selection-baseline-check.txt`

- [x] R2 Capture per-family dependency/source baselines for `foundational-types`, `auth-ticket`, `jobs-ci-core`, `trust-crypto-secrets`, and `testing-harness`, including direct dependencies, transitive app/runtime leak paths, representative consumers, existing test rails, and first blockers; save artifacts under `openspec/changes/decompose-next-five-crate-families/evidence/`. [covers=architecture.modularity.next-decomposition-first-blockers-are-explicit] ✅ 3m (started: 2026-04-29T14:13:37Z → completed: 2026-04-29T14:15:29Z)
  - Evidence: `openspec/changes/decompose-next-five-crate-families/evidence/r2-family-baselines.md`, `openspec/changes/decompose-next-five-crate-families/evidence/r2-direct-dependency-snapshot.txt`, `openspec/changes/decompose-next-five-crate-families/evidence/r2-family-baselines-check.txt`

## Review Scope Snapshot

### `git diff -- openspec/changes/decompose-next-five-crate-families/tasks.md openspec/changes/decompose-next-five-crate-families/evidence/selection-baseline.md`

- Status: captured by task-local evidence
- Artifact: `openspec/changes/decompose-next-five-crate-families/evidence/selection-baseline.md`

## Verification Commands

### `grep selected and deferred families in selection-baseline.md`

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families/evidence/r1-selection-baseline-check.txt`

### `scripts/openspec-preflight.sh decompose-next-five-crate-families`

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families/evidence/openspec-preflight-r1.txt`

### `capture selected crate direct dependencies from crates/*/Cargo.toml`

- Status: captured
- Artifact: `openspec/changes/decompose-next-five-crate-families/evidence/r2-direct-dependency-snapshot.txt`

### `grep family baseline required sections`

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families/evidence/r2-family-baselines-check.txt`

### `scripts/openspec-preflight.sh decompose-next-five-crate-families` (R2)

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families/evidence/openspec-preflight-r2.txt`
