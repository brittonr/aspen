# Verification Evidence

## Implementation Evidence

- Changed file: `scripts/check-openspec-verification-freshness.py`
- Changed file: `openspec/specs/openspec-governance/spec.md`
- Changed file: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/proposal.md`
- Changed file: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/design.md`
- Changed file: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/tasks.md`
- Changed file: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/verification.md`
- Changed file: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/fixtures/verification-freshness/run-verification-freshness-fixtures.sh`
- Changed file: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/verification-freshness-fixtures.txt`
- Changed file: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/py-compile-verification-freshness.txt`
- Changed file: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/bash-n-verification-freshness.txt`
- Changed file: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/verification-freshness-current-change.txt`
- Changed file: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/openspec-validate-final.json`
- Changed file: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/openspec-preflight-final.txt`
- Changed file: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/openspec-drain-audit-final.txt`

## Task Coverage

- [x] I1 Add freshness checks for stale active paths, placeholder transcripts, and evidence generated before final reviewed source/doc diffs, scoped to active changes and newly archived changes touched in the current diff. [covers=openspec-governance.verification-index-freshness.stale-active-path-fails,openspec-governance.verification-index-freshness.placeholder-preflight-fails,openspec-governance.verification-index-freshness.generated-before-final-diff-fails] ✅ 7m (started: 2026-05-01T02:50:00Z → completed: 2026-05-01T02:57:13Z)
  - Evidence: `scripts/check-openspec-verification-freshness.py`, `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/verification-freshness-current-change.txt`
- [x] I2 Add an exception that allows stale active paths only inside saved diff artifacts used as historical review context. [covers=openspec-governance.verification-index-freshness.saved-diff-may-contain-historical-paths] ✅ 7m (started: 2026-05-01T02:50:00Z → completed: 2026-05-01T02:57:13Z)
  - Evidence: `scripts/check-openspec-verification-freshness.py`, `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/verification-freshness-fixtures.txt`
- [x] I3 Update archive/preflight guidance and the verification template to capture final preflight after staging and path rewrite. [covers=openspec-governance.verification-index-freshness.stale-active-path-fails,openspec-governance.verification-index-freshness.placeholder-preflight-fails,openspec-governance.verification-index-freshness.generated-before-final-diff-fails] ✅ 7m (started: 2026-05-01T02:50:00Z → completed: 2026-05-01T02:57:13Z)
  - Evidence: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/verification-freshness-current-change.txt`
- [x] V1 Add integration verification fixture suite with evidence for fresh pass, stale active path fail, stale active path inside saved diff pass, placeholder preflight fail, generated-before-final-diff fail, and out-of-scope old archive ignored. [covers=openspec-governance.verification-index-freshness.fresh-index-passes,openspec-governance.verification-index-freshness.stale-active-path-fails,openspec-governance.verification-index-freshness.saved-diff-may-contain-historical-paths,openspec-governance.verification-index-freshness.placeholder-preflight-fails,openspec-governance.verification-index-freshness.generated-before-final-diff-fails] ✅ 7m (started: 2026-05-01T02:50:00Z → completed: 2026-05-01T02:57:13Z)
  - Evidence: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/fixtures/verification-freshness/run-verification-freshness-fixtures.sh`
- [x] V2 Run integration verification preflight freshness fixtures and save evidence transcripts. [covers=openspec-governance.verification-index-freshness.fresh-index-passes,openspec-governance.verification-index-freshness.stale-active-path-fails,openspec-governance.verification-index-freshness.saved-diff-may-contain-historical-paths,openspec-governance.verification-index-freshness.placeholder-preflight-fails,openspec-governance.verification-index-freshness.generated-before-final-diff-fails] ✅ 7m (started: 2026-05-01T02:50:00Z → completed: 2026-05-01T02:57:13Z)
  - Evidence: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/verification-freshness-fixtures.txt`

## Drain Verification Matrix

| Rail | Command | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| build | `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile scripts/check-openspec-verification-freshness.py` | scoped | `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/py-compile-verification-freshness.txt` | Python checker syntax/bytecode validation covers the new executable gate. | Full workspace build is not required for this governance-script-only change. |
| test | `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/fixtures/verification-freshness/run-verification-freshness-fixtures.sh` | scoped | `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/verification-freshness-fixtures.txt` | Fixtures cover fresh pass, stale active path failure, saved-diff exception, placeholder preflight failure, generated-before-final-diff failure, and out-of-scope old archive. | Future governance changes may add broader archive integration fixtures. |
| format | `bash -n scripts/openspec-preflight.sh openspec/changes/archive/2026-05-01-verify-verification-index-freshness/fixtures/verification-freshness/run-verification-freshness-fixtures.sh` | scoped | `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/bash-n-verification-freshness.txt` | Shell syntax validation covers edited preflight and fixture runner; Python syntax is covered by build rail. | `git diff --cached --check` runs as final whitespace check. |

## Verification Commands

### `scripts/check-openspec-verification-freshness.py verify-verification-index-freshness --repo-root .`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/verification-freshness-current-change.txt`

### `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/fixtures/verification-freshness/run-verification-freshness-fixtures.sh`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/verification-freshness-fixtures.txt`

### `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile scripts/check-openspec-verification-freshness.py`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/py-compile-verification-freshness.txt`

### `bash -n scripts/openspec-preflight.sh openspec/changes/archive/2026-05-01-verify-verification-index-freshness/fixtures/verification-freshness/run-verification-freshness-fixtures.sh`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/bash-n-verification-freshness.txt`

### `openspec validate verify-verification-index-freshness --json`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/openspec-validate-final.json`

### `scripts/openspec-preflight.sh verify-verification-index-freshness`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/openspec-preflight-final.txt`

### `scripts/openspec-drain-audit.sh --archive openspec/changes/archive/2026-05-01-verify-verification-index-freshness`

- Status: pass
- Artifact: `openspec/changes/archive/2026-05-01-verify-verification-index-freshness/evidence/openspec-drain-audit-final.txt`
