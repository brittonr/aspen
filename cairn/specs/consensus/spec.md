# Consensus Specification

## Purpose

Defines the `consensus` capability.

## Requirements

### Requirement: Raft-backed control-plane scope
r[molten.consensus.scope] Molten MUST use Raft-backed consensus only for explicitly declared strongly consistent control-plane state and MUST NOT require Raft for normal actor messages, ordinary choreography step traffic, gossip fanout, blob transfer, or local-only dataspace assertions.

#### Scenario: Ordinary actor message bypasses Raft
- GIVEN a normal actor-to-actor envelope that does not mutate a Raft-backed control-plane resource
- WHEN the runtime routes the envelope
- THEN the envelope may use local dataspace or remote transport without creating a Raft log entry.

### Requirement: Raft group manifest
r[molten.consensus.group_manifest] Molten MUST define a Raft group manifest that identifies group id, static members, state-machine kind, command schemas, read mode, snapshot policy, persistence/resource policy, and policy references.

#### Scenario: Manifest declares consensus group
- GIVEN a Raft group manifest for a control-plane registry
- WHEN Molten loads the manifest
- THEN it identifies the group id, initial members, state-machine schema, allowed command kinds, read mode, snapshot policy, resource refs, and policy refs.

### Requirement: Canonical command envelopes
r[molten.consensus.command_envelope] Molten MUST represent replicated control-plane commands as canonical Molten command envelopes with stable hashes, client session ids, sequence numbers, capabilities, resource refs, policy refs, and evidence refs.

#### Scenario: Equivalent commands hash identically
- GIVEN two equivalent command envelopes constructed through different code paths
- WHEN each command is encoded at the canonical boundary
- THEN both command envelopes produce the same command hash and log-entry identity.

### Requirement: Deterministic replicated state machine
r[molten.consensus.deterministic_state_machine] The replicated state-machine boundary MUST be deterministic and MUST NOT perform filesystem, network, process, clock, scripting, runtime scheduling, or adapter side effects during pure apply, read, snapshot, or restore operations.

#### Scenario: Apply returns deterministic state and outputs
- GIVEN the same prior state and committed command envelope
- WHEN two nodes apply the command through the state-machine boundary
- THEN both nodes produce the same next state and declared output records without external side effects during apply.

### Requirement: Transport-neutral Raft messages
r[molten.consensus.transport_neutral_messages] Consensus command and control-plane records MUST be canonical envelopes whose admission semantics are independent of whether a future transport uses local dataspace, Iroh, direct test channels, or another admitted carrier.

#### Scenario: Same command over different transports
- GIVEN the same command envelope delivered through two admitted carriers
- WHEN the receiving consensus boundary validates the envelope content
- THEN both deliveries produce the same admission result for the same local state.

### Requirement: Trellis Raft core admission
r[molten.consensus.trellis_raft_core] Molten MUST use Trellis-style bounded predicate receipts as the normative admission surface for append consistency, quorum/commit advancement, read-index freshness, snapshot restore, client-session idempotency, and state-machine safety in the implemented control-registry scope.

#### Scenario: Append admission uses Trellis-backed predicate
- GIVEN an append request for the control registry
- WHEN Molten evaluates whether the request can update local log state
- THEN the decision is bound to a Trellis-style predicate receipt for term, index, log, command, and quorum conditions.

### Requirement: Log-entry admission
r[molten.consensus.log_admission] Molten MUST admit log updates only when term/index ordering, append consistency, commit advancement, and command hash integrity checks pass.

#### Scenario: Conflicting append is rejected
- GIVEN a follower log or recovery input whose previous term or index does not match a log entry
- WHEN the consensus boundary evaluates the update
- THEN the update is rejected before changing committed state.

#### Scenario: Committed command hash is checked
- GIVEN a proposed log entry carrying a command envelope and declared command hash
- WHEN the consensus layer admits the entry
- THEN the declared hash must match the canonical command envelope hash before the entry can be committed or applied.

