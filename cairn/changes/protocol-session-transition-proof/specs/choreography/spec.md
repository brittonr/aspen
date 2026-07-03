## ADDED Requirements

### Requirement: Protocol endpoint transitions are legal for projected state
r[molten.protocol_state_machine_proof.endpoint_transition_legality] Molten MUST prove that protocol operation receipts for send, receive, branch, and offer operations are accepted only when the operation matches the projected local endpoint state, peer, label, prior state, and next state.

#### Scenario: Wrong branch label denies
- GIVEN a projected endpoint state with a bounded set of legal branch labels
- WHEN a protocol operation receipt uses a label that is not legal for the projected state
- THEN the operation receipt decision is `deny`
- AND diagnostics identify the missing or ambiguous branch transition.

### Requirement: Protocol lifecycle replay is complete
r[molten.protocol_state_machine_proof.lifecycle_replay_completeness] Molten MUST prove that protocol lifecycle gate receipts replay install and operation receipts against canonical endpoint states, message evidence, and terminal state refs before accepting a completed session lifecycle.

#### Scenario: Missing terminal state denies lifecycle gate
- GIVEN protocol operation evidence with a required terminal next-state ref removed
- WHEN Molten evaluates the protocol lifecycle gate
- THEN the gate receipt decision is `deny`
- AND diagnostics identify missing terminal or replay evidence.

### Requirement: Generated protocol session traces preserve projection invariants
r[molten.protocol_state_machine_proof.generated_session_traces] Molten SHOULD include bounded generated or fixture-derived protocol session traces that cover linear send/receive and branch/offer paths while preserving projected endpoint transition invariants.

#### Scenario: Generated branch trace reaches terminal state
- GIVEN a bounded projected protocol with a branch or offer path
- WHEN Molten replays a generated legal session trace
- THEN every operation receipt passes
- AND the lifecycle gate reaches the expected terminal state refs.
