## ADDED Requirements

### Requirement: Raft-backed control-plane scope
r[molten.consensus.scope] The system MUST use Raft-backed consensus only for explicitly declared strongly consistent control-plane state and MUST NOT require Raft for normal actor messages, ordinary choreography steps, gossip traffic, blob transfer, or local-only dataspace assertions.

#### Scenario: Ordinary actor message bypasses Raft
r[molten.consensus.scope.actor_message]
- GIVEN a normal actor-to-actor envelope that does not mutate a Raft-backed control-plane resource
- WHEN the runtime routes the envelope
- THEN the envelope may use local dataspace or remote transport without creating a Raft log entry

#### Scenario: Protocol registry mutation uses Raft
r[molten.consensus.scope.protocol_registry]
- GIVEN a request to install or update a replicated protocol artifact registry
- WHEN the registry is configured as Raft-backed control-plane state
- THEN the mutation is represented as a consensus command proposal before becoming visible as committed state

### Requirement: Raft group manifest
r[molten.consensus.group_manifest] The system MUST define a Raft group manifest that identifies group id, members, state-machine kind, command schemas, read mode, timeouts, snapshot policy, persistence policy, and policy references.

#### Scenario: Manifest declares consensus group
r[molten.consensus.group_manifest.declare]
- GIVEN a Raft group manifest for a control-plane registry
- WHEN Molten loads the manifest
- THEN it can identify the group id, initial voters or learners, state-machine schema, allowed command kinds, read mode, timeout bounds, snapshot policy, persistence requirements, and policy references

### Requirement: Canonical command envelopes
r[molten.consensus.command_envelope] The system MUST represent replicated log commands as canonical Molten command envelopes with stable hashes, client ids, client session ids, sequence numbers, capabilities, and evidence references.

#### Scenario: Equivalent commands hash identically
r[molten.consensus.command_envelope.stable_hash]
- GIVEN two equivalent command envelopes constructed through different Rust code paths
- WHEN each command is encoded at the canonical boundary
- THEN both command envelopes produce the same command hash and log-entry identity

### Requirement: Deterministic replicated state machine
r[molten.consensus.deterministic_state_machine] The replicated state-machine boundary MUST be deterministic and MUST NOT perform filesystem, network, process, clock, scripting, runtime scheduling, or adapter side effects during pure apply, read, snapshot, or restore operations.

#### Scenario: Apply returns deterministic state and outputs
r[molten.consensus.deterministic_state_machine.apply]
- GIVEN the same prior state and committed command envelope
- WHEN two nodes apply the command through the state-machine boundary
- THEN both nodes produce the same next state and declared output records without performing external side effects during apply

### Requirement: Transport-neutral Raft messages
r[molten.consensus.transport_neutral_messages] Raft protocol messages MUST be represented as Molten envelopes whose consensus semantics are independent of whether they are carried by local dataspace, Iroh, direct test channels, or another admitted transport.

#### Scenario: Same message over different transports
r[molten.consensus.transport_neutral_messages.same_semantics]
- GIVEN the same append-entries message envelope delivered over a test channel and over the dataspace adapter
- WHEN the receiving consensus node validates the message content
- THEN both deliveries produce the same admission result for the same local Raft state

### Requirement: Trellis Raft core admission
r[molten.consensus.trellis_raft_core] The system MUST use Trellis Raft primitives as the normative bounded admission/specification layer for election, quorum, log matching, append entries, commit advancement, reads, membership, snapshots, client sessions, and state-machine safety surfaces.

#### Scenario: Append admission uses Trellis-backed predicate
r[molten.consensus.trellis_raft_core.append]
- GIVEN an append-entries request for a Raft group
- WHEN Molten evaluates whether the request can update local log state
- THEN the decision is made through the Trellis-backed Raft admission surface for the relevant term, index, log, and quorum conditions

### Requirement: Log-entry admission
r[molten.consensus.log_admission] The system MUST admit log updates only when term/index ordering, append consistency, commit advancement, and command hash integrity checks pass.

