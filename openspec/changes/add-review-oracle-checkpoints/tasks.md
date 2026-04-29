## 1. Implementation

- [ ] I1 Add checkpoint template/guidance for unresolved review questions. [covers=openspec-governance.review-oracle-checkpoints.records-ambiguity]
- [ ] I2 Teach done-review to flag ambiguous completion without checkpoint. [covers=openspec-governance.review-oracle-checkpoints.missing-checkpoint-flagged]
- [ ] I3 Teach OpenSpec gates to recognize checkpoint records and flag ambiguous stage completion without checkpoint. [covers=openspec-governance.review-oracle-checkpoints.missing-checkpoint-flagged,openspec-governance.review-oracle-checkpoints.records-ambiguity]
- [ ] I4 Teach OpenSpec gate and review guidance/checker to accept concrete blocker reports that stop without claiming completion. [covers=openspec-governance.review-oracle-checkpoints.concrete-blocker-acceptable]
- [ ] I5 Add review-metrics promotion handling so repeated human-route findings can trigger checkpoint requirements. [covers=openspec-governance.review-oracle-checkpoints.missing-checkpoint-flagged,openspec-governance.review-oracle-checkpoints.records-ambiguity]

## 2. Verification

- [ ] V1 Add done-review and OpenSpec gate fixtures for positive checkpoint, negative missing-checkpoint, review-metrics-triggered checkpoint, and concrete-blocker accepted paths. [covers=openspec-governance.review-oracle-checkpoints]
- [ ] V2 Run reviewer/gate fixtures and save transcripts. [covers=openspec-governance.review-oracle-checkpoints]
