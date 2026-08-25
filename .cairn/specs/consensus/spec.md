# Consensus Specification

## Purpose

Defines the `consensus` capability.

## Requirements

### Requirement: Raft-backed control-plane scope
r[molten.consensus.scope] Molten MUST use consensus only for explicitly declared strongly consistent state owned by an admitted control-plane application or system extension. Each use MUST bind a consistency-group manifest, application state-machine manifest, engine profile, membership/config refs, placement refs, authority, resources, and non-claims. Molten MUST NOT require consensus for normal actor messages, ordinary choreography step traffic, gossip fanout, blob transfer, local-only dataspace assertions, or extension state that selected another admitted consistency mechanism.

#### Scenario: Ordinary actor message bypasses consensus
- GIVEN a normal actor-to-actor envelope that does not mutate an explicitly declared consistency group
- WHEN the runtime routes the envelope
- THEN the envelope may use local dataspace or remote transport without creating a consensus log entry.

#### Scenario: System extension owns a scoped group
- GIVEN an admitted system extension declares a consistency group and deterministic application state machine
- WHEN the group is activated through a compatible consistency port
- THEN the engine may order canonical commands for that group
- AND the extension's application semantics remain outside the engine and node core.

#### Scenario: Undeclared extension state bypasses consensus authority
- GIVEN extension code has not attached state to an admitted consistency group
- WHEN it mutates local or otherwise admitted extension state
- THEN consensus evidence is neither required nor fabricated for that mutation.

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
r[molten.consensus.algorithm_profile_manifest] Molten MUST represent each consistency group with an explicit algorithm profile, admitted profile version, implementation profile, read-consistency support, quorum rule, membership policy refs, placement refs, failure-model caveats, environment scope, and required evidence refs. A Raft algorithm profile MAY be the default only when its selected implementation profile has accepted live distributed-service, transport, durability, recovery, membership, placement, simulation, policy, and operator evidence for the requested environment. An in-process or modeled implementation MUST remain simulation, model, or diagnostic only and MUST NOT become a production fallback. If no implementation satisfies production admission, Molten MUST report that no production engine is available.

#### Scenario: Live admitted Raft profile becomes the default
- GIVEN a live Raft implementation profile has complete accepted production evidence for the requested environment
- WHEN a consistency-group manifest selects the environment's default admitted behavior
- THEN the manifest resolves to that exact Raft algorithm and live implementation profile
- AND the group still requires Raft membership, read-currentness, commit, snapshot, recovery, and evidence boundaries.

#### Scenario: In-process control-registry model denies production selection
- GIVEN the `in-process-raft-control-registry-v1` implementation has only in-process model or fixture evidence
- WHEN a production group resolves an engine
- THEN runtime construction denies that implementation
- AND diagnostics identify it as model or simulation only.

#### Scenario: Unknown algorithm or implementation profile denies
- GIVEN a group manifest names an unknown, unsupported, disabled, or misspelled algorithm or implementation profile
- WHEN group installation or readback validates the manifest
- THEN Molten denies the group before installation
- AND diagnostics identify the unsupported profile without falling back to another engine.

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

### Requirement: Consensus engines are registered by explicit profile identity
r[molten.consensus.engine_registry] Molten MUST maintain an explicit consensus engine registry keyed by algorithm profile id and admitted profile version. Each registry entry MUST declare implementation id, supported read consistency modes, quorum/currentness evidence classes, membership/config capabilities, production-admission status, required evidence refs, conformance receipt refs, and operator caveats.

#### Scenario: Registered Raft profile resolves
- GIVEN a control-plane group manifest names the admitted Raft algorithm profile and profile version
- WHEN runtime construction resolves the manifest through the consensus engine registry
- THEN the registry returns the Raft engine descriptor
- AND the descriptor binds supported read modes, quorum evidence class, membership capability, conformance refs, and production-admission status.

#### Scenario: Unknown engine profile denies
- GIVEN a control-plane group manifest names an unknown, disabled, or misspelled algorithm profile
- WHEN runtime construction resolves the manifest through the consensus engine registry
- THEN Molten denies runtime construction before opening control-plane state
- AND diagnostics name the unsupported profile without falling back to another engine.

