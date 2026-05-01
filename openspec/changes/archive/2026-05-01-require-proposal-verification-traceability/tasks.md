## 1. Implementation

- [x] I1 Update proposal template/instructions with requirement-ID verification mapping guidance. [covers=openspec-governance.proposal-verification-traceability.cites-requirement-ids] ✅ 5m (started: 2026-05-01T02:40:00Z → completed: 2026-05-01T02:45:21Z)
- [x] I2 Add proposal gate checks for missing requirement verification and negative-path expectations. [covers=openspec-governance.proposal-verification-traceability.missing-positive-verification-fails,openspec-governance.proposal-verification-traceability.negative-behavior-needs-verification] ✅ 5m (started: 2026-05-01T02:40:00Z → completed: 2026-05-01T02:45:21Z)
- [x] I3 Add scoped deferral parsing so `defer to design` is accepted only when paired with a requirement/scenario ID and rationale. [covers=openspec-governance.proposal-verification-traceability.cites-requirement-ids] ✅ 5m (started: 2026-05-01T02:40:00Z → completed: 2026-05-01T02:45:21Z)

## 2. Verification

- [x] V1 Add fixtures for complete mapping, missing ID mapping, missing negative path, valid deferral, and invalid deferral. [covers=openspec-governance.proposal-verification-traceability] ✅ 5m (started: 2026-05-01T02:40:00Z → completed: 2026-05-01T02:45:21Z)
- [x] V2 Run proposal gate fixtures and save transcripts. [covers=openspec-governance.proposal-verification-traceability] ✅ 5m (started: 2026-05-01T02:40:00Z → completed: 2026-05-01T02:45:21Z)
- [x] V3 Run `openspec validate require-proposal-verification-traceability --type change --strict --no-interactive` and `scripts/openspec-preflight.sh require-proposal-verification-traceability` after implementation evidence is staged; save transcripts. [covers=openspec-governance.proposal-verification-traceability] ✅ 5m (started: 2026-05-01T02:40:00Z → completed: 2026-05-01T02:45:21Z)