### Requirement: Client session idempotency
r[molten.consensus.client_sessions] Molten MUST track client sessions and sequence numbers so committed control-plane command application is idempotent at the replicated state-machine boundary.

#### Scenario: Duplicate client sequence is not applied twice
- GIVEN a committed command with client session `S` and sequence number `7` that has already been applied
- WHEN the same client session and sequence number appears again
- THEN the state machine returns the prior recorded result or rejects the duplicate without applying the command a second time.

### Requirement: Linearizable read admission
r[molten.consensus.linearizable_reads] Molten MUST support linearizable control-plane reads through read-index admission by default and MUST deny lease-read semantics unless explicit future manifest timing assumptions and policy admission are implemented.

#### Scenario: Read-index admits current read
- GIVEN a leader or local model state that has satisfied the configured read-index condition
- WHEN a client requests a linearizable read
- THEN the read can be served from state that is at least as recent as the admitted read index.

#### Scenario: Lease read without lease policy is denied
- GIVEN a group manifest that does not admit lease-read timing assumptions
- WHEN a node attempts to serve a read as a lease read
- THEN the runtime denies the lease-read path and requires read-index or another admitted linearizable read path.

### Requirement: Membership change admission
r[molten.consensus.membership_changes] Molten MUST validate static group membership in the current manifest and MUST treat dynamic voter, learner, joint-consensus, and leadership-transfer changes as denied or future explicit extensions unless represented by an admitted replicated command and policy evidence.

#### Scenario: Unauthorized member add is rejected
- GIVEN a request to add a new voter to a Raft group without an implemented admitted membership-change command path
- WHEN the request reaches the consensus boundary
- THEN the membership change is rejected before it is committed as group configuration.

### Requirement: Snapshot integrity and recovery
r[molten.consensus.snapshot_integrity] Molten MUST bind snapshots to group id, last included term/index, state-machine schema/state, canonical content hash, log refs, session refs, and receipt evidence, and MUST verify those bindings before install or restore.

#### Scenario: Tampered snapshot is rejected
- GIVEN a snapshot artifact whose bytes or content reference do not match the declared snapshot hash
- WHEN a node attempts to parse or install the snapshot
- THEN the snapshot is rejected before replacing local state.

#### Scenario: Recovery replays after admitted snapshot
- GIVEN an admitted snapshot at index `N` and durable committed log entries after `N`
- WHEN a node recovers
- THEN it restores the snapshot and deterministically checks the later committed entries to reconstruct state.

### Requirement: Consensus policy boundary
r[molten.consensus.policy_boundary] Molten MUST gate group installation, command proposal, membership change, linearizable read, and snapshot operations through explicit policy, authority, resource, Trellis-style predicate, or Cairn receipt boundaries before side effects.

#### Scenario: Command proposal without capability is denied
- GIVEN a client command that would mutate Raft-backed control-plane state
- WHEN the command lacks required authority or resource evidence
- THEN the runtime denies the proposal before appending it to the log.

### Requirement: Cairn consensus receipts
r[molten.consensus.cairn_receipts] Molten MUST validate group, command, registry, read, snapshot, recovery, and predicate receipts through canonical parsers before treating those receipts as evidence for later admission or inspection.

#### Scenario: Invalid consensus receipt is excluded
- GIVEN a committed command envelope that references a malformed admission receipt
- WHEN the consensus layer evaluates command evidence
- THEN the malformed receipt is excluded and cannot satisfy policy or audit requirements.

### Requirement: Durable consensus store boundary
r[molten.consensus.durable_store] Molten MUST define a durable storage boundary for logs, snapshots, client-session records, and receipt indexes while keeping filesystem effects outside pure state-machine logic.

#### Scenario: Store persists committed log and session result
- GIVEN a committed command applied to the state machine
- WHEN the store adapter persists consensus metadata
- THEN it records log entries, snapshots, client-session results, and receipt refs without changing pure apply semantics.

