# Verification Evidence

## Implementation Evidence

- Changed file: `scripts/check-openspec-task-size.py`
- Changed file: `scripts/openspec-preflight.sh`
- Changed file: `openspec/templates/tasks.md`
- Changed file: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/proposal.md`
- Changed file: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/design.md`
- Changed file: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/tasks.md`
- Changed file: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/verification.md`
- Changed file: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/fixtures/task-size/run-task-size-fixtures.sh`
- Changed file: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/task-size-fixtures.txt`
- Changed file: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/py-compile-task-size.txt`
- Changed file: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/bash-n-task-size.txt`
- Changed file: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/task-size-current-change.txt`
- Changed file: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/tasks-template-guidance.txt`
- Changed file: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/openspec-validate-final.json`
- Changed file: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/openspec-preflight-final.txt`

## Task Coverage

- [x] I1 Update tasks template and instructions with bounded-task guidance and examples. [covers=openspec-governance.task-size-and-ordering.bounded-implementation-task-passes] ✅ 6m (started: 2026-05-01T02:44:30Z → completed: 2026-05-01T02:50:42Z)
  - Evidence: `openspec/templates/tasks.md`, `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/tasks-template-guidance.txt`
- [x] I2 Update tasks template and instructions with oversized-task split guidance and prerequisite-order guidance. [covers=openspec-governance.task-size-and-ordering.oversized-implementation-task-flagged,openspec-governance.task-size-and-ordering.dependency-order-explicit] ✅ 6m (started: 2026-05-01T02:44:30Z → completed: 2026-05-01T02:50:42Z)
  - Evidence: `openspec/templates/tasks.md`, `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/tasks-template-guidance.txt`
- [x] I3 Add tasks-gate parsing for requirement/scenario fan-out and bounded implementation task acceptance. [covers=openspec-governance.task-size-and-ordering.bounded-implementation-task-passes] ✅ 6m (started: 2026-05-01T02:44:30Z → completed: 2026-05-01T02:50:42Z)
  - Evidence: `scripts/check-openspec-task-size.py`, `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/task-size-current-change.txt`
- [x] I4 Add configured fan-out threshold and warning/fail severity policy. [covers=openspec-governance.task-size-and-ordering.oversized-implementation-task-flagged] ✅ 6m (started: 2026-05-01T02:44:30Z → completed: 2026-05-01T02:50:42Z)
  - Evidence: `scripts/check-openspec-task-size.py`, `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/task-size-fixtures.txt`
- [x] I5 Add emitted diagnostics and split guidance for oversized implementation tasks detected by the fan-out policy. [covers=openspec-governance.task-size-and-ordering.oversized-implementation-task-flagged] ✅ 6m (started: 2026-05-01T02:44:30Z → completed: 2026-05-01T02:50:42Z)
  - Evidence: `scripts/check-openspec-task-size.py`, `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/task-size-fixtures.txt`
- [x] I6 Add tasks-gate exception handling for explicitly labeled broad integration verification tasks. [covers=openspec-governance.task-size-and-ordering.integration-verification-may-be-broad] ✅ 6m (started: 2026-05-01T02:44:30Z → completed: 2026-05-01T02:50:42Z)
  - Evidence: `scripts/check-openspec-task-size.py`, `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/task-size-fixtures.txt`
- [x] I7 Add tasks-gate dependency-order and prerequisite-note checks. [covers=openspec-governance.task-size-and-ordering.dependency-order-explicit] ✅ 6m (started: 2026-05-01T02:44:30Z → completed: 2026-05-01T02:50:42Z)
  - Evidence: `scripts/check-openspec-task-size.py`, `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/task-size-fixtures.txt`
- [x] V1 Add integration verification fixture suite with evidence for bounded, oversized, integration, out-of-order, and ambiguous-dependency task lists. [covers=openspec-governance.task-size-and-ordering.bounded-implementation-task-passes,openspec-governance.task-size-and-ordering.oversized-implementation-task-flagged,openspec-governance.task-size-and-ordering.integration-verification-may-be-broad,openspec-governance.task-size-and-ordering.dependency-order-explicit] ✅ 6m (started: 2026-05-01T02:44:30Z → completed: 2026-05-01T02:50:42Z)
  - Evidence: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/fixtures/task-size/run-task-size-fixtures.sh`
- [x] V2 Run tasks gate on fixtures and save transcripts. [covers=openspec-governance.task-size-and-ordering] ✅ 6m (started: 2026-05-01T02:44:30Z → completed: 2026-05-01T02:50:42Z)
  - Evidence: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/task-size-fixtures.txt`
- [x] V3 Review or test the updated task template/instructions and save evidence that bounded, oversized, and prerequisite guidance is present. [covers=openspec-governance.task-size-and-ordering.bounded-implementation-task-passes,openspec-governance.task-size-and-ordering.oversized-implementation-task-flagged,openspec-governance.task-size-and-ordering.dependency-order-explicit] ✅ 6m (started: 2026-05-01T02:44:30Z → completed: 2026-05-01T02:50:42Z)
  - Evidence: `openspec/templates/tasks.md`, `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/tasks-template-guidance.txt`

## Drain Verification Matrix

| Rail | Command | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| build | `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile scripts/check-openspec-task-size.py` | scoped | `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/py-compile-task-size.txt` | Python checker syntax/bytecode validation covers the new executable gate. | Full workspace build is not required for this governance-script-only change. |
| test | `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/fixtures/task-size/run-task-size-fixtures.sh` | scoped | `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/task-size-fixtures.txt` | Fixtures cover bounded tasks, oversized fan-out, integration-proof exception, out-of-order references, ambiguous dependencies, and explicit prerequisites. | Future governance changes may add broader preflight integration fixtures. |
| format | `bash -n scripts/openspec-preflight.sh openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/fixtures/task-size/run-task-size-fixtures.sh` | scoped | `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/bash-n-task-size.txt` | Shell syntax validation covers edited preflight and fixture runner; Python syntax is covered by build rail. | `git diff --cached --check` runs as final whitespace check. |

## Verification Commands

### `scripts/check-openspec-task-size.py split-oversized-openspec-tasks --repo-root .`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/task-size-current-change.txt`

### `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/fixtures/task-size/run-task-size-fixtures.sh`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/task-size-fixtures.txt`

### `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile scripts/check-openspec-task-size.py`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/py-compile-task-size.txt`

### `bash -n scripts/openspec-preflight.sh openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/fixtures/task-size/run-task-size-fixtures.sh`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/bash-n-task-size.txt`

### `openspec validate split-oversized-openspec-tasks --json`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/openspec-validate-final.json`

### `scripts/openspec-preflight.sh split-oversized-openspec-tasks`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-split-oversized-openspec-tasks/evidence/openspec-preflight-final.txt`
