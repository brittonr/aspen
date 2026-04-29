## 1. Implementation

- [ ] I1 Add checkpoint template/guidance for unresolved review questions. [covers=openspec-governance.review-oracle-checkpoints.records-ambiguity]
- [ ] I2 Teach done-review to flag ambiguous completion without checkpoint. [covers=openspec-governance.review-oracle-checkpoints.missing-checkpoint-flagged]
- [ ] I3 Teach review guidance/checker to accept concrete blocker reports that stop without claiming completion. [covers=openspec-governance.review-oracle-checkpoints.concrete-blocker-acceptable]
- [ ] I4 Add review-metrics promotion handling so repeated human-route findings can trigger checkpoint requirements. [covers=openspec-governance.review-oracle-checkpoints.missing-checkpoint-flagged,openspec-governance.review-oracle-checkpoints.records-ambiguity]

## 2. Verification

- [ ] V1 Add positive checkpoint, negative missing-checkpoint, review-metrics-triggered checkpoint, and concrete-blocker accepted fixtures. [covers=openspec-governance.review-oracle-checkpoints]
- [ ] V2 Run reviewer/gate fixtures and save transcripts. [covers=openspec-governance.review-oracle-checkpoints]
