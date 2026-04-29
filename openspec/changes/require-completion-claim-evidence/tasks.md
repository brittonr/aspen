## 1. Implementation

- [ ] I1 Add high-risk completion-claim detection to done-review, or to a checker that done-review invokes automatically. [covers=openspec-governance.completion-claim-evidence.evidence-backed-claim-passes,openspec-governance.completion-claim-evidence.unsupported-claim-fails]
- [ ] I2 Update prompt/checklist guidance to require status/list/check evidence before strong completion claims. [covers=openspec-governance.completion-claim-evidence]

## 2. Verification

- [ ] V1 Add tests for evidence-backed claims, unsupported claims, and uncertain-summary allowed behavior. [covers=openspec-governance.completion-claim-evidence.evidence-backed-claim-passes,openspec-governance.completion-claim-evidence.unsupported-claim-fails,openspec-governance.completion-claim-evidence.uncertain-summary-allowed]
- [ ] V2 Add claim-family fixtures for clean status, queue empty, all checks pass, archived, and validated claims; save transcripts. [covers=openspec-governance.completion-claim-evidence]
