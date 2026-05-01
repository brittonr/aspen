# Verification Evidence

## Implementation Evidence

- Changed file: `openspec/templates/verification.md`
- Changed file: `openspec/changes/require-completion-claim-evidence/evidence/i2-guidance-update.txt`
- Changed file: `openspec/changes/require-completion-claim-evidence/evidence/openspec-preflight-final.txt`
- Changed file: `openspec/changes/require-completion-claim-evidence/tasks.md`
- Changed file: `openspec/changes/require-completion-claim-evidence/verification.md`

## Task Coverage

- [x] I1 Add high-risk completion-claim detection to done-review, or to a checker that done-review invokes automatically. [covers=openspec-governance.completion-claim-evidence.evidence-backed-claim-passes,openspec-governance.completion-claim-evidence.unsupported-claim-fails] ✅ 15m (started: 2026-05-01T02:01:00Z → completed: 2026-05-01T02:16:13Z)
  - Evidence: `openspec/changes/require-completion-claim-evidence/evidence/i1-v1-completion-claim-fixtures.txt`
- [x] I2 Update prompt/checklist guidance to require status/list/check evidence before strong completion claims. [covers=openspec-governance.completion-claim-evidence] ✅ 5m (started: 2026-05-01T02:16:30Z → completed: 2026-05-01T02:21:55Z)
  - Evidence: `openspec/templates/verification.md`, `openspec/changes/require-completion-claim-evidence/evidence/i2-guidance-update.txt`
- [x] V1 Add tests for evidence-backed claims, unsupported claims, and uncertain-summary allowed behavior. [covers=openspec-governance.completion-claim-evidence.evidence-backed-claim-passes,openspec-governance.completion-claim-evidence.unsupported-claim-fails,openspec-governance.completion-claim-evidence.uncertain-summary-allowed] ✅ 15m (started: 2026-05-01T02:01:00Z → completed: 2026-05-01T02:16:13Z)
  - Evidence: `openspec/changes/require-completion-claim-evidence/fixtures/completion-claims/run-completion-claim-fixtures.sh`, `openspec/changes/require-completion-claim-evidence/evidence/i1-v1-completion-claim-fixtures.txt`
- [x] V2 Add claim-family fixtures for clean status, queue empty, all checks pass, archived, and validated claims; save transcripts. [covers=openspec-governance.completion-claim-evidence] ✅ 15m (started: 2026-05-01T02:01:00Z → completed: 2026-05-01T02:16:13Z)
  - Evidence: `openspec/changes/require-completion-claim-evidence/evidence/i1-v1-completion-claim-fixtures.txt`

## Verification Commands

- Command: `openspec/changes/require-completion-claim-evidence/fixtures/completion-claims/run-completion-claim-fixtures.sh > openspec/changes/require-completion-claim-evidence/evidence/i1-v1-completion-claim-fixtures.txt 2>&1`
- Artifact: `openspec/changes/require-completion-claim-evidence/evidence/i1-v1-completion-claim-fixtures.txt`
- Command: `scripts/openspec-preflight.sh require-completion-claim-evidence > openspec/changes/require-completion-claim-evidence/evidence/openspec-preflight-final.txt 2>&1`
- Artifact: `openspec/changes/require-completion-claim-evidence/evidence/openspec-preflight-final.txt`