#### Scenario: Conflicting append is rejected
r[molten.consensus.log_admission.conflict]
- GIVEN a follower log whose previous term or index does not match an append-entries request
- WHEN the follower evaluates the append request
- THEN the follower rejects the update before changing committed state

#### Scenario: Committed command hash is checked
r[molten.consensus.log_admission.hash]
- GIVEN a proposed log entry carrying a command envelope and declared command hash
- WHEN the consensus layer admits the entry
- THEN the declared hash must match the canonical command envelope hash before the entry can be committed or applied

### Requirement: Client session idempotency
r[molten.consensus.client_sessions] The system MUST track client sessions and sequence numbers so committed command application is idempotent at the replicated state-machine boundary.

#### Scenario: Duplicate client sequence is not applied twice
r[molten.consensus.client_sessions.duplicate]
- GIVEN a committed command with client session `S` and sequence number `7` that has already been applied
- WHEN the same client session and sequence number appears again
- THEN the state machine returns the prior recorded result or rejects the duplicate without applying the command a second time

### Requirement: Linearizable read admission
r[molten.consensus.linearizable_reads] The system MUST support linearizable reads through read-index admission by default and MUST allow lease reads only when explicit manifest timing assumptions and policy admission are present.

#### Scenario: Read-index admits current read
r[molten.consensus.linearizable_reads.read_index]
- GIVEN a leader that has satisfied the configured read-index quorum condition
- WHEN a client requests a linearizable read
- THEN the read can be served from state that is at least as recent as the admitted read index

#### Scenario: Lease read without lease policy is denied
r[molten.consensus.linearizable_reads.lease_denied]
- GIVEN a group manifest that does not admit lease-read timing assumptions
- WHEN a node attempts to serve a read as a lease read
- THEN the runtime denies the lease-read path and requires read-index or another admitted linearizable read path

### Requirement: Membership change admission
r[molten.consensus.membership_changes] The system MUST admit membership changes only through replicated commands that satisfy bounded membership, quorum, learner-promotion, and joint-consensus rules where applicable.

#### Scenario: Unauthorized member add is rejected
r[molten.consensus.membership_changes.unauthorized]
- GIVEN a request to add a new voter to a Raft group
- WHEN the requester lacks the required capability or the proposed configuration violates membership bounds
- THEN the membership change is rejected before it is committed as group configuration

#### Scenario: Joint consensus guards voter transition
r[molten.consensus.membership_changes.joint]
- GIVEN a voter-set transition that requires joint consensus
- WHEN Molten evaluates the transition
- THEN the new configuration is admitted only through the configured joint-consensus path and quorum predicates

### Requirement: Snapshot integrity and recovery
r[molten.consensus.snapshot_integrity] The system MUST bind snapshots to group id, last included term/index, membership state, state-machine schema, canonical content hash, optional content references, and receipt evidence, and MUST verify those bindings before install or restore.

#### Scenario: Tampered snapshot is rejected
r[molten.consensus.snapshot_integrity.tampered]
- GIVEN a snapshot artifact whose bytes or content reference do not match the declared snapshot hash
- WHEN a node attempts to install the snapshot
- THEN the snapshot is rejected before replacing local state

#### Scenario: Recovery replays after admitted snapshot
r[molten.consensus.snapshot_integrity.replay]
- GIVEN an admitted snapshot at index `N` and durable committed log entries after `N`
- WHEN a node recovers
- THEN it restores the snapshot and deterministically replays the later committed entries to reconstruct state

### Requirement: Consensus policy boundary
r[molten.consensus.policy_boundary] The system MUST gate Raft group installation, command proposal, membership change, linearizable read, and snapshot operations through Basalt, Nickel, Steel, Trellis, or Cairn policy gates as applicable before side effects occur.

#### Scenario: Command proposal without capability is denied
r[molten.consensus.policy_boundary.denied_command]
- GIVEN a client command that would mutate Raft-backed control-plane state
- WHEN the command lacks the required capability or contract admission
- THEN the runtime denies the proposal before appending it to the replicated log

