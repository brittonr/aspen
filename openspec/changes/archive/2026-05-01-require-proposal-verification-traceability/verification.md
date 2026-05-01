# Verification Evidence

## Implementation Evidence

- Changed file: `scripts/check-openspec-proposal-verification.py`
- Changed file: `scripts/openspec-preflight.sh`
- Changed file: `openspec/templates/proposal.md`
- Changed file: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/proposal.md`
- Changed file: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/design.md`
- Changed file: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/tasks.md`
- Changed file: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/verification.md`
- Changed file: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/fixtures/proposal-verification/run-proposal-verification-fixtures.sh`
- Changed file: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/proposal-verification-fixtures.txt`
- Changed file: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/py-compile-proposal-verification.txt`
- Changed file: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/bash-n-proposal-verification.txt`
- Changed file: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/proposal-gate-current-change.txt`
- Changed file: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/openspec-validate-final.json`
- Changed file: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/openspec-preflight-final.txt`

## Task Coverage

- [x] I1 Update proposal template/instructions with requirement-ID verification mapping guidance. [covers=openspec-governance.proposal-verification-traceability.cites-requirement-ids] ✅ 5m (started: 2026-05-01T02:40:00Z → completed: 2026-05-01T02:45:21Z)
  - Evidence: `openspec/templates/proposal.md`, `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/proposal.md`, `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/design.md`
- [x] I2 Add proposal gate checks for missing requirement verification and negative-path expectations. [covers=openspec-governance.proposal-verification-traceability.missing-positive-verification-fails,openspec-governance.proposal-verification-traceability.negative-behavior-needs-verification] ✅ 5m (started: 2026-05-01T02:40:00Z → completed: 2026-05-01T02:45:21Z)
  - Evidence: `scripts/check-openspec-proposal-verification.py`, `scripts/openspec-preflight.sh`, `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/proposal-gate-current-change.txt`
- [x] I3 Add scoped deferral parsing so `defer to design` is accepted only when paired with a requirement/scenario ID and rationale. [covers=openspec-governance.proposal-verification-traceability.cites-requirement-ids] ✅ 5m (started: 2026-05-01T02:40:00Z → completed: 2026-05-01T02:45:21Z)
  - Evidence: `scripts/check-openspec-proposal-verification.py`, `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/fixtures/proposal-verification/run-proposal-verification-fixtures.sh`, `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/proposal-verification-fixtures.txt`
- [x] V1 Add fixtures for complete mapping, missing ID mapping, missing negative path, valid deferral, and invalid deferral. [covers=openspec-governance.proposal-verification-traceability] ✅ 5m (started: 2026-05-01T02:40:00Z → completed: 2026-05-01T02:45:21Z)
  - Evidence: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/fixtures/proposal-verification/run-proposal-verification-fixtures.sh`
- [x] V2 Run proposal gate fixtures and save transcripts. [covers=openspec-governance.proposal-verification-traceability] ✅ 5m (started: 2026-05-01T02:40:00Z → completed: 2026-05-01T02:45:21Z)
  - Evidence: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/proposal-verification-fixtures.txt`
- [x] V3 Run `openspec validate require-proposal-verification-traceability --type change --strict --no-interactive` and `scripts/openspec-preflight.sh require-proposal-verification-traceability` after implementation evidence is staged; save transcripts. [covers=openspec-governance.proposal-verification-traceability] ✅ 5m (started: 2026-05-01T02:40:00Z → completed: 2026-05-01T02:45:21Z)
  - Evidence: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/openspec-validate-final.json`, `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/openspec-preflight-final.txt`

## Drain Verification Matrix

| Rail | Command | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| build | `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile scripts/check-openspec-proposal-verification.py` | scoped | `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/py-compile-proposal-verification.txt` | Python checker syntax/bytecode validation covers the new executable gate. | Full workspace build is not required for this governance-script-only change. |
| test | `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/fixtures/proposal-verification/run-proposal-verification-fixtures.sh` | scoped | `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/proposal-verification-fixtures.txt` | Fixtures cover complete mapping, missing ID mapping, missing negative path, valid deferral, and invalid deferral. | Future governance changes may add broader preflight integration fixtures. |
| format | `bash -n scripts/openspec-preflight.sh scripts/check-openspec-proposal-verification.py openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/fixtures/proposal-verification/run-proposal-verification-fixtures.sh` | scoped | `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/bash-n-proposal-verification.txt` | Shell syntax validation covers the edited shell preflight and fixture runner; Python syntax is covered by build rail. | `git diff --cached --check` runs as final whitespace check. |

## Verification Commands

### `scripts/check-openspec-proposal-verification.py require-proposal-verification-traceability --repo-root .`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/proposal-gate-current-change.txt`

### `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/fixtures/proposal-verification/run-proposal-verification-fixtures.sh`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/proposal-verification-fixtures.txt`

### `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile scripts/check-openspec-proposal-verification.py`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/py-compile-proposal-verification.txt`

### `bash -n scripts/openspec-preflight.sh scripts/check-openspec-proposal-verification.py openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/fixtures/proposal-verification/run-proposal-verification-fixtures.sh`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/bash-n-proposal-verification.txt`

### `openspec validate require-proposal-verification-traceability --json`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/openspec-validate-final.json`

### `scripts/openspec-preflight.sh require-proposal-verification-traceability`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-require-proposal-verification-traceability/evidence/openspec-preflight-final.txt`