### Requirement: Consensus engines implement a common control-plane interface
r[molten.consensus.engine_interface] Molten MUST define a common control-plane consensus engine boundary for proposals, linearizable reads, local-stale reads, snapshots, recovery, membership/config transitions, placement validation, and readback summaries. Engine implementations MUST return canonical commit, denial, read, snapshot, recovery, and diagnostic receipts that preserve common evidence fields while allowing engine-specific evidence refs.

#### Scenario: Proposal uses normalized commit receipt
- GIVEN a valid control-plane command and an admitted engine descriptor
- WHEN Molten proposes the command through the engine interface
- THEN the engine returns a normalized commit or denial receipt
- AND the receipt binds operation id, state-machine id, engine profile, quorum/currentness evidence, command ref, and resulting state ref.

#### Scenario: Engine-specific evidence stays attached
- GIVEN an admitted engine requires algorithm-specific evidence such as term/index, quorum certificate, ballot, view, or proof ref
- WHEN the engine emits a normalized receipt
- THEN common receipt fields remain stable for coordination and policy gates
- AND engine-specific evidence is attached as opaque evidence refs without being interpreted by engine-agnostic callers.

### Requirement: Runtime engine selection is manifest-driven and fail-closed
r[molten.consensus.runtime_engine_selection] Molten MUST construct control-plane runtimes by resolving the group manifest's algorithm profile through the consensus engine registry rather than hard-coding a single Raft-only runtime path. Runtime construction MUST deny profiles that are not registered, not policy-admitted for the requested environment, or missing required evidence refs.

#### Scenario: Manifest selects admitted production engine
- GIVEN a group manifest names a registered production-admitted engine profile with complete evidence refs
- WHEN Molten constructs the control-plane runtime
- THEN runtime construction selects the matching engine implementation
- AND readback reports the selected engine, profile version, production status, and evidence refs.

#### Scenario: Experimental engine cannot start as production
- GIVEN a group manifest requests an experimental leaderless engine profile for production use
- WHEN runtime construction checks policy and evidence admission
- THEN construction denies unless the profile has accepted production-admission policy and all required proof, simulation, placement, membership, and operator evidence
- AND diagnostics identify each missing admission class.

### Requirement: Engine admission policy is auditable
r[molten.consensus.engine_admission_policy] Molten MUST require auditable policy evidence before a consensus engine registry entry can be marked production-admitted. Admission evidence MUST include implementation identity, profile version, supported consistency modes, failure model, non-claim boundaries, conformance receipts, placement requirements, membership requirements, and proof/model or simulation evidence appropriate to the algorithm.

#### Scenario: Complete admission evidence admits engine
- GIVEN an engine registry entry has accepted implementation identity, policy refs, conformance receipts, placement requirements, membership requirements, and proof or simulation evidence
- WHEN engine admission policy evaluates the entry
- THEN the entry may be marked production-admitted for the declared environment
- AND readback includes the evidence refs used for that admission.

#### Scenario: Missing conformance evidence denies admission
- GIVEN an engine registry entry omits required conformance or proof/model evidence
- WHEN engine admission policy evaluates the entry
- THEN production admission denies
- AND diagnostics name the missing evidence class and profile id.

### Requirement: Consensus application state is engine-portable
r[molten.consensus.engine_portable_state] Molten MUST keep control-plane application state machines independent from consensus engine internals. Application state transitions MUST consume canonical command/log envelopes and normalized commit/read evidence, and MUST NOT infer application semantics from engine-specific leader identity, term, view, ballot, transport path, replica locality, or local cache state.

#### Scenario: Same canonical commands produce same state across engines
- GIVEN two admitted engine simulations apply the same canonical control-plane command sequence to the same application manifest
- WHEN each engine emits normalized commit receipts for the sequence
- THEN Molten derives the same application state ref and application receipt refs
- AND any engine-specific evidence remains outside the application-state hash.

#### Scenario: Engine internals cannot authorize application mutation
- GIVEN an engine-specific local observation lacks a normalized commit or linearizable read receipt
- WHEN a control-plane application mutation guard evaluates the observation
- THEN the mutation guard denies the observation as insufficient evidence
- AND diagnostics require normalized consensus evidence.

