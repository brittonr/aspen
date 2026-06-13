# Consensus Delta: Raft Control-Plane Registry

### Requirement: Raft is scoped to control-plane commands
r[molten.raft_control_plane_registry.spec.control_scope] Raft-backed state machines MUST accept only explicit control-plane command schemas and MUST reject normal actor messages, ordinary choreography steps, gossip, docs, and blob transfer payloads.

#### Scenario: Registry command accepted
- GIVEN a Raft command envelope whose command schema is `install-protocol`
- AND policy/authority/resource checks pass
- WHEN the command is proposed
- THEN it can enter the replicated log
- AND commit/apply receipts bind the command ref

#### Scenario: Actor message rejected
- GIVEN a Raft command envelope containing an actor message payload
- WHEN proposal validation runs
- THEN Molten emits a denial receipt
- AND the payload is not appended to the control-plane log

### Requirement: Registry apply is deterministic and idempotent
r[molten.raft_control_plane_registry.spec.registry_apply] The control registry state machine MUST apply committed commands deterministically and MUST deduplicate client session sequence numbers before state changes.

#### Scenario: Duplicate client sequence is idempotent
- GIVEN a committed registry update for client session `c` sequence `7`
- WHEN the same client sequence is proposed again
- THEN the state machine returns the prior result receipt
- AND no second state delta is applied

### Requirement: Reads and recovery bind evidence
r[molten.raft_control_plane_registry.spec.read_recovery] Control-plane reads and recovery MUST bind read-index, snapshot, log, client-session, and state refs before serving results.

#### Scenario: Read-index read passes
- GIVEN a committed control registry state at term/index
- WHEN a read-index request is admitted
- THEN the read receipt binds the committed index and returned registry refs

#### Scenario: Snapshot restore verifies content
- GIVEN a chunk-backed snapshot and committed log suffix
- WHEN a node recovers
- THEN it verifies snapshot refs and replays the suffix deterministically
- AND emits a recovery receipt before reporting healthy
