# Verification Evidence

## Implementation Evidence

- Changed file: `openspec/changes/detect-delta-spec-consistency/tasks.md`
- Changed file: `openspec/changes/detect-delta-spec-consistency/verification.md`
- Changed file: `openspec/changes/detect-delta-spec-consistency/evidence/v2-delta-consistency-fixtures.txt`
- Changed file: `openspec/changes/detect-delta-spec-consistency/fixtures/run-delta-consistency-fixtures.sh`

## Task Coverage

- [x] I1 Add a delta-spec consistency checker for added requirement/scenario IDs, modified-target existence, and removed-target existence. [covers=openspec-governance.delta-spec-consistency.added-requirements-have-ids,openspec-governance.delta-spec-consistency.modified-requirement-exists,openspec-governance.delta-spec-consistency.missing-modified-target-fails,openspec-governance.delta-spec-consistency.removed-requirement-exists]
  - Evidence: `openspec/changes/detect-delta-spec-consistency/evidence/i1-delta-consistency-checker.txt`

- [x] I2 Wire the checker into proposal/spec validation for active changes. [covers=openspec-governance.delta-spec-consistency.added-requirements-have-ids,openspec-governance.delta-spec-consistency.modified-requirement-exists,openspec-governance.delta-spec-consistency.missing-modified-target-fails,openspec-governance.delta-spec-consistency.removed-requirement-exists]
  - Evidence: `openspec/changes/detect-delta-spec-consistency/evidence/i2-preflight-wiring.txt`

- [x] I3 Add a `Migration note:` repair/migration exemption for modified requirements that intentionally repair missing or legacy main-spec entries. [covers=openspec-governance.delta-spec-consistency.migration-note-permits-repair]
  - Evidence: `openspec/changes/detect-delta-spec-consistency/evidence/v2-delta-consistency-fixtures.txt`

- [x] I4 Add warning logic for conflicting feature-contract phrases across proposal, specs, and design. [covers=openspec-governance.delta-spec-consistency.conflicting-feature-contracts-warn]
  - Evidence: `openspec/changes/detect-delta-spec-consistency/evidence/v2-delta-consistency-fixtures.txt`

- [x] V1 Add fixtures for valid add, missing requirement ID, missing scenario ID, valid modify, missing modified target, valid removal, missing removal target, migration-note positive, and conflicting feature-contract warning. [covers=openspec-governance.delta-spec-consistency.added-requirements-have-ids,openspec-governance.delta-spec-consistency.modified-requirement-exists,openspec-governance.delta-spec-consistency.missing-modified-target-fails,openspec-governance.delta-spec-consistency.removed-requirement-exists,openspec-governance.delta-spec-consistency.migration-note-permits-repair,openspec-governance.delta-spec-consistency.conflicting-feature-contracts-warn]
  - Evidence: `openspec/changes/detect-delta-spec-consistency/fixtures/run-delta-consistency-fixtures.sh`

- [x] V2 Run validation on fixtures and save transcripts. [covers=openspec-governance.delta-spec-consistency.added-requirements-have-ids,openspec-governance.delta-spec-consistency.modified-requirement-exists,openspec-governance.delta-spec-consistency.missing-modified-target-fails,openspec-governance.delta-spec-consistency.removed-requirement-exists,openspec-governance.delta-spec-consistency.migration-note-permits-repair,openspec-governance.delta-spec-consistency.conflicting-feature-contracts-warn]
  - Evidence: `openspec/changes/detect-delta-spec-consistency/evidence/v2-delta-consistency-fixtures.txt`
## Verification Commands

### `python3 scripts/check-openspec-delta-consistency.py detect-delta-spec-consistency`

- Status: pass
- Artifact: `openspec/changes/detect-delta-spec-consistency/evidence/i1-delta-consistency-checker.txt`

### `scripts/openspec-preflight.sh detect-delta-spec-consistency` and `scripts/test-openspec-preflight-evidence.sh`

- Status: pass
- Artifact: `openspec/changes/detect-delta-spec-consistency/evidence/i2-preflight-wiring.txt`

### `openspec/changes/detect-delta-spec-consistency/fixtures/run-delta-consistency-fixtures.sh`

- Status: pass
- Artifact: `openspec/changes/detect-delta-spec-consistency/evidence/v2-delta-consistency-fixtures.txt`