### Requirement: Consensus engine switchover requires canonical receipts
r[molten.consensus.engine_switchover_receipts] Molten MUST require a canonical switchover plan and receipt before changing an existing control-plane group from one consensus engine/profile to another. The receipt MUST bind source profile, target profile, source state ref, target bootstrap state ref, membership/config refs, placement refs, fencing epoch, replay/conformance evidence, currentness evidence, operator approval refs, rollback posture, decision, and diagnostics.

#### Scenario: Safe switchover emits committed transition receipt
- GIVEN an existing control-plane group has current linearizable source-state evidence and a target engine profile with admitted policy, placement, membership, and conformance evidence
- WHEN the operator executes an approved switchover plan
- THEN Molten emits a committed switchover receipt
- AND the receipt binds source state, target bootstrap state, fencing epoch, replay evidence, and rollback posture.

#### Scenario: Unsafe switchover denies before target activation
- GIVEN a switchover plan has stale source-state evidence, missing target admission, incompatible membership, missing placement evidence, or failed replay/conformance evidence
- WHEN Molten evaluates the plan
- THEN Molten denies target activation before accepting new writes through the target engine
- AND diagnostics name each failed switchover precondition.

### Requirement: Engine switchover fences stale writers and readers
r[molten.consensus.engine_switchover_fencing] Molten MUST fence stale source-engine writers and stale target-engine readers during and after a consensus engine switchover. Mutation and linearizable-read receipts MUST bind the active engine epoch so downstream gates can reject receipts from inactive or superseded epochs.

#### Scenario: Old engine write is fenced after switchover
- GIVEN a switchover receipt has activated a target engine epoch
- WHEN a delayed source-engine proposal receipt arrives for the prior epoch
- THEN Molten rejects the receipt for current mutation authority
- AND diagnostics identify the stale engine epoch.

#### Scenario: Target read waits for activated epoch
- GIVEN a target engine has bootstrap state but the switchover receipt is not committed
- WHEN a client requests a linearizable control-plane read from the target engine
- THEN Molten denies the read as production currentness evidence
- AND diagnostics require the committed switchover epoch.

### Requirement: Consensus engine capabilities are trait-separated
r[molten.consensus_engine_traits.capability_traits] Molten SHOULD expose consensus engine behavior through separate capability traits for descriptor/readback, proposal transitions, reads, snapshots, and recovery rather than requiring one monolithic trait for every operation.

#### Scenario: Engine declares only supported capabilities
- GIVEN a consensus engine descriptor for an engine that supports reads but not snapshots
- WHEN production admission evaluates the engine
- THEN supported capabilities are explicit
- AND unsupported snapshot behavior cannot be inferred from a no-op trait method.

### Requirement: Proposal transitions have a pure core
r[molten.consensus_engine_traits.pure_transition_core] Consensus proposal admission and state-machine transition logic MUST be testable as a deterministic pure core that performs no filesystem, network, process, clock, scripting, runtime scheduling, or durable-store mutation.

#### Scenario: Invalid command does not mutate runtime
- GIVEN an invalid command envelope and a cloned prior control-registry state
- WHEN the pure transition core evaluates the input
- THEN it returns a denial result
- AND the prior state remains unchanged.

### Requirement: Runtime mutation is a thin consensus shell
r[molten.consensus_engine_traits.imperative_shell] Consensus runtime shells MUST apply mutations, persistence, and receipt storage only after the pure transition core returns an admitted transition result.

#### Scenario: Passing transition updates shell after core
- GIVEN a pure proposal transition result with decision `pass`
- WHEN the runtime shell applies the result
- THEN it updates runtime state and receipt indexes according to the returned canonical values
- AND it does not recompute divergent transition semantics.

### Requirement: Unsupported engine capabilities deny explicitly
r[molten.consensus_engine_traits.unsupported_denials] Molten MUST deny unsupported consensus engine capabilities with canonical diagnostics rather than silently treating missing trait support as success.

#### Scenario: Unsupported recovery denies
- GIVEN an engine descriptor that does not admit recovery
- WHEN recovery is requested for that engine
- THEN Molten emits a deny receipt or error diagnostic before any state replacement.

