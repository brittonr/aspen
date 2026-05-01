# Verification Evidence

## Implementation Evidence

- Changed file: `scripts/check-openspec-design-verification.py`
- Changed file: `scripts/openspec-preflight.sh`
- Changed file: `openspec/changes/archive/2026-05-01-require-design-verification-strategy/fixtures/design-verification/run-design-verification-fixtures.sh`
- Changed file: `openspec/changes/archive/2026-05-01-require-design-verification-strategy/evidence/design-verification-fixtures.txt`
- Changed file: `openspec/changes/archive/2026-05-01-require-design-verification-strategy/evidence/real-change-design-verification.txt`
- Changed file: `openspec/changes/archive/2026-05-01-require-design-verification-strategy/evidence/openspec-preflight-final.txt`
- Changed file: `openspec/changes/archive/2026-05-01-require-design-verification-strategy/tasks.md`
- Changed file: `openspec/changes/archive/2026-05-01-require-design-verification-strategy/verification.md`

## Task Coverage

- [x] I1 Update design template/instructions to require `## Verification Strategy` for spec-changing changes. [covers=openspec-governance.design-verification-strategy.mapped-strategy-present] ✅ 5m (started: 2026-05-01T02:20:30Z → completed: 2026-05-01T02:25:28Z)
  - Evidence: `openspec/changes/archive/2026-05-01-require-design-verification-strategy/evidence/i1-design-template.txt`
- [x] I2 Extend the design gate to fail missing strategy sections and strategies that omit requirement/scenario ID references. [covers=openspec-governance.design-verification-strategy.mapped-strategy-present,openspec-governance.design-verification-strategy.missing-strategy-fails] ✅ 7m (started: 2026-05-01T02:20:30Z → completed: 2026-05-01T02:27:24Z)
  - Evidence: `scripts/check-openspec-design-verification.py`, `scripts/openspec-preflight.sh`, `openspec/changes/archive/2026-05-01-require-design-verification-strategy/evidence/design-verification-fixtures.txt`
- [x] I3 Extend the design gate to require negative-path checks or explicit defer-with-rationale entries when changed specs include negative behavior. [covers=openspec-governance.design-verification-strategy.negative-paths-planned] ✅ 7m (started: 2026-05-01T02:20:30Z → completed: 2026-05-01T02:27:24Z)
  - Evidence: `scripts/check-openspec-design-verification.py`, `openspec/changes/archive/2026-05-01-require-design-verification-strategy/evidence/design-verification-fixtures.txt`
- [x] V1 Add fixtures for valid mapped strategy, missing strategy, missing requirement/scenario ID, and missing negative-path/defer rationale. [covers=openspec-governance.design-verification-strategy] ✅ 7m (started: 2026-05-01T02:20:30Z → completed: 2026-05-01T02:27:24Z)
  - Evidence: `openspec/changes/archive/2026-05-01-require-design-verification-strategy/fixtures/design-verification/run-design-verification-fixtures.sh`
- [x] V2 Run the design gate on fixtures and save transcripts. [covers=openspec-governance.design-verification-strategy] ✅ 7m (started: 2026-05-01T02:20:30Z → completed: 2026-05-01T02:27:24Z)
  - Evidence: `openspec/changes/archive/2026-05-01-require-design-verification-strategy/evidence/design-verification-fixtures.txt`
- [x] V3 Run the design gate against a real sample change with mapped positive and negative checks; save the transcript. [covers=openspec-governance.design-verification-strategy.mapped-strategy-present,openspec-governance.design-verification-strategy.negative-paths-planned] ✅ 7m (started: 2026-05-01T02:20:30Z → completed: 2026-05-01T02:27:24Z)
  - Evidence: `openspec/changes/archive/2026-05-01-require-design-verification-strategy/evidence/real-change-design-verification.txt`

## Verification Commands

- Command: `openspec/changes/archive/2026-05-01-require-design-verification-strategy/fixtures/design-verification/run-design-verification-fixtures.sh > openspec/changes/archive/2026-05-01-require-design-verification-strategy/evidence/design-verification-fixtures.txt 2>&1`
- Artifact: `openspec/changes/archive/2026-05-01-require-design-verification-strategy/evidence/design-verification-fixtures.txt`
- Command: `scripts/check-openspec-design-verification.py require-design-verification-strategy --repo-root . > openspec/changes/archive/2026-05-01-require-design-verification-strategy/evidence/real-change-design-verification.txt 2>&1`
- Artifact: `openspec/changes/archive/2026-05-01-require-design-verification-strategy/evidence/real-change-design-verification.txt`
- Command: `scripts/openspec-preflight.sh require-design-verification-strategy > openspec/changes/archive/2026-05-01-require-design-verification-strategy/evidence/openspec-preflight-final.txt 2>&1`
- Artifact: `openspec/changes/archive/2026-05-01-require-design-verification-strategy/evidence/openspec-preflight-final.txt`