### Requirement: Consensus observability
r[molten.consensus.observability] Molten MUST emit or classify structured evidence for consensus operations including group id/ref, term, index, node/member refs, message or command kind, admission decision, commit state, and receipt reference.

#### Scenario: Commit emits trace event
- GIVEN a log entry that becomes committed
- WHEN observability evidence is recorded
- THEN the record identifies group id/ref, term, index, member refs, admission decision, and receipt reference.

### Requirement: Consensus recovery
r[molten.consensus.recovery] Molten MUST recover Raft-backed state by restoring the latest admitted snapshot, checking durable committed log entries, restoring client-session idempotency records, and validating receipt indexes.

#### Scenario: Restart reconstructs state
- GIVEN a node with a durable snapshot, log suffix, client-session table, and receipt index
- WHEN the node restarts
- THEN it reconstructs the same committed state-machine value and does not reapply already recorded client-session commands.

### Requirement: Consensus integration tests
r[molten.consensus.integration_tests] Molten MUST include tests for group manifest validation, command proposal/commit/apply, read-index behavior, duplicate client sequence rejection, static membership bounds, snapshot restore, durable store status, and receipt validation.

#### Scenario: Control-plane command commits
- GIVEN a Raft-backed control-registry fixture with admitted policy
- WHEN a client proposes a valid registry command
- THEN the command commits, applies once, emits receipt evidence, and becomes visible through an admitted read.

### Requirement: Consensus property tests
r[molten.consensus.property_tests] Molten MUST use bounded Hegel property tests for generated finite logs, sessions, snapshots, and control-registry commands within supported bounds.

#### Scenario: Generated duplicate sessions remain idempotent
- GIVEN a generated sequence of client-session commands with possible duplicate sequence numbers
- WHEN the model applies admitted committed commands
- THEN each unique client session sequence mutates state at most once.

### Requirement: Consensus transport tests
r[molten.consensus.transport_tests] Molten MUST test that canonical consensus command envelopes are interpreted identically when passed through the local test boundary and dataspace/control-plane integration layers.

#### Scenario: Control command has identical interpretation
- GIVEN the same command envelope and the same local control-registry state
- WHEN the request is delivered through a local test path and through a coordination/dataspace-facing path
- THEN both paths produce the same admission decision.

### Requirement: First Raft-backed choreography registry
r[molten.consensus.choreography_registry] Molten SHOULD demonstrate the first Raft-backed control-plane state machine by replicating installed choreography protocol artifacts or another explicitly selected control-plane registry.

#### Scenario: Choreography protocol artifact becomes replicated control-plane state
- GIVEN an admitted protocol artifact or another selected control-plane record
- WHEN the registry install command commits
- THEN nodes that apply the committed command expose the same artifact hash, registry namespace/name, and receipt reference.

### Requirement: Control-registry command logs are deterministic
r[molten.consensus_state_machine_proof.registry_log_determinism] Molten MUST prove that independent control-registry runtimes applying the same admitted command log produce identical state refs, registry receipt refs, log entry refs, and commit receipt refs after each committed command.

#### Scenario: Matching logs converge
- GIVEN two independent control-registry runtimes with the same manifest
- WHEN both apply the same bounded admitted command log
- THEN their registry state refs match after every committed command
- AND their emitted registry and commit receipt refs match.

### Requirement: Duplicate client sequences do not apply twice
r[molten.consensus_state_machine_proof.duplicate_client_sequence] Molten MUST prove that duplicate client-session and sequence-number commands return prior result evidence or deny without applying the state-machine mutation a second time.

#### Scenario: Duplicate command preserves state
- GIVEN a committed control-registry command for a client session and sequence number
- WHEN the same client session and sequence number is submitted again
- THEN Molten returns prior result evidence or a denial receipt
- AND the registry state ref does not advance a second time.

### Requirement: Control-registry snapshots restore equivalent state
r[molten.consensus_state_machine_proof.snapshot_restore_equivalence] Molten MUST prove that control-registry snapshots and restore receipts preserve canonical registry state refs and fail closed for missing, stale, or tampered snapshot evidence.