### Requirement: Trait refactors preserve canonical consensus evidence
r[molten.consensus_engine_traits.hash_stability] Refactoring consensus traits MUST preserve canonical command, registry, commit, read, snapshot, and recovery refs for equivalent semantic inputs unless the change records an explicit schema migration.

#### Scenario: Duplicate proposal ref remains stable
- GIVEN a duplicate client-sequence fixture before trait decomposition
- WHEN the same fixture runs through the decomposed traits
- THEN the duplicate denial or replay receipt ref is unchanged or the migration note records the intentional difference.

### Requirement: Consensus conformance binds declared capabilities
r[molten.consensus_engine_traits.conformance] Consensus conformance receipts or tests SHOULD assert that declared engine capabilities match the implemented trait support for descriptor/readback, proposal transitions, reads, snapshots, and recovery.

#### Scenario: Declared capability is implemented
- GIVEN a consensus engine descriptor that declares a proposal capability
- WHEN conformance checks exercise the engine through the decomposed trait surface
- THEN the proposal transition trait is implemented and returns canonical transition evidence
- AND unsupported capabilities still deny explicitly.

### Requirement: Cluster config selects consensus profile
r[molten.consensus.cluster_config_selection] Molten MUST allow cluster configuration to select the control-plane consensus algorithm profile and admitted profile version before group manifest construction. Omitted selection MUST preserve the current Raft production profile default. Selected profiles MUST still pass manifest validation and consensus engine registry admission before runtime construction, and configuration MUST NOT promote experimental, disabled, unknown, or evidence-incomplete engines into production by itself.

#### Scenario: Configured Raft profile starts runtime
- GIVEN cluster config selects the admitted Raft consensus profile and profile version
- WHEN Molten builds the control-plane group manifest from that config
- THEN the manifest records the selected profile and version
- AND runtime construction resolves the matching production-admitted engine through the registry.

#### Scenario: Experimental profile is manifestable but denied for production
- GIVEN cluster config selects an experimental leaderless consensus profile with required manifest refs
- WHEN Molten builds the control-plane group manifest from that config
- THEN the manifest records the selected experimental profile
- AND production runtime construction denies the profile unless separate admission evidence and policy promote it.

#### Scenario: Unknown configured profile is rejected
- GIVEN cluster config names an unknown or misspelled consensus profile
- WHEN Molten validates the config or builds the group manifest
- THEN validation fails before runtime construction
- AND diagnostics identify the unsupported configured profile without falling back to Raft.

### Requirement: System extensions consume consistency through a typed port
r[molten.fabric_consistency.extension_port] Molten MUST expose canonical system-extension operations for consistency-group creation or attachment, proposal, declared read modes, snapshot, recovery, supported membership or configuration transition, health, drain, removal, and bounded status. Unsupported operations MUST deny explicitly. Extension code MUST NOT receive engine-internal replica objects, transport handles, durable-store handles, timers, leader pointers, terms, ballots, or runtime executors.

#### Scenario: Extension proposes a canonical command
- GIVEN a running extension generation is attached to an admitted group and holds proposal authority
- WHEN it submits a canonical command matching the application manifest
- THEN the port returns a normalized commit, denial, retryable, cancelled, or uncertain outcome with engine-specific evidence refs kept opaque.

#### Scenario: Unsupported read mode denies
- GIVEN a group profile supports local-stale and linearizable reads but not lease reads
- WHEN the extension requests a lease read
- THEN the port denies before serving state
- AND it does not silently downgrade or upgrade the read mode.

### Requirement: Consistency groups are isolated and extension-owned
r[molten.fabric_consistency.group_isolation] Molten MUST bind every consistency group to group identity, owning extension and service identity, active service generation, application state-machine manifest, engine algorithm and implementation profiles, membership/config epoch, placement ref, fencing epoch, resource envelope, policy refs, and non-claims. Commands, reads, callbacks, snapshots, or receipts from another group, extension, generation, or inactive epoch MUST NOT authorize mutation or currentness.

#### Scenario: Two extensions use independent groups
- GIVEN two system extensions attach different state machines to separate admitted groups
- WHEN both propose commands
- THEN each group orders and applies only its own canonical command schema and resource envelope.

