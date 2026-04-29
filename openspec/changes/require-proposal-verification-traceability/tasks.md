## 1. Implementation

- [ ] I1 Update proposal template/instructions with requirement-ID verification mapping guidance. [covers=openspec-governance.proposal-verification-traceability.cites-requirement-ids]
- [ ] I2 Add proposal gate checks for missing requirement verification and negative-path expectations. [covers=openspec-governance.proposal-verification-traceability.missing-positive-verification-fails,openspec-governance.proposal-verification-traceability.negative-behavior-needs-verification]
- [ ] I3 Add scoped deferral parsing so `defer to design` is accepted only when paired with a requirement/scenario ID and rationale. [covers=openspec-governance.proposal-verification-traceability.cites-requirement-ids]

## 2. Verification

- [ ] V1 Add fixtures for complete mapping, missing ID mapping, missing negative path, valid deferral, and invalid deferral. [covers=openspec-governance.proposal-verification-traceability]
- [ ] V2 Run proposal gate fixtures and save transcripts. [covers=openspec-governance.proposal-verification-traceability]
- [ ] V3 Run `openspec validate require-proposal-verification-traceability --type change --strict --no-interactive` and `scripts/openspec-preflight.sh require-proposal-verification-traceability` after implementation evidence is staged; save transcripts. [covers=openspec-governance.proposal-verification-traceability]
