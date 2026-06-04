## Phase 1: Consensus scope and artifacts

- [ ] [serial] r[molten.consensus.scope] Define the Raft-backed control-plane scope and explicitly exclude normal actor messages, ordinary choreography steps, gossip, blobs, and local-only dataspace assertions.
- [ ] [serial] r[molten.consensus.group_manifest] Define a Raft group manifest with group id, members, state-machine kind, command schemas, read mode, timeouts, snapshot policy, persistence policy, and policy references.
- [ ] [serial] r[molten.consensus.command_envelope] Define canonical Molten command envelopes for replicated log entries with hashes, client session ids, sequence numbers, capabilities, and evidence references.
- [ ] [parallel] r[molten.consensus.deterministic_state_machine] Define the pure deterministic state-machine boundary for apply, read, snapshot, and restore operations.
- [ ] [parallel] r[molten.consensus.transport_neutral_messages] Define transport-neutral Raft message envelopes that can be carried by dataspace, Iroh, or test channels without changing semantics.

## Phase 2: Trellis Raft admission layer

- [ ] [serial] r[molten.consensus.trellis_raft_core] Wrap Trellis Raft primitives for election, quorum, log matching, append entries, commit advancement, reads, membership, snapshots, client sessions, and state-machine safety.
- [ ] [serial] r[molten.consensus.log_admission] Add log-entry admission checks for term/index ordering, append consistency, commit advancement, and command hash integrity.
- [ ] [serial] r[molten.consensus.client_sessions] Add client-session and sequence-number idempotency checks before applying committed commands.
- [ ] [parallel] r[molten.consensus.linearizable_reads] Add read-index admission by default and lease-read admission only behind explicit manifest and policy conditions.
- [ ] [parallel] r[molten.consensus.membership_changes] Add membership-change admission using bounded membership, joint-consensus, learner-promotion, and quorum predicates where applicable.
- [ ] [parallel] r[molten.consensus.snapshot_integrity] Add snapshot creation, install, restore, chunking, and content-integrity admission checks.

## Phase 3: Policy, receipts, persistence, and observability

- [ ] [serial] r[molten.consensus.policy_boundary] Gate group installation, command proposal, membership change, linearizable read, and snapshot operations through Basalt/Nickel/Steel/Trellis policy before side effects.
- [ ] [serial] r[molten.consensus.cairn_receipts] Validate group, command, membership, read, and snapshot receipts through Cairn before treating them as evidence.
- [ ] [parallel] r[molten.consensus.durable_store] Add a durable log, snapshot, client-session, and receipt-index storage boundary without leaking filesystem effects into pure state-machine logic.
- [ ] [parallel] r[molten.consensus.observability] Emit tracing events for Raft group id, term, index, node id, message kind, admission decision, commit state, and receipt reference.
- [ ] [parallel] r[molten.consensus.recovery] Add recovery logic that restores the latest admitted snapshot and replays committed log entries deterministically.

## Phase 4: Tests and integration

- [ ] [serial] r[molten.consensus.integration_tests] Add tests for group manifest validation, command proposal/commit/apply, read-index behavior, duplicate client sequence rejection, membership changes, snapshot restore, and receipt validation.
- [ ] [parallel] r[molten.consensus.property_tests] Add Hegel property tests for generated finite logs, sessions, snapshots, and membership changes within supported bounds.
- [ ] [parallel] r[molten.consensus.transport_tests] Add transport-neutral tests proving the same Raft message envelope is interpreted identically over local test channels and the dataspace adapter.
- [ ] [serial] r[molten.consensus.choreography_registry] Demonstrate the first Raft-backed control-plane state machine by replicating installed choreography protocol artifacts or another explicitly selected control-plane registry.
