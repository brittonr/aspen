# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.

## Implementation Evidence

- Changed file: `openspec/changes/add-review-oracle-checkpoints/tasks.md`
- Changed file: `openspec/changes/add-review-oracle-checkpoints/verification.md`

## Task Coverage

- [x] I1 Add checkpoint template/guidance for unresolved review questions. [covers=openspec-governance.review-oracle-checkpoints.records-ambiguity] ✅ 1m (started: 2026-04-29T13:58:56Z → completed: 2026-04-29T13:59:36Z)
  - Evidence: `openspec/changes/add-review-oracle-checkpoints/evidence/i1-template-guidance.diff`, `openspec/changes/add-review-oracle-checkpoints/evidence/i1-template-guidance-check.txt`

- [x] I2 Teach done-review to flag ambiguous completion without checkpoint. [covers=openspec-governance.review-oracle-checkpoints.missing-checkpoint-flagged] ✅ 4m (started: 2026-04-29T14:00:50Z → completed: 2026-04-29T14:04:27Z)
  - Evidence: `openspec/changes/add-review-oracle-checkpoints/evidence/agentkit-oracle-checkpoint-commits.patch`, `openspec/changes/add-review-oracle-checkpoints/evidence/agentkit-review-parsing-test.txt`, `openspec/changes/add-review-oracle-checkpoints/evidence/agentkit-node-checks.txt`
- [x] I3 Teach OpenSpec gates to recognize checkpoint records and flag ambiguous stage completion without checkpoint. [covers=openspec-governance.review-oracle-checkpoints.missing-checkpoint-flagged,openspec-governance.review-oracle-checkpoints.records-ambiguity] ✅ 1m (started: 2026-04-29T14:05:59Z → completed: 2026-04-29T14:06:08Z)
  - Evidence: `openspec/changes/add-review-oracle-checkpoints/evidence/agentkit-oracle-checkpoint-commits.patch`, `openspec/changes/add-review-oracle-checkpoints/evidence/agentkit-review-parsing-test.txt`, `openspec/changes/add-review-oracle-checkpoints/evidence/agentkit-node-checks.txt`

## Review Scope Snapshot

### `git diff -- openspec/templates/oracle-checkpoint.md openspec/templates/verification.md openspec/changes/add-review-oracle-checkpoints/tasks.md`

- Status: captured
- Artifact: `openspec/changes/add-review-oracle-checkpoints/evidence/i1-template-guidance.diff`

## Verification Commands

### `test -s openspec/templates/oracle-checkpoint.md && grep required checkpoint fields`

- Status: pass
- Artifact: `openspec/changes/add-review-oracle-checkpoints/evidence/i1-template-guidance-check.txt`

### `scripts/openspec-preflight.sh add-review-oracle-checkpoints`

- Status: pass
- Artifact: `openspec/changes/add-review-oracle-checkpoints/evidence/openspec-preflight-i1.txt`

### `git -C ../agentkit show --stat --patch 7b03596 7966c67 074131f`

- Status: captured
- Artifact: `openspec/changes/add-review-oracle-checkpoints/evidence/agentkit-oracle-checkpoint-commits.patch`

### `nix shell nixpkgs#nodejs -c node tools/test-review-parsing.mjs`

- Status: pass
- Artifact: `openspec/changes/add-review-oracle-checkpoints/evidence/agentkit-review-parsing-test.txt`

### `nix shell nixpkgs#nodejs -c node --check <agentkit oracle files>`

- Status: pass
- Artifact: `openspec/changes/add-review-oracle-checkpoints/evidence/agentkit-node-checks.txt`

### `scripts/openspec-preflight.sh add-review-oracle-checkpoints` (I2)

- Status: pass
- Artifact: `openspec/changes/add-review-oracle-checkpoints/evidence/openspec-preflight-i2.txt`
