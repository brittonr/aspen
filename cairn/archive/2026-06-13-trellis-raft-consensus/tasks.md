## Phase 1: Consensus scope and artifacts

- [x] [serial] r[molten.consensus.scope] Define the Raft-backed control-plane scope and explicitly exclude normal actor messages, ordinary choreography steps, gossip, blobs, and local-only dataspace assertions.
- [x] [serial] r[molten.consensus.group_manifest] Define a Raft group manifest with group id, static members, state-machine kind, command schemas, read mode, snapshot policy, resource policy, and policy references.
- [x] [serial] r[molten.consensus.command_envelope] Define canonical Molten command envelopes for replicated control-plane log entries with hashes, client session ids, sequence numbers, capabilities, and evidence references.
- [x] [parallel] r[molten.consensus.deterministic_state_machine] Define the pure deterministic state-machine boundary for apply, read, snapshot, and restore operations.
- [x] [parallel] r[molten.consensus.transport_neutral_messages] Define transport-neutral canonical command/control-plane records that can be carried by dataspace, Iroh, or test channels without changing admission semantics.

## Phase 2: Trellis Raft admission layer

- [x] [serial] r[molten.consensus.trellis_raft_core] Wrap Trellis-style predicate receipts for append consistency, commit advancement, reads, snapshots, client sessions, and state-machine safety in the implemented control-registry scope.
- [x] [serial] r[molten.consensus.log_admission] Add log-entry admission checks for term/index ordering, append consistency, commit advancement, and command hash integrity.
- [x] [serial] r[molten.consensus.client_sessions] Add client-session and sequence-number idempotency checks before applying committed commands.
- [x] [parallel] r[molten.consensus.linearizable_reads] Add read-index admission by default and deny lease-read semantics unless future explicit lease policy exists.
- [x] [parallel] r[molten.consensus.membership_changes] Validate static group membership and document dynamic voter/learner/joint-consensus changes as denied/future explicit extensions.
- [x] [parallel] r[molten.consensus.snapshot_integrity] Add snapshot creation, install/parse, restore, and content-integrity checks.

## Phase 3: Policy, receipts, persistence, and observability

- [x] [serial] r[molten.consensus.policy_boundary] Gate group installation, command proposal, membership change, linearizable read, and snapshot operations through explicit policy/authority/resource/Trellis/Cairn evidence before side effects.
- [x] [serial] r[molten.consensus.cairn_receipts] Validate group, command, registry, read, snapshot, recovery, and predicate receipts through canonical parsers before treating them as evidence.
- [x] [parallel] r[molten.consensus.durable_store] Add a durable log, snapshot, client-session, and receipt-index storage boundary without leaking filesystem effects into pure state-machine logic.
- [x] [parallel] r[molten.consensus.observability] Emit/classify tracing evidence for Raft group id, term, index, node/member ids, operation kind, admission decision, commit state, and receipt reference.
- [x] [parallel] r[molten.consensus.recovery] Add recovery logic that restores the latest admitted snapshot and checks committed log entries deterministically.

## Phase 4: Tests and integration

- [x] [serial] r[molten.consensus.integration_tests] Add tests for group manifest validation, command proposal/commit/apply, read-index behavior, duplicate client sequence rejection, static membership bounds, snapshot restore, durable store status, and receipt validation.
- [x] [parallel] r[molten.consensus.property_tests] Add Hegel property tests for generated finite logs, sessions, snapshots, and control-registry commands within supported bounds.
- [x] [parallel] r[molten.consensus.transport_tests] Add transport-neutral tests proving canonical command/control-plane records are interpreted identically through local and dataspace-facing paths.
- [x] [serial] r[molten.consensus.choreography_registry] Demonstrate the first Raft-backed control-plane state machine by replicating installed choreography protocol artifacts or another selected control-plane registry.
