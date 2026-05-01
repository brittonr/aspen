# Verification Evidence

## Implementation Evidence

- Changed file: `scripts/check-openspec-drain-verification.py`
- Changed file: `scripts/openspec-preflight.sh`
- Changed file: `openspec/templates/verification.md`
- Changed file: `openspec/changes/require-drain-verification-cycle-evidence/design.md`
- Changed file: `openspec/changes/require-drain-verification-cycle-evidence/fixtures/drain-verification/run-drain-verification-fixtures.sh`
- Changed file: `openspec/changes/require-drain-verification-cycle-evidence/evidence/drain-verification-fixtures.txt`
- Changed file: `openspec/changes/require-drain-verification-cycle-evidence/evidence/py-compile-drain-verification.txt`
- Changed file: `openspec/changes/require-drain-verification-cycle-evidence/evidence/openspec-preflight-final.txt`
- Changed file: `openspec/changes/require-drain-verification-cycle-evidence/tasks.md`
- Changed file: `openspec/changes/require-drain-verification-cycle-evidence/verification.md`

## Task Coverage

- [x] I1 Add a drain verification matrix section to the verification template and drain instructions, including rail, command, status, artifact, scope rationale, and next-best-check fields. [covers=openspec-governance.drain-verification-cycle.full-cycle-recorded,openspec-governance.drain-verification-cycle.scoped-alternative-recorded,openspec-governance.drain-verification-cycle.doc-only-bypass-explicit] ✅ 10m (started: 2026-05-01T02:21:30Z → completed: 2026-05-01T02:31:45Z)
  - Evidence: `openspec/templates/verification.md`, `openspec/changes/require-drain-verification-cycle-evidence/evidence/drain-verification-fixtures.txt`
- [x] I2 Extend preflight or done-review to flag missing matrix entries for checked implementation tasks and before final verification tasks are checked, while allowing explicit doc-only bypasses with no source changes. [covers=openspec-governance.drain-verification-cycle.missing-cycle-fails,openspec-governance.drain-verification-cycle.doc-only-bypass-explicit,openspec-governance.drain-verification-cycle.final-verification-requires-matrix-first] ✅ 10m (started: 2026-05-01T02:21:30Z → completed: 2026-05-01T02:31:45Z)
  - Evidence: `scripts/check-openspec-drain-verification.py`, `scripts/openspec-preflight.sh`, `openspec/changes/require-drain-verification-cycle-evidence/evidence/drain-verification-fixtures.txt`
- [x] V1 Add full-cycle, scoped-alternative, doc-only-bypass, missing-cycle, and final-verification-before-matrix fixtures. [covers=openspec-governance.drain-verification-cycle.full-cycle-recorded,openspec-governance.drain-verification-cycle.scoped-alternative-recorded,openspec-governance.drain-verification-cycle.doc-only-bypass-explicit,openspec-governance.drain-verification-cycle.missing-cycle-fails,openspec-governance.drain-verification-cycle.final-verification-requires-matrix-first] ✅ 10m (started: 2026-05-01T02:21:30Z → completed: 2026-05-01T02:31:45Z)
  - Evidence: `openspec/changes/require-drain-verification-cycle-evidence/fixtures/drain-verification/run-drain-verification-fixtures.sh`
- [x] V2 Run checks over fixtures and save transcripts. [covers=openspec-governance.drain-verification-cycle.full-cycle-recorded,openspec-governance.drain-verification-cycle.scoped-alternative-recorded,openspec-governance.drain-verification-cycle.doc-only-bypass-explicit,openspec-governance.drain-verification-cycle.missing-cycle-fails,openspec-governance.drain-verification-cycle.final-verification-requires-matrix-first] ✅ 10m (started: 2026-05-01T02:21:30Z → completed: 2026-05-01T02:31:45Z)
  - Evidence: `openspec/changes/require-drain-verification-cycle-evidence/evidence/drain-verification-fixtures.txt`

## Drain Verification Matrix

| Rail | Command | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| build | `python3 -m py_compile scripts/check-openspec-drain-verification.py` | scoped | `openspec/changes/require-drain-verification-cycle-evidence/evidence/py-compile-drain-verification.txt` | dependency-light Python checker and shell/preflight wiring changed; targeted syntax check covers buildability of the new checker | run full workspace build before release |
| test | `openspec/changes/require-drain-verification-cycle-evidence/fixtures/drain-verification/run-drain-verification-fixtures.sh` | scoped | `openspec/changes/require-drain-verification-cycle-evidence/evidence/drain-verification-fixtures.txt` | fixture suite covers full-cycle, scoped, doc-only, missing-cycle, and incomplete-matrix behavior | run full preflight suite before release |
| format | `git diff --cached --check` | scoped | `openspec/changes/require-drain-verification-cycle-evidence/evidence/openspec-preflight-final.txt` | staged whitespace check is the relevant format rail for script/template changes in this slice | run nix fmt before release |

## Verification Commands

- Command: `openspec/changes/require-drain-verification-cycle-evidence/fixtures/drain-verification/run-drain-verification-fixtures.sh > openspec/changes/require-drain-verification-cycle-evidence/evidence/drain-verification-fixtures.txt 2>&1`
- Artifact: `openspec/changes/require-drain-verification-cycle-evidence/evidence/drain-verification-fixtures.txt`
- Command: `python3 -m py_compile scripts/check-openspec-drain-verification.py > openspec/changes/require-drain-verification-cycle-evidence/evidence/py-compile-drain-verification.txt 2>&1`
- Artifact: `openspec/changes/require-drain-verification-cycle-evidence/evidence/py-compile-drain-verification.txt`
- Command: `scripts/openspec-preflight.sh require-drain-verification-cycle-evidence > openspec/changes/require-drain-verification-cycle-evidence/evidence/openspec-preflight-final.txt 2>&1`
- Artifact: `openspec/changes/require-drain-verification-cycle-evidence/evidence/openspec-preflight-final.txt`
