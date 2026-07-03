## ADDED Requirements

### Requirement: Lifecycle graph reachability matches the specified state model
r[molten.lifecycle_state_machine_proof.reachability] Molten MUST prove that lifecycle states reachable from `declared` are reachable only through the specified lifecycle transition relation, and forbidden shortcuts MUST deny before producing passing lifecycle evidence.

#### Scenario: Startup path is reachable without shortcuts
- GIVEN a lifecycle entity in the `declared` state
- WHEN the lifecycle proof computes reachable states from the allowed transition relation
- THEN `spawning`, `starting`, and `ready` are reachable through their required intermediate states
- AND a direct `declared` to `ready` transition denies.

### Requirement: Lifecycle terminal and cleanup boundaries are closed
r[molten.lifecycle_state_machine_proof.terminal_cleanup] Molten MUST prove terminal and cleanup boundaries in the lifecycle graph: `cleaned` has no outgoing passing transition, `stopped` can only clean up, `failed` can only restart or clean up, and `restarting` can only return to starting or clean up.

#### Scenario: Cleaned state cannot exit
- GIVEN a lifecycle entity already in the `cleaned` state
- WHEN any lifecycle transition is evaluated from `cleaned`
- THEN the transition receipt decision is `deny`
- AND no outgoing lifecycle edge is accepted.
