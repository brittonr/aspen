# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.

## Implementation Evidence

- Changed file: `scripts/openspec-preflight.sh`
- Changed file: `scripts/test-openspec-preflight-evidence.sh`
- Changed file: `openspec/templates/verification.md`
- Changed file: `openspec/changes/enforce-task-evidence-coverage/fixtures/openspec-preflight-evidence/README.md`
- Changed file: `openspec/changes/enforce-task-evidence-coverage/tasks.md`
- Changed file: `openspec/changes/enforce-task-evidence-coverage/verification.md`
- Changed file: `openspec/changes/enforce-task-evidence-coverage/evidence/bash-n-preflight-scripts.txt`
- Changed file: `openspec/changes/enforce-task-evidence-coverage/evidence/preflight-fixture-suite.txt`
- Changed file: `openspec/changes/enforce-task-evidence-coverage/evidence/openspec-validate.txt`
- Changed file: `openspec/changes/enforce-task-evidence-coverage/evidence/implementation-diff.patch`
- Changed file: `openspec/changes/enforce-task-evidence-coverage/evidence/openspec-preflight.txt`
- Changed file: `openspec/changes/enforce-task-evidence-coverage/evidence/openspec-gate-tasks.txt`

## Task Coverage

- [x] I1 Extend `scripts/openspec-preflight.sh` to reject missing, untracked, empty, or placeholder evidence for checked tasks, while accepting only change-local evidence or currently changed source/doc files. [covers=openspec-governance.task-evidence-coverage.checked-task-cites-valid-evidence,openspec-governance.task-evidence-coverage.missing-evidence-fails,openspec-governance.task-evidence-coverage.placeholder-evidence-fails]
  - Evidence: `scripts/openspec-preflight.sh`, `openspec/changes/enforce-task-evidence-coverage/evidence/implementation-diff.patch`, `openspec/changes/enforce-task-evidence-coverage/evidence/preflight-fixture-suite.txt`
- [x] I2 Add positive and negative fixtures for active and archived changes with both valid evidence routes (change-local evidence and currently changed source/doc implementation evidence) plus missing, untracked, empty, `pending`, `TODO`, `placeholder`, and tracked-but-disallowed external evidence path cases. [covers=openspec-governance.task-evidence-coverage.checked-task-cites-valid-evidence,openspec-governance.task-evidence-coverage.missing-evidence-fails,openspec-governance.task-evidence-coverage.placeholder-evidence-fails]
  - Evidence: `scripts/test-openspec-preflight-evidence.sh`, `openspec/changes/enforce-task-evidence-coverage/fixtures/openspec-preflight-evidence/README.md`, `openspec/changes/enforce-task-evidence-coverage/evidence/preflight-fixture-suite.txt`
- [x] I3 Add failure-message output that names the checked task, missing or invalid evidence path, and concrete remediation. [covers=openspec-governance.task-evidence-coverage.missing-evidence-fails,openspec-governance.task-evidence-coverage.invalid-evidence-output-actionable,openspec-governance.task-evidence-coverage.placeholder-evidence-fails]
  - Evidence: `scripts/openspec-preflight.sh`, `scripts/test-openspec-preflight-evidence.sh`, `openspec/changes/enforce-task-evidence-coverage/evidence/preflight-fixture-suite.txt`, `openspec/changes/enforce-task-evidence-coverage/evidence/implementation-diff.patch`
- [x] I4 Update OpenSpec gate/template documentation so task evidence coverage and placeholder rejection are visible before implementation starts. [covers=openspec-governance.task-evidence-coverage]
  - Evidence: `openspec/templates/verification.md`, `scripts/openspec-preflight.sh`, `openspec/changes/enforce-task-evidence-coverage/evidence/implementation-diff.patch`
- [x] V1 Run the active-change and archived-change fixture suite and save transcripts under `openspec/changes/enforce-task-evidence-coverage/evidence/` for change-local valid evidence, changed source/doc valid evidence, missing, untracked, empty, `pending`, `TODO`, `placeholder`, and disallowed external evidence path cases. [covers=openspec-governance.task-evidence-coverage]
  - Evidence: `openspec/changes/enforce-task-evidence-coverage/evidence/preflight-fixture-suite.txt`
- [x] V2 Run `scripts/openspec-preflight.sh` against a known-good change and each negative fixture; assert failure output includes the task text, missing/invalid path, and remediation; save all results under `openspec/changes/enforce-task-evidence-coverage/evidence/`. [covers=openspec-governance.task-evidence-coverage.checked-task-cites-valid-evidence,openspec-governance.task-evidence-coverage.missing-evidence-fails,openspec-governance.task-evidence-coverage.placeholder-evidence-fails]
  - Evidence: `scripts/test-openspec-preflight-evidence.sh`, `openspec/changes/enforce-task-evidence-coverage/evidence/preflight-fixture-suite.txt`, `openspec/changes/enforce-task-evidence-coverage/evidence/openspec-preflight.txt`
- [x] V3 Update this change's `verification.md` so every checked task appears verbatim with evidence paths before any task is checked complete. [covers=openspec-governance.task-evidence-coverage.checked-task-cites-valid-evidence]
  - Evidence: `openspec/changes/enforce-task-evidence-coverage/verification.md`, `openspec/changes/enforce-task-evidence-coverage/evidence/openspec-preflight.txt`

## Review Scope Snapshot

### `git diff HEAD -- scripts/openspec-preflight.sh scripts/test-openspec-preflight-evidence.sh openspec/templates/verification.md openspec/changes/enforce-task-evidence-coverage/fixtures/openspec-preflight-evidence/README.md`

- Status: captured
- Artifact: `openspec/changes/enforce-task-evidence-coverage/evidence/implementation-diff.patch`

## Verification Commands

### `bash -n scripts/openspec-preflight.sh scripts/test-openspec-preflight-evidence.sh`

- Status: pass
- Artifact: `openspec/changes/enforce-task-evidence-coverage/evidence/bash-n-preflight-scripts.txt`

### `scripts/test-openspec-preflight-evidence.sh`

- Status: pass
- Artifact: `openspec/changes/enforce-task-evidence-coverage/evidence/preflight-fixture-suite.txt`

### `openspec validate enforce-task-evidence-coverage --type change --strict --no-interactive`

- Status: pass
- Artifact: `openspec/changes/enforce-task-evidence-coverage/evidence/openspec-validate.txt`

### `scripts/openspec-preflight.sh enforce-task-evidence-coverage`

- Status: pass
- Artifact: `openspec/changes/enforce-task-evidence-coverage/evidence/openspec-preflight.txt`

### `openspec_gate stage=tasks change=enforce-task-evidence-coverage`

- Status: fail (diagnostic reviewer packet did not include saved evidence paths; transcript retained to avoid unsupported gate-status claims)
- Artifact: `openspec/changes/enforce-task-evidence-coverage/evidence/openspec-gate-tasks.txt`
