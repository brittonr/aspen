# Consensus deltas

## ADDED Requirements

### Requirement: Production Raft has non-tautological CI model checking
r[molten.consensus.production_raft_formal_model] Production replicated Raft (`fabric_consistency::raft`) MUST be backed by a CI-executed formal model covering leader election, log replication, conflicting-suffix truncation, compaction, and `ReadIndex` confirmation without assuming Leader Completeness in `Init`. Fast-path models MUST genuinely model `Quorum` intersection and per-replica state rather than centralized guard tautologies.

#### Scenario: Missing election or truncation safety violates model invariant
- GIVEN a production Raft model mutation that permits committing a previous-term entry directly or truncating committed entries
- WHEN TLC or Quint checks the model
- THEN the model checker MUST emit a counterexample trace and fail CI.

#### Scenario: Fast-path model exercises quorum intersection
- GIVEN competing superquorums and message reordering in `ConsensusFastPath.tla`
- WHEN the model runs with an insufficient quorum size
- THEN the safety invariant MUST fail rather than passing vacuously with an unused `Quorum` constant.