#### Scenario: Cross-group receipt is rejected
- GIVEN a commit receipt belongs to another group or inactive engine epoch
- WHEN an extension uses it as mutation or read-currentness evidence
- THEN validation denies with a group or epoch mismatch.

### Requirement: Live engines execute over admitted fabric ports
r[molten.fabric_consistency.live_service_ports] Molten MUST run a live consensus engine as a supervised service whose peer traffic, durable log, snapshots, timers, entropy, membership, placement, fencing, resources, cancellation, and lifecycle use admitted fabric ports. The engine MUST NOT use ambient sockets, files, clocks, randomness, membership, or untracked process state as substitutes for those bindings.

#### Scenario: Live replica starts with complete bindings
- GIVEN a replica has compatible admitted transport, durable-state, time, membership, placement, fencing, policy, and resource bindings
- WHEN the supervisor starts it
- THEN it registers its protocol and reconstructs or initializes state under the selected engine and group epochs.

#### Scenario: Missing durable-log binding denies startup
- GIVEN a live profile requires durable log persistence but no compatible binding is admitted
- WHEN runtime construction runs
- THEN startup denies before protocol activation
- AND it does not substitute an in-memory log.

### Requirement: The first live Raft profile demonstrates real quorum behavior
r[molten.fabric_consistency.live_raft] Molten MUST implement the first live Raft service profile with distinct replica processes, extension-neutral canonical command application, admitted peer protocol transport, durable log and snapshots, election timers and entropy, quorum commit, declared reads, recovery, static admitted membership, and bounded status. Unsupported dynamic membership, leadership transfer, lease reads, or other optional capabilities MUST deny until separately implemented and evidenced.

#### Scenario: Majority commits while minority cannot
- GIVEN a live group has distinct admitted replicas and a partition separates a majority from a minority
- WHEN equivalent valid proposals reach both sides
- THEN only the side satisfying the profile's quorum and leadership rules can emit admitted commit evidence
- AND the minority reports unavailable, redirected, retryable, or denied outcomes without fabricating commit.

#### Scenario: Restarted replica catches up
- GIVEN a replica restarts from durable state behind the current committed boundary
- WHEN it reconnects to an admitted quorum
- THEN it catches up through log or snapshot recovery before serving current reads or acknowledging current mutations.

### Requirement: Production admission requires live distributed evidence
r[molten.fabric_consistency.production_admission] Molten MUST deny production admission for an engine implementation profile until accepted evidence demonstrates separate process identity, separate durable namespaces, admitted live transport, quorum formation and loss, elections or equivalent progress, commit, supported read currentness, process crash and restart, durable recovery, snapshot catch-up where supported, stale-epoch fencing, placement and membership checks, bounded resources, and operator recovery within the exact declared environment and failure model. Structurally valid or fabricated receipts, pure transition tests, and same-process replicas are insufficient.

#### Scenario: Complete environment-scoped evidence admits
- GIVEN a live engine profile has passing conformance and distributed failure evidence plus policy and operator approval for a declared environment
- WHEN production admission evaluates the exact implementation identity and profile version
- THEN the registry may mark it production-admitted only for that scope.

#### Scenario: Same-process fixture cannot prove production quorum
- GIVEN an engine fixture models several replicas inside one process and emits structurally valid quorum receipts
- WHEN production admission runs
- THEN it denies live production status
- AND retains the fixture only as model, simulation, or diagnostic evidence.

### Requirement: Consistency evidence remains off protocol hot paths
r[molten.fabric_consistency.evidence_granularity] Molten MUST emit canonical evidence for group admission, implementation and configuration epochs, selected semantic commit boundaries, quorum-backed reads, snapshots, recovery, material failures, fencing changes, and aggregate health. The default production profile MUST NOT require a heavyweight authority receipt for every heartbeat, vote request, append message, replication acknowledgement, timer tick, or local log read.

#### Scenario: Batched commit evidence remains verifiable
- GIVEN an engine replicates many internal protocol messages to establish one or more committed application boundaries
- WHEN its selected evidence profile emits a commit-range or checkpoint receipt
- THEN the receipt binds the group, engine epoch, command or range refs, quorum/currentness evidence, resulting state ref, and non-claims
- AND individual protocol packets need not each have standalone receipts.