### Requirement: Cairn consensus receipts
r[molten.consensus.cairn_receipts] The system MUST validate Raft group, command, membership, read, and snapshot receipts through Cairn before treating those receipts as evidence for later admission or inspection.

#### Scenario: Invalid consensus receipt is excluded
r[molten.consensus.cairn_receipts.invalid]
- GIVEN a committed command envelope that references a malformed admission receipt
- WHEN the consensus layer evaluates command evidence
- THEN the malformed receipt is excluded and cannot satisfy policy or audit requirements

### Requirement: Durable consensus store boundary
r[molten.consensus.durable_store] The system MUST define a durable storage boundary for Raft logs, snapshots, client-session records, and receipt indexes while keeping filesystem effects outside the pure state-machine logic.

#### Scenario: Store persists committed log and session result
r[molten.consensus.durable_store.persist]
- GIVEN a committed command applied to the state machine
- WHEN the store adapter persists consensus metadata
- THEN it records the log entry, client-session sequence result, and receipt reference without changing the pure apply semantics

### Requirement: Consensus observability
r[molten.consensus.observability] The system MUST emit structured tracing events for consensus operations including Raft group id, term, index, node id, message kind, admission decision, commit state, and receipt reference.

#### Scenario: Commit emits trace event
r[molten.consensus.observability.commit]
- GIVEN a log entry that becomes committed
- WHEN observability is enabled
- THEN the runtime emits a structured event identifying group id, term, index, leader or node id, admission decision, and receipt reference

### Requirement: Consensus recovery
r[molten.consensus.recovery] The system MUST recover Raft-backed state by restoring the latest admitted snapshot, replaying durable committed log entries, restoring client-session idempotency records, and validating receipt indexes.

#### Scenario: Restart reconstructs state
r[molten.consensus.recovery.restart]
- GIVEN a node with a durable snapshot, log suffix, client-session table, and receipt index
- WHEN the node restarts
- THEN it reconstructs the same committed state-machine value and does not reapply already recorded client-session commands

### Requirement: Consensus integration tests
r[molten.consensus.integration_tests] The system MUST include tests for group manifest validation, command proposal/commit/apply, read-index behavior, duplicate client sequence rejection, membership changes, snapshot restore, and receipt validation.

#### Scenario: Three-node control-plane command commits
r[molten.consensus.integration_tests.three_node_commit]
- GIVEN a three-node Raft-backed control-plane registry with admitted policy
- WHEN a client proposes a valid registry command and a quorum admits the log entry
- THEN the command commits, applies once, emits receipt evidence, and becomes visible through an admitted linearizable read

### Requirement: Consensus property tests
r[molten.consensus.property_tests] The system MUST use Hegel property-based tests for generated finite logs, sessions, snapshots, and membership changes within supported bounds.

#### Scenario: Generated duplicate sessions remain idempotent
r[molten.consensus.property_tests.generated_sessions]
- GIVEN a generated sequence of client-session commands with possible duplicate sequence numbers
- WHEN the model applies admitted committed commands
- THEN each unique client session sequence mutates state at most once

### Requirement: Consensus transport tests
r[molten.consensus.transport_tests] The system MUST test that transport-neutral Raft message envelopes are interpreted identically over local test channels and the dataspace adapter.

#### Scenario: Vote request has identical interpretation
r[molten.consensus.transport_tests.vote_request]
- GIVEN the same vote-request envelope and the same local Raft state
- WHEN the request is delivered through a local test channel and through the dataspace adapter
- THEN both paths produce the same vote admission decision

### Requirement: First Raft-backed choreography registry
r[molten.consensus.choreography_registry] The system SHOULD demonstrate the first Raft-backed control-plane state machine by replicating installed choreography protocol artifacts or another explicitly selected control-plane registry.

#### Scenario: Choreography protocol artifact becomes replicated control-plane state
r[molten.consensus.choreography_registry.install]
- GIVEN an admitted Trellis-backed choreography protocol artifact
- WHEN the artifact registry is configured as Raft-backed and the install command commits
- THEN all nodes that apply the committed command expose the same protocol artifact hash, role map, label map, payload registry, and installation receipt reference