#### Scenario: Snapshot restore preserves registry ref
- GIVEN a control-registry runtime with committed commands and a canonical snapshot
- WHEN Molten restores a fresh runtime from the snapshot
- THEN the restored registry state ref equals the snapshotted state ref
- AND restore evidence binds the snapshot ref and checks.

### Requirement: Generic capability tokens do not replace membership admission
r[molten.capability_token.no_generic_membership] Molten MUST NOT allow generic capability tokens or proofsets to replace Raft membership preflight, quorum-safety predicate receipts, read-index evidence, or membership commit receipts.

#### Scenario: Membership token supports request only
- GIVEN a peer presents a capability token permitting it to request Raft membership preflight
- WHEN membership admission evaluates the peer
- THEN the token can satisfy only the request authority input
- AND separate membership preflight, quorum-safety, and commit evidence remain required before membership changes.

### Requirement: Subscriber peers are not Raft members
r[molten.peer_subscriber.raft_boundary] Molten MUST NOT treat subscriber, observer, or read-only peer roles as Raft voters, non-voters, learners, or linearizable read replicas without separate membership admission and read-index/read-capability evidence.

#### Scenario: Subscriber cannot serve linearizable read by role alone
- GIVEN a peer has a read-only subscription grant for control-plane status summaries
- WHEN a client asks that peer to serve a linearizable control-plane read
- THEN Molten denies unless separate read-index and read-capability evidence is present
- AND the subscriber role is not recorded as Raft membership.

### Requirement: Raft membership changes have canonical records
r[molten.raft_membership_admission.model] Molten MUST define canonical Raft membership-change request, preflight receipt, and commit receipt records that bind group id, target peer/node, requested role, prior configuration ref, proposed configuration ref, authority refs, policy refs, resource refs, peer/session refs, source-gate/provenance refs, readiness refs, and diagnostics.

#### Scenario: Preflight receipt binds target and config
- GIVEN an operator requests adding a node to a Raft-backed control-plane group
- WHEN Molten emits a membership preflight receipt
- THEN the receipt names the group, target node, requested role, prior config ref, proposed config ref, evidence refs, decision, and diagnostics
- AND the receipt ref is derived from canonical Preserves bytes.

### Requirement: Raft membership is stronger than peer connectivity
r[molten.raft_membership_admission.stronger_than_peer] Molten MUST NOT admit Raft/control-plane membership from connected peer state, transport observations, gossip topic joins, docs namespace joins, protocol sessions, or job-pool joins alone.

#### Scenario: Connected peer cannot become voter implicitly
- GIVEN a peer session is connected and admitted for a gossip topic
- WHEN a Raft group membership check evaluates the peer
- THEN membership admission denies without a dedicated membership-change request and preflight receipt
- AND no Raft configuration entry is appended.

### Requirement: Membership preflight checks control-plane readiness
r[molten.raft_membership_admission.preflight_checks] Molten MUST require membership preflight to validate peer session scope, membership authority, policy, resources, source-gate/provenance, state-machine/schema compatibility, transport support, replay support, snapshot/log catch-up readiness, and operator evidence before mutation.

#### Scenario: Missing source-gate denies membership
- GIVEN a peer has valid transport and peer-session evidence but lacks current source-gate or provenance evidence for the control-plane artifact set
- WHEN membership preflight runs
- THEN the preflight decision is deny
- AND diagnostics identify source-gate or provenance evidence as missing.

### Requirement: Membership transitions preserve quorum safety
r[molten.raft_membership_admission.quorum_safety] Molten MUST bind Trellis/Raft predicate receipts for quorum preservation and configuration transition safety before any Raft membership change can commit.

#### Scenario: Unsafe removal denies before commit
- GIVEN a proposed membership change would remove voters without preserving quorum under the configured transition rule
- WHEN membership preflight or commit validation runs
- THEN the decision is deny
- AND no membership commit receipt claims the unsafe configuration.

