# Artifact Registry Delta

## MODIFIED Requirements

### Requirement: Checkpoints are explicit artifacts
r[molten.evidence.chain_hashing.control_plane_checkpoints] Accepted control-plane chain heads SHOULD be represented by canonical checkpoint artifacts or receipts that name chain scope/id/epoch, prior checkpoint ref, new head ref, verified range, policy/membership refs, and the control-plane mode that accepted the head. The mode MUST name the mechanism that actually accepted the checkpoint (`local-checkpoint` for acceptance into the local ledger) and MUST NOT name a consensus path that did not run.

#### Scenario: Checkpoint descends from prior checkpoint
r[molten.evidence.chain_hashing.control_plane_checkpoints.descends]
- GIVEN a prior accepted checkpoint for a chain scope/id/epoch
- WHEN a new checkpoint is proposed
- THEN verification confirms the new head descends from the prior checkpoint head or explicitly records an admitted reconfiguration/epoch change

#### Scenario: Checkpoint naming an unimplemented mode is rejected
- GIVEN a checkpoint that names the `trellis-raft` control-plane mode, under either checkpoint version
- WHEN Molten parses it
- THEN the checkpoint is rejected.
