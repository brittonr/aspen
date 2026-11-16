# Consensus Delta

## MODIFIED Requirements

### Requirement: Linearizable read admission
r[molten.consensus.linearizable_reads] Molten MUST support linearizable control-plane reads through read-index admission by default and MUST deny lease-read semantics unless explicit future manifest timing assumptions and policy admission are implemented. A leader MUST NOT take a read index until its commit index covers every entry an earlier leader may have committed: the entry at its commit index belongs to its current term, or its commit index reaches the last entry in its log. Until then a linearizable read MUST be reported retryable rather than served.

#### Scenario: Read-index admits current read
- GIVEN a leader or local model state that has satisfied the configured read-index condition
- WHEN a client requests a linearizable read
- THEN the read can be served from state that is at least as recent as the admitted read index.

#### Scenario: Lease read without lease policy is denied
- GIVEN a group manifest that does not admit lease-read timing assumptions
- WHEN a node attempts to serve a read as a lease read
- THEN the runtime denies the lease-read path and requires read-index or another admitted linearizable read path.

#### Scenario: New leader has not committed a current-term entry
- GIVEN a previous leader committed and acknowledged a write that the new leader holds above its commit index
- WHEN a client requests a linearizable read from the new leader before any current-term entry commits
- THEN the leader reports the read retryable and registers no pending read
- AND after a current-term entry commits, the read index covers the acknowledged write.

#### Scenario: New leader already holds only committed entries
- GIVEN a new leader whose commit index reaches the last entry in its log, with no current-term entry committed
- WHEN a client requests a linearizable read
- THEN the leader takes its commit index as the read index, which covers every write an earlier leader acknowledged.