### Requirement: Membership diagnostics distinguish evidence classes
r[molten.raft_membership_admission.diagnostics] Molten SHOULD diagnose peer connectivity, membership preflight, committed membership state, readiness evidence, and linearizable read evidence as separate state classes.

#### Scenario: Status shows connected but not member
- GIVEN a peer is connected but has no passing membership preflight or commit receipt
- WHEN membership diagnostics render status
- THEN they report the peer as connected but not a Raft member
- AND they name the missing membership evidence.

### Requirement: Membership CLI starts with dry-run preflight
r[molten.raft_membership_admission.cli_preflight] Molten SHOULD provide an operator dry-run membership preflight and readback summary before enabling or executing mutating membership changes.

#### Scenario: Dry-run does not mutate group
- GIVEN an operator runs membership preflight for a candidate node
- WHEN the command completes
- THEN it emits a preflight receipt and readback summary
- AND it does not append a Raft configuration entry or change group membership.

### Requirement: Membership tests cover positive and negative paths
r[molten.raft_membership_admission.positive_negative_tests] Molten SHOULD include positive membership preflight fixtures and negative tests for connected-peer-only, missing authority, missing source-gate, incompatible state-machine, stale snapshot, revoked peer, and quorum-safety denial.

#### Scenario: Connected-peer-only fixture denies
- GIVEN a fixture contains a connected peer session and live transport evidence but no membership-change request
- WHEN membership admission validates the fixture
- THEN admission denies
- AND diagnostics state that peer connectivity is not Raft membership.

### Requirement: Generic peer promotion cannot grant Raft membership
r[molten.peer_promotion.raft_boundary] Molten MUST NOT allow generic peer capability promotion to grant Raft voter, non-voter, learner, control-plane membership, or linearizable-read roles without the separate Raft membership admission and read-index/read-capability gates.

#### Scenario: Promotion to learner denies outside membership path
- GIVEN a peer promotion grant requests promotion to a Raft learner role
- WHEN generic peer promotion validates the request
- THEN validation denies the generic promotion path
- AND diagnostics direct the operator to the Raft membership admission preflight.

### Requirement: Consensus manifests declare algorithm profiles
r[molten.consensus.algorithm_profile_manifest] Molten MUST represent each control-plane consensus group with an explicit algorithm profile, admitted profile version, read-consistency support, quorum rule, membership policy refs, placement refs, fault-model caveats, and required evidence refs. The current Raft-backed profile MUST remain the default admitted production profile until another profile has separate accepted implementation, proof, simulation, and policy evidence.

#### Scenario: Existing Raft group remains default
- GIVEN a control-plane group manifest that targets the currently admitted production behavior
- WHEN Molten loads the manifest
- THEN the manifest resolves to the Raft-backed profile
- AND the group still requires the existing Raft membership, read-index, commit, snapshot, and receipt evidence.

#### Scenario: Unknown algorithm profile denies
- GIVEN a group manifest names an unknown, unsupported, or misspelled algorithm profile
- WHEN group installation or readback validates the manifest
- THEN Molten denies the group before installation
- AND diagnostics identify the unsupported profile without falling back to another consensus algorithm.

### Requirement: Leaderless quorum profile is experimental and gated
r[molten.consensus.leaderless_profile_boundary] Molten MAY define a Meerkat/QuePaxa-inspired leaderless quorum profile only as an explicit experimental control-plane profile. The profile MUST deny production admission unless policy, proof/model evidence, deterministic simulation evidence, placement evidence, membership evidence, and operator caveat receipts all pass.

#### Scenario: Experimental profile requires evidence
- GIVEN a manifest requests the experimental leaderless quorum profile
- WHEN required proof, simulation, placement, membership, or policy evidence is missing
- THEN Molten denies production admission
- AND the profile can be inspected only as experimental or diagnostic evidence.

#### Scenario: Any-replica proposal is profile-scoped
- GIVEN the experimental profile is admitted for a deterministic simulation run
- WHEN a majority-connected non-leader replica proposes a control-plane command
- THEN the command can progress only through the profile's quorum rule
- AND the emitted commit receipt names the proposing replica, quorum evidence, and experimental caveat.

