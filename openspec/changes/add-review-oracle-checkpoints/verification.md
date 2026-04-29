# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.

## Implementation Evidence

- Changed file: `openspec/templates/oracle-checkpoint.md`
- Changed file: `openspec/templates/verification.md`
- Changed file: `openspec/changes/add-review-oracle-checkpoints/tasks.md`
- Changed file: `openspec/changes/add-review-oracle-checkpoints/verification.md`

## Task Coverage

- [x] I1 Add checkpoint template/guidance for unresolved review questions. [covers=openspec-governance.review-oracle-checkpoints.records-ambiguity] ✅ 1m (started: 2026-04-29T13:58:56Z → completed: 2026-04-29T13:59:36Z)
  - Evidence: `openspec/templates/oracle-checkpoint.md`, `openspec/templates/verification.md`, `openspec/changes/add-review-oracle-checkpoints/evidence/i1-template-guidance.diff`, `openspec/changes/add-review-oracle-checkpoints/evidence/i1-template-guidance-check.txt`

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