#### Scenario: Diagnostic tracing is opt-in
- GIVEN an operator selects a bounded diagnostic profile with detailed protocol tracing
- WHEN the profile is activated
- THEN its additional resource budget and retention are explicit
- AND it does not become the production default.

### Requirement: Consistency service status is bounded and honest
r[molten.fabric_consistency.operator_readback] Molten MUST expose bounded authorized readback for group and owner identity, engine algorithm and implementation profile, production-admission scope, service and engine epochs, local replica role, membership/config ref, placement ref, fencing ref, durable and applied boundaries, supported read modes, health, quorum observation, snapshot/recovery state, resource use, evidence refs, and non-claims. A local observation MUST be labeled local unless supported by admitted quorum or currentness evidence.

#### Scenario: Minority status remains local
- GIVEN a replica is partitioned from quorum
- WHEN an operator reads its status
- THEN the report labels role, log, and peer observations as local or stale
- AND does not claim current leadership, commit, or cluster health without quorum evidence.

### Requirement: Live consistency validation covers success and failure
r[molten.fabric_consistency.final_validation] Molten MUST include shared engine conformance, deterministic simulation, and multi-process live tests covering extension isolation, protocol binding, election, commit, reads, quorum loss, partitions, process crash, durable restart, snapshot catch-up, stale leader and generation fencing, resource exhaustion, cancellation, drain, cleanup, model-profile production denial, evidence granularity, and non-claims.

#### Scenario: Live profile passes its declared fault matrix
- GIVEN distinct processes, endpoints, durable namespaces, and a deterministic fault plan
- WHEN the live profile suite executes
- THEN observed commits, denials, reads, recovery, fencing, status, and offline evidence satisfy the declared engine contract.

#### Scenario: False quorum evidence fails validation
- GIVEN a fixture emits commit or current-read evidence without the required distinct admitted replica acknowledgements
- WHEN offline validation runs
- THEN validation denies with a quorum-evidence diagnostic.

### Requirement: Fast-path hazard models are explicit and model-only
r[molten.consensus.fast_path_model.profile] Molten MUST represent a consensus fast-path hazard model with an explicit profile identity, source-reference cohort, base-engine model identity, crash-fault assumptions, node and proposer bounds, derived majority and superquorum rules, command/conflict profile, view and recovery bounds, invariant set, evidence profile, and non-claims. The profile MUST remain pure-model or deterministic-simulation only and MUST deny live or production engine selection.

#### Scenario: Complete three-replica model is admitted
- GIVEN a bounded three-replica crash-fault profile with a pinned source cohort, compatible base model, derived quorum rules, conflict contract, invariants, and model-only claim profile
- WHEN model preflight runs
- THEN it produces a canonical admitted model plan
- AND reports that the fast-path superquorum contains every replica.

#### Scenario: Model profile cannot select production
- GIVEN a structurally valid fast-path model has no distinct-process live evidence
- WHEN a live or production group attempts to select it
- THEN selection denies before runtime construction
- AND diagnostics preserve the pure-model claim boundary.

### Requirement: Base models declare fast-path ordering prerequisites
r[molten.consensus.fast_path_model.base_prerequisites] A fast-path model MUST bind evidence that the base model preserves proposal order in log and execution order for conflicting commands proposed by one proposer and preserves proposer receive order in proposal order. A model whose buffering can reorder receive and proposal MAY remain compatible only when fast acknowledgement waits for equivalent proposal-order evidence. A model that can reorder conflicting proposals at execution MUST deny transparent fast-path compatibility.

#### Scenario: Ordered proposer model is compatible
- GIVEN a base model appends conflicting commands in proposal order and executes them in log order
- WHEN fast-path compatibility validates its declared proposer contract
- THEN the ordering prerequisite may pass subject to the remaining quorum, conflict, view, and recovery requirements.

#### Scenario: Buffered reorder requires a later acknowledgement boundary
- GIVEN a base proposer may receive command A before command B but buffer and propose B first
- WHEN the acceleration layer can observe only receive order
- THEN transparent receive-time acknowledgement denies
- AND compatibility requires proposal-order evidence or original-path fallback.