### Requirement: Consensus reads declare consistency mode
r[molten.consensus.read_consistency_modes] Molten MUST classify every control-plane read request and read receipt as `linearizable` or `local-stale`. Linearizable reads MUST bind read-index evidence or admitted algorithm-specific quorum evidence. Local-stale reads MAY expose local diagnostics but MUST NOT satisfy mutation guards, release gates, policy currentness checks, membership admission, or production pass evidence.

#### Scenario: Linearizable read has quorum-backed freshness
- GIVEN a client requests a linearizable control-plane read
- WHEN Molten serves the read
- THEN the receipt binds read-index or algorithm-specific quorum freshness evidence
- AND the returned state ref is at least as fresh as the admitted read evidence.

#### Scenario: Local-stale read is visibly non-authoritative
- GIVEN a client requests a local-stale control-plane read from a replica that may lag
- WHEN Molten serves the read
- THEN the receipt marks the response as local-stale
- AND downstream gates reject that receipt wherever linearizable currentness is required.

### Requirement: Consensus placement evidence is explicit
r[molten.consensus.replica_placement_evidence] Molten SHOULD emit canonical placement reports for consensus groups that bind group id, candidate members, admitted members, fault-domain policy, membership refs, placement policy refs, expected majority-reachability assumptions, latency diagnostics, denied candidates, and refresh requirements.

#### Scenario: Placement report binds selected members
- GIVEN an operator plans a control-plane consensus group
- WHEN placement evaluation runs
- THEN the placement report names the selected members, policy refs, membership evidence, fault-domain diagnostics, and majority-reachability assumptions
- AND the report ref is available to the group manifest or install receipt.

#### Scenario: Unsafe placement denies installation
- GIVEN a candidate placement concentrates members in a disallowed fault domain or lacks required placement evidence
- WHEN group installation validates the placement refs
- THEN Molten denies installation before consensus state is created
- AND diagnostics identify the missing or unsafe placement condition.

### Requirement: Consensus applications use deterministic canonical logs
r[molten.consensus.canonical_log_applications] Molten MUST model control-plane applications as deterministic state machines over canonical command/log envelopes. Application code MUST NOT infer consensus semantics from transport, leader identity, replica locality, or local cache state outside the admitted log/read evidence.

#### Scenario: Same log gives same application state
- GIVEN two replicas have the same admitted command log and application manifest
- WHEN each replica applies the log through the pure state-machine boundary
- THEN they produce the same state ref, application receipt refs, and status assertion refs.

#### Scenario: Local cache cannot bypass log evidence
- GIVEN a replica has local cached control-plane state but no admitted commit or read evidence for a requested current value
- WHEN a mutation guard or release gate evaluates the cache result
- THEN the gate denies the cache result as insufficient evidence
- AND requires the relevant log, commit, or linearizable read receipt.

### Requirement: Consensus non-claim boundaries are recorded
r[molten.consensus.non_claim_boundaries] Molten MUST record explicit non-claim boundaries for consensus groups: no Byzantine tolerance, no general-purpose database guarantee, no ordinary actor-message ordering, no global dataspace, no blob/gossip/job transport ordering, no lease-read semantics without accepted timing assumptions, and no production leaderless profile without accepted evidence.

#### Scenario: Unsupported use is diagnostic-only
- GIVEN a report tries to use consensus evidence as proof of Byzantine tolerance, general database correctness, ordinary actor ordering, global dataspace consistency, or transport delivery correctness
- WHEN evidence gates evaluate the report
- THEN the report cannot satisfy those claims
- AND diagnostics name the consensus non-claim boundary that applies.

#### Scenario: Lease read remains denied without timing policy
- GIVEN a group manifest lacks accepted lease timing assumptions and policy evidence
- WHEN a replica attempts to serve a control-plane read as a lease read
- THEN Molten denies the lease-read path
- AND requires linearizable read evidence or a visibly local-stale receipt.
