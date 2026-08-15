## MODIFIED Requirements

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

## ADDED Requirements

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