#### Scenario: Execution reorder is incompatible
- GIVEN a base model may propose conflicting command A before command B but execute B first
- WHEN compatibility admission runs
- THEN the transparent fast-path profile denies
- AND does not treat model-checking of the base protocol alone as sufficient.

### Requirement: Conflict classification is pure, versioned, and conservative
r[molten.consensus.fast_path_model.conflict_contract] A modeled fast path MUST bind a versioned extension-owned conflict contract to exact command and state-machine schemas. The conflict function MUST be deterministic and side-effect free and MUST report conflict whenever command order can affect application state or either command response. Unknown schemas, aliases, ranges, predicates, preconditions, analysis failures, and unsupported operations MUST conservatively conflict and use the original path.

#### Scenario: Independent keys can use the fast path
- GIVEN two key-value commands address distinct canonical keys and their responses do not depend on shared state
- WHEN the bound conflict contract evaluates them
- THEN it may classify them as non-conflicting for fast-path modeling.

#### Scenario: Unknown dependency falls back safely
- GIVEN a command contains an unsupported range predicate or unresolved alias
- WHEN conflict classification cannot establish independence
- THEN it classifies the command as conflicting
- AND the command remains eligible for the original path rather than being rejected or fast-committed.

### Requirement: Stable-view fast commit requires one view and all proposer promises
r[molten.consensus.fast_path_model.stable_view] A modeled fast commit MUST bind one acceleration view and one matching base-engine view, obtain acknowledgements from the derived same-view fast superquorum, and include a compatible ordering promise from every active original-path proposer in that view. Acknowledgements or promises from different views MUST NOT combine into a fast commit.

#### Scenario: Same-view superquorum commits
- GIVEN a command is conflict-free, both paths are in the same normal view, the fast superquorum acknowledges that view, and every active proposer promises compatible ordering
- WHEN the client evaluates the attempt
- THEN the model may classify the command as fast-committed.

#### Scenario: View-straddled acknowledgements fail
- GIVEN individually valid acknowledgements were issued across two acceleration or base-engine views
- WHEN their union would meet the numeric superquorum size
- THEN the fast commit still fails
- AND the original path remains available for fallback.

### Requirement: Both paths converge on one canonical operation
r[molten.consensus.fast_path_model.fallback_identity] The modeled fast and original paths MUST carry the same canonical command ref, client session and sequence, group, extension generation, application schema, policy/authority/resource cohort, and engine epoch. Fast-path failure MUST fall back to the original path without changing operation identity. Convergence MUST apply and reply to the operation at most once.

#### Scenario: Conflict falls back without changing identity
- GIVEN a fast attempt encounters an in-flight conflicting command
- WHEN the fast superquorum cannot form
- THEN the original path continues with the same canonical operation identity
- AND a later commit applies the operation once.

#### Scenario: Duplicate path completion does not duplicate effects
- GIVEN the client observed a fast commit and the original path later reaches the same command
- WHEN the state machine processes the converged record
- THEN client-session and command identity suppress duplicate application and duplicate authoritative reply.

### Requirement: View changes recover and order prior fast commits first
r[molten.consensus.fast_path_model.view_change_recovery] The modeled acceleration layer MUST track a view independently from the base engine. After a base-engine view change, it MUST pause new fast admission, agree on the last normal view's recoverable fast-command set, carry any previously accepted recovery set forward, commit the recovery set or an explicit no-op recovery marker through the original path, and only then admit commands in the new normal view. Recovered commands MUST precede every conflicting uncommitted command admitted by the new view.

#### Scenario: Leader fails after fast reply
- GIVEN a client received a valid fast reply and the original-path proposer fails before canonical commit
- WHEN a new proposer recovers the last normal view
- THEN the acknowledged command appears in the agreed recovery set
- AND commits through the original path before conflicting new-view commands.

#### Scenario: Empty recovery still creates a boundary
- GIVEN recovery finds no possibly fast-committed command in the last normal view
- WHEN the new proposer completes recovery
- THEN it commits an explicit no-op recovery marker before accepting normal new-view work.

