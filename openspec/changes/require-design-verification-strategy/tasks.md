## 1. Implementation

- [x] I1 Update design template/instructions to require `## Verification Strategy` for spec-changing changes. [covers=openspec-governance.design-verification-strategy.mapped-strategy-present] ✅ 5m (started: 2026-05-01T02:20:30Z → completed: 2026-05-01T02:25:28Z)
- [x] I2 Extend the design gate to fail missing strategy sections and strategies that omit requirement/scenario ID references. [covers=openspec-governance.design-verification-strategy.mapped-strategy-present,openspec-governance.design-verification-strategy.missing-strategy-fails] ✅ 7m (started: 2026-05-01T02:20:30Z → completed: 2026-05-01T02:27:24Z)
- [x] I3 Extend the design gate to require negative-path checks or explicit defer-with-rationale entries when changed specs include negative behavior. [covers=openspec-governance.design-verification-strategy.negative-paths-planned] ✅ 7m (started: 2026-05-01T02:20:30Z → completed: 2026-05-01T02:27:24Z)

## 2. Verification

- [x] V1 Add fixtures for valid mapped strategy, missing strategy, missing requirement/scenario ID, and missing negative-path/defer rationale. [covers=openspec-governance.design-verification-strategy] ✅ 7m (started: 2026-05-01T02:20:30Z → completed: 2026-05-01T02:27:24Z)
- [x] V2 Run the design gate on fixtures and save transcripts. [covers=openspec-governance.design-verification-strategy] ✅ 7m (started: 2026-05-01T02:20:30Z → completed: 2026-05-01T02:27:24Z)
- [x] V3 Run the design gate against a real sample change with mapped positive and negative checks; save the transcript. [covers=openspec-governance.design-verification-strategy.mapped-strategy-present,openspec-governance.design-verification-strategy.negative-paths-planned] ✅ 7m (started: 2026-05-01T02:20:30Z → completed: 2026-05-01T02:27:24Z)
