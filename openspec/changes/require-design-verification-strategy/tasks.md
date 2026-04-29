## 1. Implementation

- [ ] I1 Update design template/instructions to require `## Verification Strategy` for spec-changing changes. [covers=openspec-governance.design-verification-strategy.mapped-strategy-present]
- [ ] I2 Extend the design gate to fail missing strategy sections and strategies that omit requirement/scenario ID references. [covers=openspec-governance.design-verification-strategy.mapped-strategy-present,openspec-governance.design-verification-strategy.missing-strategy-fails]
- [ ] I3 Extend the design gate to require negative-path checks or explicit defer-with-rationale entries when changed specs include negative behavior. [covers=openspec-governance.design-verification-strategy.negative-paths-planned]

## 2. Verification

- [ ] V1 Add fixtures for valid mapped strategy, missing strategy, missing requirement/scenario ID, and missing negative-path/defer rationale. [covers=openspec-governance.design-verification-strategy]
- [ ] V2 Run the design gate on fixtures and save transcripts. [covers=openspec-governance.design-verification-strategy]
- [ ] V3 Run the design gate against a real sample change with mapped positive and negative checks; save the transcript. [covers=openspec-governance.design-verification-strategy.mapped-strategy-present,openspec-governance.design-verification-strategy.negative-paths-planned]
