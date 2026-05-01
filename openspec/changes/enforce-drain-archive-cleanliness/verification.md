# Verification Evidence

## Implementation Evidence

- Changed file: `openspec/templates/verification.md`
- Changed file: `openspec/changes/enforce-drain-archive-cleanliness/evidence/i3-drain-instructions-template.txt`
- Changed file: `openspec/changes/enforce-drain-archive-cleanliness/evidence/openspec-preflight-i1-v1.txt`
- Changed file: `openspec/changes/enforce-drain-archive-cleanliness/tasks.md`
- Changed file: `openspec/changes/enforce-drain-archive-cleanliness/verification.md`

## Task Coverage

- [x] I1 Add a deterministic drain completion audit for active paths, archive path, and `.drain-state.md` absence. [covers=openspec-governance.drain-archive-cleanliness.clean-drain-passes,openspec-governance.drain-archive-cleanliness.leftover-active-path-fails] ✅ 20m (started: 2026-05-01T01:45:00Z → completed: 2026-05-01T02:05:33Z)
  - Evidence: `openspec/changes/enforce-drain-archive-cleanliness/evidence/i1-v1-drain-audit-fixtures.txt`
- [x] I2 Add archive-path consistency checks for archived verification/task coverage files. [covers=openspec-governance.drain-archive-cleanliness.archive-paths-used] ✅ 15m (started: 2026-05-01T01:53:00Z → completed: 2026-05-01T02:08:10Z)
  - Evidence: `openspec/changes/enforce-drain-archive-cleanliness/evidence/i2-drain-audit-archive-paths.txt`
- [x] I3 Update drain skill instructions and evidence template to require the post-archive audit transcript. [covers=openspec-governance.drain-archive-cleanliness] ✅ 10m (started: 2026-05-01T02:00:30Z → completed: 2026-05-01T02:10:39Z)
  - Evidence: `openspec/templates/verification.md`, `openspec/changes/enforce-drain-archive-cleanliness/evidence/i3-drain-instructions-template.txt`
- [x] V1 Add positive clean-drain and negative leftover-active fixtures. [covers=openspec-governance.drain-archive-cleanliness] ✅ 20m (started: 2026-05-01T01:45:00Z → completed: 2026-05-01T02:05:33Z)
  - Evidence: `openspec/changes/enforce-drain-archive-cleanliness/fixtures/run-drain-audit-fixtures.sh`, `openspec/changes/enforce-drain-archive-cleanliness/evidence/i1-v1-drain-audit-fixtures.txt`
- [x] V2 Run the audit fixtures and save transcripts. [covers=openspec-governance.drain-archive-cleanliness] ✅ 20m (started: 2026-05-01T01:45:00Z → completed: 2026-05-01T02:05:33Z)
  - Evidence: `openspec/changes/enforce-drain-archive-cleanliness/evidence/i1-v1-drain-audit-fixtures.txt`

## Verification Commands

- Command: `openspec/changes/enforce-drain-archive-cleanliness/fixtures/run-drain-audit-fixtures.sh > openspec/changes/enforce-drain-archive-cleanliness/evidence/i1-v1-drain-audit-fixtures.txt 2>&1`
- Artifact: `openspec/changes/enforce-drain-archive-cleanliness/evidence/i1-v1-drain-audit-fixtures.txt`
- Command: `openspec/changes/enforce-drain-archive-cleanliness/fixtures/run-drain-audit-fixtures.sh > openspec/changes/enforce-drain-archive-cleanliness/evidence/i2-drain-audit-archive-paths.txt 2>&1`
- Artifact: `openspec/changes/enforce-drain-archive-cleanliness/evidence/i2-drain-audit-archive-paths.txt`
- Command: `update Hermes openspec-drain skill with post-archive audit instruction and update openspec/templates/verification.md`
- Artifact: `openspec/changes/enforce-drain-archive-cleanliness/evidence/i3-drain-instructions-template.txt`
- Command: `scripts/openspec-preflight.sh enforce-drain-archive-cleanliness > openspec/changes/enforce-drain-archive-cleanliness/evidence/openspec-preflight-i1-v1.txt 2>&1`
- Artifact: `openspec/changes/enforce-drain-archive-cleanliness/evidence/openspec-preflight-i1-v1.txt`
