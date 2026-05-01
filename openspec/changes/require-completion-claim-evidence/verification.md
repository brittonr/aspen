# Verification Evidence

## Implementation Evidence

- Changed file: `scripts/check-completion-claim-evidence.py`
- Changed file: `openspec/changes/require-completion-claim-evidence/fixtures/completion-claims/run-completion-claim-fixtures.sh`
- Changed file: `openspec/changes/require-completion-claim-evidence/evidence/i1-v1-completion-claim-fixtures.txt`
- Changed file: `openspec/changes/require-completion-claim-evidence/evidence/openspec-preflight-i1-v2.txt`
- Changed file: `openspec/changes/require-completion-claim-evidence/tasks.md`
- Changed file: `openspec/changes/require-completion-claim-evidence/verification.md`

## Task Coverage

- [x] I1 Add high-risk completion-claim detection to done-review, or to a checker that done-review invokes automatically. [covers=openspec-governance.completion-claim-evidence.evidence-backed-claim-passes,openspec-governance.completion-claim-evidence.unsupported-claim-fails] ✅ 15m (started: 2026-05-01T02:01:00Z → completed: 2026-05-01T02:16:13Z)
  - Evidence: `scripts/check-completion-claim-evidence.py`, `openspec/changes/require-completion-claim-evidence/evidence/i1-v1-completion-claim-fixtures.txt`
- [x] V1 Add tests for evidence-backed claims, unsupported claims, and uncertain-summary allowed behavior. [covers=openspec-governance.completion-claim-evidence.evidence-backed-claim-passes,openspec-governance.completion-claim-evidence.unsupported-claim-fails,openspec-governance.completion-claim-evidence.uncertain-summary-allowed] ✅ 15m (started: 2026-05-01T02:01:00Z → completed: 2026-05-01T02:16:13Z)
  - Evidence: `openspec/changes/require-completion-claim-evidence/fixtures/completion-claims/run-completion-claim-fixtures.sh`, `openspec/changes/require-completion-claim-evidence/evidence/i1-v1-completion-claim-fixtures.txt`
- [x] V2 Add claim-family fixtures for clean status, queue empty, all checks pass, archived, and validated claims; save transcripts. [covers=openspec-governance.completion-claim-evidence] ✅ 15m (started: 2026-05-01T02:01:00Z → completed: 2026-05-01T02:16:13Z)
  - Evidence: `openspec/changes/require-completion-claim-evidence/evidence/i1-v1-completion-claim-fixtures.txt`

## Verification Commands

- Command: `openspec/changes/require-completion-claim-evidence/fixtures/completion-claims/run-completion-claim-fixtures.sh > openspec/changes/require-completion-claim-evidence/evidence/i1-v1-completion-claim-fixtures.txt 2>&1`
- Artifact: `openspec/changes/require-completion-claim-evidence/evidence/i1-v1-completion-claim-fixtures.txt`
- Command: `scripts/openspec-preflight.sh require-completion-claim-evidence > openspec/changes/require-completion-claim-evidence/evidence/openspec-preflight-i1-v2.txt 2>&1`
- Artifact: `openspec/changes/require-completion-claim-evidence/evidence/openspec-preflight-i1-v2.txt`