### Requirement: The fault corpus checks fast-path composition invariants
r[molten.consensus.fast_path_model.fault_corpus] Molten MUST provide bounded positive and negative schedules for three-replica and five-replica profiles covering non-conflicting fast commit, conflict fallback, original-only operation, view-straddled acknowledgements, missing proposer promises, leader failure after fast reply, stale conflicting entries, partitions, quorum loss, interrupted and cascading recovery, restart, convergence, and duplicate suppression. The model MUST check recoverability, no conflicting predecessor, committed-order agreement, execution agreement, linearizable conflicting-command order, and at-most-once application.

#### Scenario: Stale conflicting predecessor is detected
- GIVEN a new proposer carries an uncommitted conflicting command from an older view ahead of a recovered fast-committed command
- WHEN invariant evaluation examines the candidate execution order
- THEN the run fails with a no-conflicting-predecessor counterexample.

#### Scenario: Three-replica failure preserves only the original path
- GIVEN a three-replica profile loses one replica
- WHEN the remaining majority can run the base protocol but the fast superquorum requires every replica
- THEN the model reports fast-path unavailable and original-path availability separately
- AND does not promote fallback latency to fast-path success.

### Requirement: Model evidence is replayable and bounded
r[molten.consensus.fast_path_model.evidence] Molten MUST emit canonical model profile, source cohort, run, transition trace, fault, recovery, invariant, coverage, first-divergence, minimized-counterexample, and final-state evidence under explicit finite bounds. Exported repro bundles MUST identify the model/runtime inputs needed for deterministic replay and MUST NOT contain live-engine or measured-performance claims.

#### Scenario: Counterexample replays from canonical inputs
- GIVEN bounded exploration finds a recovery-order violation and minimizes its causal schedule
- WHEN the repro bundle is replayed with the same canonical model inputs
- THEN it reaches the same failure class and first violating boundary.

#### Scenario: Unexplored state space remains visible
- GIVEN configured bounds stop exploration before all eligible alternatives are visited
- WHEN evidence is finalized
- THEN coverage reports the unexplored alternatives
- AND the run cannot claim exhaustive verification.

### Requirement: External reference conformance does not transfer proof
r[molten.consensus.fast_path_model.reference_conformance] Molten SHOULD compare independently expressed named scenarios, assumptions, and invariant outcomes against the pinned Jetpack paper and artifact cohort. Reference conformance MUST record source identity, compared behavior, mismatches, unsupported assumptions, and license posture, and MUST NOT treat external TLA+ success, tests, or benchmarks as proof of Molten code or performance.

#### Scenario: Reference mismatch blocks conformance
- GIVEN a Molten recovery scenario permits new-view work before the recovery marker while the pinned reference requires recovery priority
- WHEN reference conformance runs
- THEN it reports the semantic mismatch
- AND does not issue a passing conformance decision.

### Requirement: Fast-path model claims remain bounded
r[molten.consensus.fast_path_model.nonclaims] Fast-path model evidence MUST state that it does not prove the external artifact, a live Molten base engine, real transport, durability, timing, production linearizability, throughput, latency improvement, Byzantine tolerance, interactive transactions, arbitrary conflict predicates, or release readiness. A stronger profile MUST require its own implementation and environment evidence.

#### Scenario: Benchmark citation cannot admit production
- GIVEN the source cohort reports lower latency in an external geo-distributed benchmark
- WHEN Molten production admission evaluates only model and citation evidence
- THEN admission denies
- AND identifies missing live implementation, environment, failure, and performance evidence.

### Requirement: Fast-path model validation covers success and failure
r[molten.consensus.fast_path_model.validation] Molten MUST include positive and negative tests for profile admission, quorum derivation, conflict classification, stable-view promises, fallback identity, duplicate suppression, view-change recovery, recovery ordering, fault schedules, invariants, deterministic replay, minimization, source-reference conformance, bounded evidence, non-claims, and live/production denial.

#### Scenario: Valid bounded model suite passes
- GIVEN admitted three-replica and five-replica profiles and their complete positive and negative fixture cohorts
- WHEN focused validation runs
- THEN expected safe traces pass, expected hazards produce the named counterexamples, and model-only evidence validates offline.

#### Scenario: False non-conflict fixture fails
- GIVEN a fixture deliberately classifies two response-dependent commands as non-conflicting
- WHEN exploration finds an execution that changes state or response order
- THEN the semantic invariant fails
- AND the fixture cannot satisfy conflict-contract or production evidence gates.
