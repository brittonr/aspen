# Node Runtime Specification

## Purpose

Defines the `node-runtime` capability.

## Requirements

### Requirement: Production deployment profile
r[molten.prod_ops.deployment_profile] Molten MUST define an explicit production node deployment profile that records required adapters, state-root layout, source-gate evidence refs, resource limits, redaction/logging settings, live transport settings, and startup/shutdown expectations as canonical evidence.

#### Scenario: Node starts with production profile evidence
- GIVEN an operator starts a node under the production deployment profile
- WHEN startup receipts are emitted
- THEN the receipts bind the profile ref, required adapter refs, resource limits, source-gate refs, and redaction settings
- AND startup denies if required profile evidence is missing or stale.

### Requirement: Production profile scalar fields use domain contracts
r[molten.prod_ops.profile_domain_contracts.scalar_types] The production node deployment profile MUST validate evidence refs, profile names, state roots, and state layout directory fields with domain-specific Nickel contracts before exporting profile JSON.

#### Scenario: Valid scalar domains export
- GIVEN a production node profile whose refs use the supported BLAKE3 content-ref syntax, whose profile name is non-empty, whose state root is absolute, and whose layout directories are safe relative directory names
- WHEN the operator exports the profile through Nickel
- THEN the export succeeds and preserves the reviewed profile field names and values

#### Scenario: Malformed evidence ref fails early
- GIVEN a production node profile containing a malformed, uppercase, empty, or non-BLAKE3 source-gate ref
- WHEN the operator exports the profile through Nickel
- THEN the export fails before any production readiness receipt can bind that profile

#### Scenario: Unsafe state path fails early
- GIVEN a production node profile whose state root is relative or whose layout directory is absolute, empty, current-directory, parent-directory, or path-traversal shaped
- WHEN the operator exports the profile through Nickel
- THEN the export fails with a contract diagnostic for the path field

### Requirement: Production resource limits are positive integers
r[molten.prod_ops.profile_domain_contracts.positive_limits] Production profile resource limits MUST be positive integer values at the Nickel contract boundary.

#### Scenario: Positive integer limits export
- GIVEN a production profile whose queue, receipt, store, delivery-latency, and recovery-time limits are positive integers
- WHEN the operator exports the profile
- THEN the exported JSON contains numeric limit values accepted by production readiness evidence generation

#### Scenario: Non-positive or fractional limit fails
- GIVEN a production profile with a zero, negative, fractional, or non-numeric resource limit
- WHEN the operator exports the profile through Nickel
- THEN the export fails before startup or production-readiness evidence can treat the limit as reviewed

### Requirement: Production profile vocabularies are contract-bound
r[molten.prod_ops.profile_enum_contracts.allowed_vocabularies] Production profile arrays for required adapters, redaction settings, live transport settings, startup expectations, and shutdown expectations MUST accept only reviewed vocabulary values through Nickel contracts.

#### Scenario: Reviewed vocabulary values export
- GIVEN a production profile whose adapter, redaction, transport, startup, and shutdown arrays contain only reviewed vocabulary values
- WHEN the operator exports the profile through Nickel
- THEN the export succeeds and the exported values remain the reviewed strings

#### Scenario: Misspelled vocabulary value fails
- GIVEN a production profile with a misspelled or unreviewed adapter, redaction setting, transport setting, startup expectation, or shutdown expectation
- WHEN the operator exports the profile through Nickel
- THEN the export fails before the unreviewed string can be bound into production readiness evidence

### Requirement: Vocabulary growth is reviewed
r[molten.prod_ops.profile_enum_contracts.reviewed_growth] New production profile vocabulary values MUST be added through an explicit contract and documentation update rather than accepted as arbitrary text.

#### Scenario: New adapter requires contract update
- GIVEN an operator wants to require a new production adapter in the deployment profile
- WHEN the adapter name is not present in the reviewed vocabulary contract
- THEN Nickel export rejects the profile until the contract and operator documentation are updated

### Requirement: Production resource thresholds are named
r[molten.prod_ops.profile_named_units.named_thresholds] Production deployment profile resource limits MUST be expressed in Nickel through named unit and threshold constants rather than unexplained numeric literals in the concrete profile body.

#### Scenario: Named thresholds define profile limits
- GIVEN a reviewer inspects the production profile source
- WHEN they read queue, receipt, store, delivery-latency, and recovery-time limits
- THEN each limit is derived from a named Nickel constant that states the unit and reviewed threshold meaning

#### Scenario: Threshold change is review-visible
- GIVEN a production resource threshold changes
- WHEN the profile diff is reviewed
- THEN the diff names the threshold being changed rather than exposing only an unexplained numeric literal

### Requirement: Named units preserve exported profile values
r[molten.prod_ops.profile_named_units.export_stability] Replacing production profile numeric literals with named Nickel constants MUST preserve the reviewed exported JSON values unless the same change explicitly updates the threshold.

#### Scenario: Current profile export remains stable
- GIVEN the current production profile is rewritten to use named unit and threshold constants
- WHEN the operator exports the profile through Nickel
- THEN the exported resource-limit values match the previous reviewed profile export

#### Scenario: Unintended numeric drift is caught
- GIVEN a named unit or threshold edit changes an exported resource-limit value without an explicit threshold-review update
- WHEN profile fixture validation runs
- THEN validation fails and reports export drift before production readiness receipts are updated

### Requirement: Production profile requires startup evidence inputs
r[molten.prod_ops.profile_invariants.required_evidence] Production deployment profile export MUST fail unless required evidence arrays are non-empty and the required adapter list includes the reviewed core production adapter set.

#### Scenario: Complete startup evidence exports
- GIVEN a production profile with at least one source-gate input and all reviewed core production adapters listed
- WHEN the operator exports the profile through Nickel
- THEN the export succeeds and the startup receipt can bind the declared evidence and adapter refs

#### Scenario: Missing startup evidence fails
- GIVEN a production profile with no source-gate inputs or with a required core production adapter omitted
- WHEN the operator exports the profile through Nickel
- THEN the export fails before startup receipts can claim the profile is deployment-ready

### Requirement: Production state layout directories are distinct
r[molten.prod_ops.profile_invariants.layout_distinct] Production deployment profile export MUST fail when two logical state layout directories resolve to the same relative directory name.

#### Scenario: Distinct layout directories export
- GIVEN a production profile whose ledger, Redb, chunk, identity, retention, and inbox directories are distinct relative directory names
- WHEN the operator exports the profile
- THEN the exported state layout preserves each logical directory mapping

#### Scenario: Layout collision fails
- GIVEN a production profile that assigns the same relative directory name to two logical state layout entries
- WHEN the operator exports the profile through Nickel
- THEN the export fails before runtime state can be initialized with an ambiguous layout

### Requirement: Production resource limits are internally coherent
r[molten.prod_ops.profile_invariants.resource_relationships] Production deployment profile export MUST fail when resource limits contradict each other, including store capacity smaller than receipt capacity or timing limits that invert the reviewed delivery and recovery envelope.

#### Scenario: Coherent limits export
- GIVEN a production profile whose store capacity can contain the maximum receipt size and whose timing limits preserve the reviewed delivery and recovery envelope
- WHEN the operator exports the profile
- THEN the resource-limit block exports as reviewed production profile evidence

#### Scenario: Contradictory limits fail
- GIVEN a production profile whose store limit is smaller than the maximum receipt size or whose timing limits contradict the reviewed delivery and recovery envelope
- WHEN the operator exports the profile through Nickel
- THEN the export fails with a resource-limit invariant diagnostic

### Requirement: Production profile contracts are reusable
r[molten.prod_ops.profile_contract_library.reusable_module] Production deployment profile Nickel contracts and constants MUST live in a reusable module that can be imported by the checked-in profile and by validation fixtures.

#### Scenario: Profile and fixtures share one contract
- GIVEN the checked-in production profile and profile validation fixtures
- WHEN they are evaluated through Nickel
- THEN they import the same reusable production profile contract module rather than carrying copied schema definitions

#### Scenario: Contract update applies to all profiles
- GIVEN a production profile contract is tightened or extended
- WHEN profile instances and fixtures are exported
- THEN each import path observes the same reviewed contract behavior

### Requirement: Checked-in profile remains a concrete instance
r[molten.prod_ops.profile_contract_library.instance_profile] The operator-facing production profile file MUST remain a concrete deployment profile instance that applies the reusable contract to reviewed values.

#### Scenario: Operator exports concrete profile
- GIVEN an operator follows the production deployment runbook
- WHEN they export the checked-in production profile file
- THEN the exported JSON represents the concrete reviewed profile instance, not only the reusable contract module

### Requirement: Runtime does not evaluate Nickel for startup
r[molten.prod_ops.profile_contract_library.no_runtime_nickel] Node startup MUST continue to consume checked exported profile JSON and MUST NOT introduce runtime Nickel evaluation as part of production startup side effects.

#### Scenario: Startup uses exported profile evidence
- GIVEN a production node startup receives profile evidence
- WHEN startup validation runs
- THEN it validates the exported profile JSON and bound receipts without invoking a Nickel interpreter at runtime

### Requirement: Production profile contracts have positive and negative fixtures
r[molten.prod_ops.profile_contract_fixtures.positive_negative] Production profile Nickel contracts MUST be covered by positive fixtures for reviewed valid profiles and negative fixtures for malformed refs, missing evidence arrays, unsafe paths, vocabulary typos, invalid resource limits, cross-field invariant failures, and metadata errors.

#### Scenario: Reviewed profile fixture exports
- GIVEN the checked-in production profile fixture represents the reviewed valid profile
- WHEN fixture validation runs
- THEN Nickel export succeeds and the exported JSON matches the reviewed profile expectation

#### Scenario: Invalid profile fixtures fail
- GIVEN negative fixtures that each violate one production profile contract or invariant
- WHEN fixture validation runs
- THEN each negative fixture fails Nickel export and reports the expected failure class

### Requirement: Profile fixture validation is deterministic
r[molten.prod_ops.profile_contract_fixtures.validation_gate] Production profile fixture validation MUST run without live network, production credentials, mutable state roots, or ambient filesystem assumptions beyond reading source-controlled fixture files.

#### Scenario: Fixture gate runs locally
- GIVEN the repository checkout contains the profile contract and fixture files
- WHEN the profile fixture validation command runs
- THEN it deterministically reports valid positive exports and rejected negative exports using only source-controlled inputs

#### Scenario: Fixture regression blocks profile evidence update
- GIVEN a profile contract edit accidentally accepts an invalid fixture or changes the valid export unexpectedly
- WHEN the profile fixture validation command runs
- THEN validation fails before production readiness receipt expectations are updated

### Requirement: Profile fixtures are static-contract evidence only
r[molten.prod_ops.profile_contract_fixtures.evidence_boundary] Production profile fixture results MUST NOT replace runtime startup receipts, source-gate freshness checks, adapter conformance evidence, resource-pressure observations, or production drill receipts.

#### Scenario: Fixture pass does not grant runtime trust
- GIVEN all profile contract fixtures pass
- WHEN a production node startup or release gate needs live authority, source-gate, adapter, resource, or drill evidence
- THEN the normal subsystem receipts remain required and fixture results alone are insufficient

### Requirement: Production profile exports carry schema metadata
r[molten.prod_ops.profile_schema_metadata.root_identity] Production deployment profile exports MUST include explicit schema identity, schema version, source language, and stable profile identity metadata.

#### Scenario: Metadata identifies reviewed profile export
- GIVEN a production profile exported from the reviewed Nickel contract boundary
- WHEN the exported JSON is inspected or bound into evidence
- THEN it includes metadata naming the production profile schema, schema version, source language, and profile identity

#### Scenario: Missing metadata fails validation
- GIVEN an exported profile JSON document without required schema or source-language metadata
- WHEN deployment-profile or startup validation evaluates it
- THEN validation fails before accepting the profile as production evidence

### Requirement: Profile metadata is bound into receipts
r[molten.prod_ops.profile_schema_metadata.receipt_binding] Deployment-profile and startup receipts MUST bind production profile metadata together with the profile content ref and MUST reject stale, unsupported, or tampered metadata bindings.

#### Scenario: Receipt binds matching metadata
- GIVEN a production profile export with supported metadata and a matching content ref
- WHEN deployment-profile evidence is generated
- THEN the receipt records the schema, version, source language, profile identity, and profile ref consistently

#### Scenario: Tampered metadata denies
- GIVEN a profile receipt whose schema, version, source language, profile identity, or profile ref no longer matches the exported profile under review
- WHEN validation runs
- THEN validation denies the profile evidence before startup can rely on it

### Requirement: Profile metadata is evidence-only
r[molten.prod_ops.profile_schema_metadata.evidence_only] Production profile metadata MUST NOT grant authority, source-gate acceptance, adapter readiness, provenance trust, resource sufficiency, retention clearance, or live transport correctness.

#### Scenario: Metadata does not replace subsystem gates
- GIVEN a profile export with valid metadata
- WHEN a subsystem requires authority, source-gate, adapter, resource, retention, or transport evidence
- THEN that subsystem still requires its own matching gate receipts and MUST NOT rely on metadata alone

### Requirement: State backup and restore drill evidence
r[molten.prod_ops.state_backup_restore] Molten MUST provide backup and restore drill evidence for local ledgers, Redb stores, chunk-store state, retention pins, node identity metadata, and source-gate refs, and MUST verify restored refs before normal control operations resume.

#### Scenario: Tampered backup denies restore
- GIVEN a backup bundle with a missing or tampered ledger, chunk, Redb index, retention, or source-gate member
- WHEN a restore drill verifies the bundle
- THEN Molten emits a deny receipt and MUST NOT resume normal production control operations from that restored state.

### Requirement: Production observability and SLO evidence
r[molten.prod_ops.observability_slo] Molten MUST emit structured observability evidence for node health, adapter health, queue depth, control-loop liveness, resource pressure, source-gate freshness, retention drift, receipt import/export failures, and live transport delivery health.

#### Scenario: Observability snapshot reports degraded resource pressure
- GIVEN a running production-profile node with queue or resource pressure over its configured threshold
- WHEN an observability snapshot is emitted
- THEN the snapshot records the degraded status, relevant resource refs, and operator diagnostics without treating logs as canonical pass evidence.

### Requirement: Upgrade and rollback drills
r[molten.prod_ops.upgrade_rollback_drill] Molten MUST support upgrade and rollback drills that bind migration receipts, copied-state smoke or dogfood evidence, rollback eligibility, irreversible-operation exclusions, and post-rollback verification receipts.

#### Scenario: Irreversible migration blocks rollback claim
- GIVEN an upgrade plan includes an irreversible migration or destructive retention action without explicit rollback exclusion evidence
- WHEN rollback eligibility is evaluated
- THEN the rollback drill emits a deny receipt rather than claiming safe rollback.

### Requirement: Operator runbooks are evidence-backed
r[molten.prod_ops.operator_runbooks] Molten SHOULD provide operator runbooks for init, run, status, stop, evidence export, source-gate refresh, backup, restore, upgrade, rollback, and emergency stop, and MUST distinguish canonical receipts from auxiliary logs or summaries.

#### Scenario: Runbook points to canonical artifacts
- GIVEN an operator follows a production runbook
- WHEN the runbook references a successful operation
- THEN it names the canonical receipt, evidence bundle, or verification artifact required for review instead of relying on terminal output alone.

### Requirement: Runtime-managed Iroh protocol router
r[molten.node_runtime.iroh_protocol_router] Molten MUST provide a runtime-managed Iroh protocol router boundary that installs, replaces, removes, and shuts down ALPN protocol handlers only after explicit admission by authority, policy, resource, and evidence inputs.

#### Scenario: Admitted ALPN handler is installed
- GIVEN a node-control request to install an Iroh protocol handler with valid ALPN, authority refs, policy refs, resource refs, and supporting evidence refs
- WHEN the router admission core evaluates the request
- THEN it returns a pass decision with the installed handler descriptor and generation
- AND the live router shell advertises the ALPN only after the pass receipt is recorded.

#### Scenario: Unsupported ALPN denies before delivery
- GIVEN an incoming Iroh connection for an ALPN that is not registered or no longer registered
- WHEN the router evaluates the connection
- THEN Molten emits deny evidence for unsupported ALPN
- AND no envelope frame is delivered to node-control, protocol-session, plugin, dataspace, or service state.

#### Scenario: Replacement advances generation and shuts down prior handler
- GIVEN an existing registered ALPN handler
- WHEN an admitted replacement is applied
- THEN the router records the previous handler generation, advances the replacement generation, and binds shutdown evidence for the previous handler
- AND new connections use the replacement handler while existing connections follow the configured drain policy.

### Requirement: Iroh protocol router receipts
r[molten.node_runtime.iroh_protocol_router_receipts] Molten MUST emit canonical router receipts for protocol install, replacement, removal, shutdown, unsupported-ALPN denial, and stale-generation denial.

#### Scenario: Removed handler no longer advertises ALPN
- GIVEN a registered ALPN handler with a current generation
- WHEN an admitted remove request succeeds
- THEN the router receipt records decision `pass`, operation `remove`, the removed generation, and handler shutdown evidence
- AND subsequent connection attempts for that ALPN deny before frame delivery.

#### Scenario: Stale generation cannot replace handler
- GIVEN a replacement request that references a stale prior generation
- WHEN the router admission core evaluates the request
- THEN it emits a deny receipt with stale-generation diagnostics
- AND the live advertised ALPN map remains unchanged.

### Requirement: Framed canonical envelope stream over Iroh
r[molten.node_runtime.iroh_framed_envelope_stream] Molten MUST support a bounded framed-envelope stream over Iroh bidirectional connections where each frame carries canonical Preserves envelope bytes, declared envelope refs, peer/node ids, sequence, ALPN, and limit-profile evidence.

#### Scenario: Valid frame delivers canonical envelope
- GIVEN a framed stream session for an admitted ALPN and a frame whose canonical Preserves bytes hash to the declared envelope ref
- WHEN the framed-envelope validator checks the frame against configured byte and sequence limits
- THEN it emits a pass receipt binding the frame length, actual envelope ref, declared envelope ref, ALPN, peer, node, and sequence
- AND the envelope may be handed to the normal node-control or protocol-session admission path.

#### Scenario: Oversized frame denies before parsing payload
- GIVEN a frame whose declared length exceeds the configured max frame bytes
- WHEN the framed-envelope validator receives the frame
- THEN it emits a deny receipt for oversized frame
- AND the payload is not parsed, delivered, or written into runtime state.

#### Scenario: Declared envelope ref mismatch denies
- GIVEN a frame with canonical Preserves bytes whose hash differs from the declared envelope ref
- WHEN the framed-envelope validator checks the frame
- THEN it emits a deny receipt with declared and actual refs
- AND the frame is excluded from deterministic pass evidence and live delivery.

### Requirement: Iroh service-session streaming patterns
r[molten.node_runtime.iroh_service_session_streaming] Molten SHOULD model local and remote service interactions over admitted Iroh framed streams with explicit unary request/response, server-streaming, client-streaming, and bidirectional-streaming session descriptors while preserving canonical Preserves envelope identity for every remote frame.

#### Scenario: Unary request response binds same local and remote model
- GIVEN a service method that can run locally or over an admitted Iroh framed stream
- WHEN Molten records the request and response session
- THEN both local and remote paths bind the same service id, operation id, interaction kind, capability refs, policy refs, resource refs, request ref, and response ref
- AND the remote path additionally binds ALPN, peer, node, stream, and frame receipts.

#### Scenario: Streaming session applies per-frame admission
- GIVEN a server-streaming, client-streaming, or bidirectional service session over Iroh
- WHEN a stream update frame is received
- THEN each update is validated as a bounded canonical Preserves envelope with sequence and flow-control evidence
- AND malformed, oversized, stale, or unauthorized updates deny without mutating service state.

#### Scenario: Postcard-only IRPC wire format is not canonical Molten boundary
- GIVEN an IRPC-style Rust service interaction pattern
- WHEN Molten exposes that interaction across node or process boundaries
- THEN the canonical Molten boundary remains versioned Preserves envelope frames
- AND postcard or Rust-only message serialization may only be an internal implementation detail behind explicit conversion evidence.

### Requirement: Network diagnostics reports
r[molten.node_runtime.network_diagnostics_report] Molten MUST emit canonical network diagnostics reports that bind NAT classification, UDP reachability, direct-path status, relay latency observations, port-map protocol availability, interface or route snapshot refs, diagnostics, and explicit evidence-only caveats.

#### Scenario: Local diagnostics report is evidence-only
- GIVEN a node runs a local network diagnostics report
- WHEN the report is emitted
- THEN it records pass, deny, or degraded diagnostic checks for NAT, UDP, relay, direct path, and port-map protocol observations
- AND the report states that diagnostics do not grant authority, policy admission, resource rights, provenance trust, source-gate acceptance, retention clearance, transport correctness, or deterministic replay trust.

#### Scenario: Live-only observation is marked non-replayable
- GIVEN a diagnostics report includes unrecorded live network observations
- WHEN the report is used in review evidence
- THEN the report marks those observations as non-replayable diagnostics
- AND deterministic pass evidence must come from a separately recorded replay log or gate.

### Requirement: Connectivity probe receipts
r[molten.node_runtime.connectivity_probe_receipts] Molten MUST emit canonical receipts for diagnostic accept/connect probes between nodes, including direct path, relay path, timeout, denial, and degraded outcomes.

#### Scenario: Relay-only path reports degraded or scoped pass
- GIVEN two nodes can connect through relay but not through a direct path
- WHEN the connectivity probe completes
- THEN the receipt records the relay path, direct-path failure diagnostics, and a degraded or scoped pass decision
- AND it does not claim broad direct-connectivity support.

#### Scenario: Failed probe denies without state mutation
- GIVEN a diagnostic peer cannot authenticate, connect, or satisfy the expected endpoint identity
- WHEN the connectivity probe evaluates the attempt
- THEN it emits deny evidence
- AND node-control, protocol-session, dataspace, and service state remain unchanged.

### Requirement: Latest-state network watcher snapshots
r[molten.node_runtime.network_watcher_snapshot] Molten SHOULD maintain bounded latest-state snapshots for network interface, address, default-route, relay, endpoint-online, and transport health changes without requiring unbounded event queues.

#### Scenario: Watcher snapshot records latest route state
- GIVEN route or interface state changes several times before an observer reads it
- WHEN Molten emits a watcher snapshot
- THEN the snapshot records the latest observed state and bounded diagnostics
- AND it does not claim to preserve every intermediate route-change event.

### Requirement: Deny-by-default port mapping policy
r[molten.node_runtime.port_mapping_policy] Molten MUST treat UPnP, PCP, NAT-PMP, or equivalent port mapping attempts as network mutations that deny by default unless explicit requester, node identity, authority refs, policy refs, resource refs, port/protocol scope, duration bounds, and operator evidence are supplied.

#### Scenario: Port map attempt without authority denies
- GIVEN a node requests a port mapping without required authority, policy, resource, or operator evidence
- WHEN Molten evaluates the port-mapping decision
- THEN it emits a deny receipt before attempting the mapping
- AND no router or gateway state is mutated by Molten.

#### Scenario: Probe-only port-map report does not mutate network state
- GIVEN an operator asks whether port mapping protocols appear available
- WHEN Molten runs a probe-only diagnostics check
- THEN it emits availability diagnostics without creating, refreshing, or deleting a mapping
- AND the receipt distinguishes probe evidence from mutation evidence.

### Requirement: Metrics snapshots and OpenMetrics export
r[molten.node_runtime.metrics_snapshot] Molten SHOULD emit bounded metrics snapshot receipts and MAY expose an OpenMetrics-compatible read-only endpoint for operational counters, gauges, and histograms covering node-control, live transport, queue depth, delivery idempotency, chunk/artifact sync, and resource pressure.

#### Scenario: Metrics labels are bounded and redacted
- GIVEN a metrics snapshot includes peer, topic, route, ticket, path, or ref-like dimensions
- WHEN Molten validates the snapshot
- THEN labels are bounded and redacted according to policy
- AND raw secrets, tickets, full paths, high-cardinality user inputs, and hidden refs are not exposed.

#### Scenario: Metrics support observability but not admission
- GIVEN a passing metrics snapshot receipt
- WHEN a downstream operation attempts to use it as authority, policy, provenance, source-gate, retention, or execution evidence
- THEN the downstream gate denies unless the normal evidence for that operation is supplied independently.

### Requirement: Optional external diagnostics bridge
r[molten.node_runtime.external_diagnostics_bridge] Molten MAY support an explicit iroh-services-style external diagnostics bridge for pushing metrics or allowing remote diagnostic requests, but it MUST require operator configuration, redacted API-secret provenance, allowed capability scope, target service refs, and policy evidence.

#### Scenario: External bridge is disabled by default
- GIVEN no operator-approved external diagnostics profile is configured
- WHEN Molten starts a node
- THEN it does not push metrics to an external service and does not grant remote diagnostics capability.

#### Scenario: Remote diagnostics capability is scoped
- GIVEN an operator enables an external diagnostics bridge
- WHEN Molten grants remote diagnostic capability
- THEN the bridge receipt binds the target service identity, capability scope, redaction policy, upload or request mode, and expiry or revocation evidence
- AND inbound remote diagnostics requests still pass through normal protocol/router admission before any report is served.

### Requirement: Node live workflow lifecycle is ordered
r[molten.node_live_workflow_state_proof.ordered_lifecycle] Molten MUST prove that node live workflow bundle evidence advances only in the order bundle export, verify or gate, apply, optional send, receiver ingress or queue evidence, reconcile, ack, and import or protocol gate.

#### Scenario: Out-of-order apply denies
- GIVEN a live workflow bundle apply receipt without a matching passing bundle gate receipt
- WHEN Molten evaluates the workflow as enqueue or dispatch evidence
- THEN the workflow decision is `deny`
- AND no ingress, queue, dispatch, or import side effect is admitted from that apply receipt.

### Requirement: Node live workflow evidence binds operation identity
r[molten.node_live_workflow_state_proof.operation_binding] Molten MUST prove that live workflow reconcile, ack, import, and protocol-gate evidence bind the same bundle ref, request ref, operation ref, envelope ref, and expected receiver evidence before accepting a completed workflow.

#### Scenario: Ack for wrong operation denies
- GIVEN a passing reconcile receipt for one operation ref
- WHEN an ack bundle carries a different operation ref or request ref
- THEN the protocol gate or import receipt decision is `deny`
- AND diagnostics identify the mismatched workflow binding.

### Requirement: Live transport remains non-authorizing
r[molten.node_live_workflow_state_proof.transport_evidence_only] Molten MUST prove that live transport, neighbor, listener, send, and receive receipts do not replace peer admission, authority grant, provenance, policy, resource, source-gate, or operation evidence.

#### Scenario: Transport-only evidence cannot enqueue
- GIVEN a live send receipt and no imported peer admission or authority grant evidence
- WHEN node control evaluates ingress admission
- THEN the request is denied before enqueue
- AND diagnostics state which non-transport evidence is missing.

### Requirement: Node state stores peer session read model
r[molten.peer_session.node_state_table] Molten MUST store a bounded node-local peer read model that indexes peer sessions by peer id, node id, profile ref, ticket ref, admission ref, and admitted scope while preserving canonical receipts as the authority source.

#### Scenario: Status reads peer table without granting trust
- GIVEN a node state root contains a peer session read-model entry
- WHEN an operator runs peer status
- THEN the status output reports lifecycle state, refs, scopes, freshness, and diagnostics
- AND the read-model entry alone cannot satisfy authority, policy, provenance, source-gate, resource, retention, or execution gates.

### Requirement: Static peer config is contract-bound
r[molten.peer_session.nickel_config] Molten MUST validate static peer profile configuration through typed Nickel contracts before exporting runtime-consumed peer config, and runtime node operations MUST consume checked exports rather than evaluating Nickel live.

#### Scenario: Invalid peer config fails before runtime
- GIVEN a static peer profile uses an unsupported transport, malformed evidence ref, unsafe endpoint pattern, or contradictory resource bound
- WHEN the profile is exported through Nickel
- THEN export fails before node startup or peer connection can rely on that profile.

### Requirement: Live tickets bind into peer sessions
r[molten.peer_session.live_ticket_session_binding] Molten MUST bind existing live tickets, peer admissions, and imported authority grants into peer session records without changing their canonical receipt semantics or making imports authoritative by themselves.

#### Scenario: Ticket import updates session readback only
- GIVEN a sender imports a receiver live ticket and matching peer admission receipt
- WHEN the peer session read model updates
- THEN the session records the ticket and admission refs as bootstrap evidence
- AND operation authority remains absent until a matching authority grant is imported.

### Requirement: Peer lifecycle CLI wraps existing gates
r[molten.peer_session.peer_cli] Molten SHOULD expose peer invite, connect, status, revoke, and diagnose commands as thin shells over canonical peer-session, ticket, admission, authority, policy, resource, and replay evidence.

#### Scenario: Diagnose reports next missing live-send step
- GIVEN a sender has a receiver ticket but no matching peer admission in its state root
- WHEN `molten peer diagnose` runs for that peer and scope
- THEN it reports the missing peer admission import
- AND it does not attempt a live send or mutate authority state.

### Requirement: Peer lifecycle validation is reproducible
r[molten.peer_session.validation] Molten SHOULD validate peer-session lifecycle work with focused positive and negative tests, Nickel peer config fixtures, formatting, peer-related cargo tests, and Cairn validation before the change is archived.

#### Scenario: Validation catches stale ticket regression
- GIVEN a regression accepts an expired or wrong-topic peer ticket as an admitted session
- WHEN focused peer-session validation runs
- THEN the negative fixture fails
- AND the change cannot be marked complete until the denial is restored.

### Requirement: Node-control supports generic peer handoff bundles
r[molten.peer_handoff.node_control_compat] Molten MUST preserve existing node-control live workflow receipt semantics while allowing node-control handoff export, verify, gate, import, and apply flows to use the generic peer handoff bundle model.

#### Scenario: Existing node-control bundle remains readable
- GIVEN a node-control live workflow bundle produced before the generic handoff model
- WHEN the compatibility parser reads the bundle
- THEN it can summarize the existing ticket, peer admission, authority grant, and receipt refs
- AND it does not reinterpret the bundle as authority beyond the embedded grant artifacts.

### Requirement: Handoff validation covers subsystem consumers
r[molten.peer_handoff.consumer_scope_binding] Molten MUST require node-control, remote dataspace, job worker, retention clearance, and remote artifact sync consumers to check the handoff scope before using imported peer evidence.

#### Scenario: Job handoff cannot satisfy node-control scope
- GIVEN a peer handoff bundle is scoped to a job worker pool
- WHEN a node-control live send tries to use that handoff as peer bootstrap evidence
- THEN node-control preflight denies the scope mismatch
- AND diagnostics name the expected node-control topic or operation scope.

### Requirement: Peer handoff validation is reproducible
r[molten.peer_handoff.validation] Molten SHOULD validate generic handoff work with focused handoff tests, node-control bundle compatibility tests, remote dataspace/job/retention/sync consumer tests, formatting, and Cairn validation before the change is archived.

#### Scenario: Consumer regression is caught
- GIVEN a subsystem consumer accepts a handoff bundle whose declared scope does not match the operation
- WHEN focused peer handoff validation runs
- THEN the negative consumer fixture fails
- AND the change cannot complete until the scope denial is restored.

### Requirement: Node runtime applies promotions only after gates pass
r[molten.peer_promotion.node_apply_boundary] Molten MUST update node-local peer session read models for promoted or demoted capabilities only after promotion apply or demotion receipts pass all authority, policy, resource, expiry, revocation, and approval gates.

#### Scenario: Failed promotion leaves read model unchanged
- GIVEN a peer promotion apply operation fails because the issuer is revoked
- WHEN the node runtime updates peer-session state
- THEN the prior session capabilities remain unchanged
- AND the failed promotion receipt is stored only as denial evidence.

### Requirement: Promotion apply does not perform subsystem side effects
r[molten.peer_promotion.apply_no_subsystem_side_effects] Molten SHOULD limit promotion apply to capability/session state changes and MUST NOT automatically execute node-control operations, job work, retention actions, sync imports, relay publication, or Raft membership changes.

#### Scenario: Publisher promotion does not publish
- GIVEN a peer is promoted from subscriber to scoped publisher
- WHEN promotion apply passes
- THEN the session records the new publish capability
- AND no message is published until a separate publish operation passes its own gates.

### Requirement: Node daemon responsibilities are semantically separated
r[molten.node_runtime.modularity.daemon_modules] Node daemon implementation SHOULD be organized into semantic ownership boundaries for config, identity, locks, inbox, ingress, dispatch, supervision, live workflow, receipts, and shell orchestration.

#### Scenario: Node module name reveals responsibility
- GIVEN a node daemon behavior is moved during modularity cleanup
- WHEN reviewers inspect the new file or module name
- THEN the name identifies the daemon responsibility rather than an ordinal shard sequence

#### Scenario: Existing node API remains stable
- GIVEN a documented `molten node` command or compatibility module path
- WHEN daemon internals are reorganized
- THEN the command or path remains available unless a separate compatibility change owns the break

### Requirement: Node daemon decisions have pure cores
r[molten.node_runtime.modularity.pure_daemon_core] Node daemon decisions for locks, duplicate requests, ingress admission, dispatch routing, supervisor policy, and workflow gating SHOULD be expressed as pure functions over typed inputs.

#### Scenario: Duplicate enqueue decision is pure
- GIVEN an existing request ref and a new request input represented in memory
- WHEN the duplicate decision core evaluates the input
- THEN it returns enqueue, replay-prior, deny, or diagnostic output without reading the state root or writing queue files

#### Scenario: Supervisor denial is pure
- GIVEN restart history and supervisor policy represented in memory
- WHEN the supervisor decision core evaluates a failed service
- THEN it returns restart or deny output without reading clocks, sockets, files, or live transport state

### Requirement: Node shell owns IO and transport
r[molten.node_runtime.modularity.shell_boundary] Node daemon shells MUST own state-root filesystem IO, service locks, control sockets, live Iroh sessions, process lifecycle, and receipt file writes.

#### Scenario: Shell executes admitted node plan
- GIVEN a pure node decision returns an admitted plan
- WHEN the node shell executes it
- THEN filesystem, lock, or transport effects occur through the shell and canonical node evidence is recorded

#### Scenario: Denied node plan does not perform IO
- GIVEN a pure node decision returns deny
- WHEN the node shell receives the decision
- THEN no queue write, lock mutation, live send, or operation side effect is performed

### Requirement: Node daemon extraction has positive and negative tests
r[molten.node_runtime.modularity.tests] Node daemon boundary refactors SHOULD include positive and negative tests for the extracted decision or shell boundary.

#### Scenario: Node boundary tests cover pass and deny
- GIVEN a node daemon decision boundary is extracted
- WHEN reviewers inspect the tests
- THEN at least one admitted path and one denied or malformed path are covered

### Requirement: Iroh ALPN registry entries are canonical
r[molten.node_runtime.iroh_alpn_registry_model] Molten MUST define canonical registry entries for Molten-owned Iroh ALPN protocols, binding symbolic name, ALPN bytes or deterministic string encoding, owner namespace, handler profile, supported schema/profile versions, lifecycle state, limit profile refs, required admission evidence, and receipt schema refs.

#### Scenario: Registry entry binds handler ownership
- GIVEN a Molten-owned Iroh protocol handler is proposed for installation
- WHEN the router admission core evaluates it
- THEN the handler descriptor references a canonical registry entry with matching symbolic name, ALPN, owner namespace, and handler profile.

#### Scenario: Deprecated entry is explicit
- GIVEN an ALPN exists only for migration or diagnostic compatibility
- WHEN it appears in the registry
- THEN the registry marks its lifecycle state explicitly and prevents it from becoming a production default without a reviewed registry update.

### Requirement: ALPN registry validation fails closed
r[molten.node_runtime.iroh_alpn_registry_validation] Molten MUST validate ALPN registry entries for deterministic encoding, non-empty owner namespace, unique ALPN bytes within the active registry, supported lifecycle state, and compatible limit/profile refs before live router mutation.

#### Scenario: Duplicate ALPN denies registry admission
- GIVEN two active registry entries declare the same ALPN bytes
- WHEN registry validation runs
- THEN validation denies the registry and no live handler map is updated from it.

#### Scenario: Malformed ALPN denies before advertising
- GIVEN a proposed handler names an empty, malformed, or unsupported ALPN encoding
- WHEN router admission evaluates the handler
- THEN the admission denies before the endpoint advertises the ALPN.

### Requirement: Router handler ownership is enforced
r[molten.node_runtime.iroh_alpn_handler_ownership] Runtime router install, replacement, removal, and shutdown operations MUST check registry ownership, current generation, handler profile compatibility, and lifecycle state before mutating the live advertised ALPN map.

#### Scenario: Wrong owner cannot replace handler
- GIVEN an active handler owned by one protocol namespace
- WHEN a replacement request from a different owner namespace targets the same ALPN
- THEN router admission denies with an owner-mismatch diagnostic and the existing generation remains active.

#### Scenario: Replacement records registry generation
- GIVEN a valid replacement for an active handler
- WHEN router admission passes
- THEN the router receipt binds the registry entry ref, prior generation, new generation, handler profile, and prior-handler shutdown evidence.

### Requirement: ALPN routing evidence is not operation authority
r[molten.node_runtime.iroh_alpn_non_authority] Molten MUST NOT treat ALPN negotiation, endpoint identity, router registry entries, router receipts, stream sessions, or framed-envelope receipts as authority for node-control, protocol-session, artifact import, retention, execution, provenance, resource, policy, or source-gate side effects.

#### Scenario: Valid ALPN without authority denies operation
- GIVEN a peer opens a connection using a registered ALPN and sends a well-formed frame
- WHEN the downstream operation lacks matching authority, policy, resource, or subsystem evidence
- THEN operation admission denies before side effects and diagnostics state that ALPN routing is not authority.

#### Scenario: Unsupported ALPN denies before frame delivery
- GIVEN an incoming connection offers an ALPN that is absent, removed, or unsupported by the current registry generation
- WHEN the router evaluates the connection
- THEN the router emits deny evidence before delivering any frame to node-control, protocol-session, plugin, dataspace, or service state.

### Requirement: Deployment profiles declare review tier

r[molten.prod_ops.release_profile.tiers] Molten SHOULD distinguish development, pilot, and release deployment profile tiers so fixture evidence, pilot evidence, and release-candidate evidence carry different validation expectations.

#### Scenario: Pilot profile remains pilot scoped

- GIVEN a deployment profile intended for local or pilot review
- WHEN the profile is exported or bound into production-readiness receipts
- THEN the profile tier records that it is development or pilot scoped
- AND the profile cannot be presented as release-tier evidence without passing release-tier validation.

#### Scenario: Release profile requires release validation

- GIVEN an operator prepares a release-candidate deployment profile
- WHEN the profile is exported for release review
- THEN release-tier validation requires the stricter evidence fields and placeholder-ref denial rules.

### Requirement: Release profiles reject placeholder refs

r[molten.prod_ops.release_profile.no_placeholder_refs] Release-tier deployment profiles MUST reject all-zero BLAKE3 refs, repeated-character dummy refs, declared fixture placeholders, and other configured placeholder evidence refs before they can be bound into release-readiness receipts.

#### Scenario: Non-placeholder release refs pass

- GIVEN a release-tier profile whose source-gate, policy, Octet, Cairn, stack-provenance, and production-profile refs are reviewed non-placeholder BLAKE3 refs
- WHEN release profile validation runs
- THEN validation passes and records the accepted refs as release-review inputs.

#### Scenario: Zero source-gate ref denies release profile

- GIVEN a release-tier profile whose source-gate input is `blake3:0000000000000000000000000000000000000000000000000000000000000000`
- WHEN release profile validation runs
- THEN validation denies the profile before release-readiness evidence can treat it as current.

#### Scenario: Dummy repeated ref denies release profile

- GIVEN a release-tier profile whose evidence input uses an obvious repeated-character fixture ref
- WHEN release profile validation runs
- THEN validation denies with a placeholder-ref diagnostic naming the affected field.

### Requirement: Release profiles bind freshness evidence

r[molten.prod_ops.release_profile.freshness] Release-tier deployment profile validation MUST bind current source-gate, policy, Octet, Cairn, generated-export, and profile content refs and MUST deny stale or missing refs before promotion evidence treats the profile as release-ready.

#### Scenario: Fresh release profile validates

- GIVEN a release-tier profile whose generated export, source-gate evidence, Octet evidence, Cairn validation evidence, and policy refs match the source candidate under review
- WHEN release profile freshness validation runs
- THEN validation passes and records the matched refs.

#### Scenario: Stale generated profile denies

- GIVEN a release-tier profile export whose generated JSON ref no longer matches the reviewed Nickel source
- WHEN release profile freshness validation runs
- THEN validation denies with expected and actual profile refs.

### Requirement: Release profile behavior has fixtures

r[molten.prod_ops.release_profile.fixtures] Release profile hardening SHOULD include positive fixtures for development, pilot, and release tiers plus negative fixtures for zero refs, dummy refs, stale refs, missing required evidence, optional stack provenance in release mode, and unsupported tier values.

#### Scenario: Negative fixture denies placeholder release evidence

- GIVEN a negative release profile fixture containing placeholder source-gate or policy refs
- WHEN fixture validation runs
- THEN the fixture fails before generated release evidence can be refreshed.

#### Scenario: Positive pilot fixture remains diagnostic scoped

- GIVEN a pilot profile fixture with local or synthetic evidence refs
- WHEN fixture validation runs
- THEN it may pass as pilot evidence
- AND it is not counted as release-tier evidence.

### Requirement: Node lifecycle accepts profile-backed configuration
r[molten.node_runtime.profile_backed_config] Molten MUST allow node lifecycle commands to construct durable `node-config-v1` from a checked exported node profile or profile evidence ref. Runtime startup MUST consume checked profile artifacts and MUST NOT evaluate Nickel during node init, run, serve, or startup receipt generation.

#### Scenario: Profile-backed node config is written
- GIVEN a checked exported node profile with supported metadata, state layout, required adapter profiles, source-gate refs, policy refs, capability refs, resource refs, and effect-profile refs
- WHEN `molten node init` is run with that profile
- THEN the durable node config records the profile-selected values
- AND the config ref is derived from canonical Preserves bytes.

#### Scenario: Runtime Nickel evaluation is denied
- GIVEN a node startup command receives a Nickel source file instead of a checked exported profile artifact or profile ref
- WHEN startup validation runs
- THEN startup denies before side effects that depend on the profile
- AND diagnostics state that runtime startup consumes checked exports rather than evaluating Nickel.

### Requirement: Startup receipts bind effective profile evidence
r[molten.node_runtime.profile_startup_receipt_binding] Node startup receipts MUST bind the effective profile ref, profile schema metadata, selected adapter profile refs, state-layout ref, source-gate refs, policy refs, capability refs, resource refs, effect-profile refs, and profile-resolution diagnostics.

#### Scenario: Startup binds matching profile metadata
- GIVEN a node starts from a checked exported profile whose content ref matches the supplied profile ref
- WHEN startup emits a receipt
- THEN the receipt records the matching profile ref, schema id, schema version, source language, profile identity, selected adapters, and evidence refs
- AND downstream review can distinguish profile evidence from authority, source-gate, adapter, and transport receipts.

#### Scenario: Tampered profile ref denies startup
- GIVEN a supplied profile ref does not match the checked profile artifact bytes
- WHEN node startup validates the profile evidence
- THEN startup denies before adapter start receipts are accepted
- AND diagnostics name the profile-ref mismatch.

### Requirement: Profile and CLI overrides are explicit
r[molten.node_runtime.profile_override_policy] Molten MUST apply profile-to-CLI override rules through a deterministic core that records every accepted override and denies overrides that weaken required profile invariants, omit required evidence, or change a release-tier profile into a local fixture configuration.

#### Scenario: Allowed local override is recorded
- GIVEN a development profile marks a non-security path field as overrideable
- WHEN an operator supplies the corresponding CLI override
- THEN profile resolution records the override source and effective value in diagnostics or receipts
- AND the canonical node config uses the effective value.

#### Scenario: Production invariant override denies
- GIVEN a production or release-tier profile requires a source-gate ref, required adapter, or resource-limit invariant
- WHEN an operator supplies a CLI override that removes or weakens that invariant
- THEN profile resolution denies before writing durable node config
- AND diagnostics identify the denied override and profile rule.

### Requirement: Local default node config is fixture-scoped
r[molten.node_runtime.local_default_config_caveat] Molten MAY retain current no-profile local node defaults for development fixtures, but startup receipts emitted from those defaults MUST carry a local-fixture caveat and MUST NOT satisfy production or release profile evidence requirements.

#### Scenario: No-profile init remains usable for fixtures
- GIVEN an operator runs `molten node init` without a profile in a local test state root
- WHEN durable config and startup receipts are emitted
- THEN the command preserves the local development behavior
- AND the receipts record that the config came from local fixture defaults.

#### Scenario: Fixture config cannot pass release profile gate
- GIVEN a release gate requires profile-backed configuration evidence
- WHEN the gate receives only a startup receipt produced from local defaults
- THEN the gate denies or records the config as insufficient for release evidence.

### Requirement: Node-control service lifecycle is explicit
r[molten.node_runtime.service_fsm_model] Molten MUST model the node-control service lifecycle as a reviewed finite state machine over service state, event, startup evidence, service-lock evidence, supervisor-policy facts, heartbeat/tick facts, shutdown facts, and explicit shell intents.

#### Scenario: Service reaches serving through reviewed transitions
- GIVEN node initialization and startup evidence have passed and no active service lock exists
- WHEN the service acquires a lock and evaluates the serve event
- THEN Molten emits passing service lifecycle evidence
- AND the next state is serving with startup and service-lock refs bound.

#### Scenario: Serve cannot start from missing startup evidence
- GIVEN a state root has no current startup receipt
- WHEN the service serve event is evaluated
- THEN the transition decision is deny
- AND no service lock, live listener, run-loop, or shutdown side effect is authorized.

### Requirement: Node-control service receipts bind lifecycle state
r[molten.node_runtime.service_fsm_receipts] Node-control service-lock, heartbeat, supervisor, service-run, live-listener, run-loop, health, and shutdown receipts MUST bind the relevant service FSM prior state, event, next state or preserved state, startup ref, service-lock ref when present, supervisor policy ref when present, decision, diagnostics, and evidence-only caveats.

#### Scenario: Denied duplicate runner preserves service state
- GIVEN a service lock is already active for a startup receipt
- WHEN a second runner attempts to acquire service ownership
- THEN Molten emits a deny receipt bound to the preserved service state
- AND the existing service lock remains authoritative only as lifecycle evidence, not operation authority.

### Requirement: Stale lock recovery and restart bounds are transitions
r[molten.node_runtime.service_fsm_lock_recovery] Molten MUST model stale-lock recovery, duplicate-runner denial, heartbeat timeout, restart admission, restart denial, shutdown requested, shutdown drain, and drain completion as explicit node-control service FSM transitions that fail closed when policy or evidence is missing.

#### Scenario: Stale lock without recovery policy denies
- GIVEN a stale service lock is observed and no current supervisor policy permits stale-lock recovery
- WHEN recovery is evaluated
- THEN the transition decision is deny
- AND no lock replacement or service restart side effect occurs.

#### Scenario: Shutdown drain completes before stop
- GIVEN a running node-control service receives a shutdown request and pending inbox work is within the reviewed drain bound
- WHEN drain completion is evaluated
- THEN Molten emits passing service lifecycle evidence
- AND the next state permits stop and lock release.

### Requirement: Node-control service FSM tests cover positive and negative paths
r[molten.node_runtime.service_fsm_tests] Molten SHOULD test the node-control service FSM with positive startup, serve, heartbeat, drain, shutdown, and stop traces, and negative traces for missing startup evidence, duplicate runner, stale lock without recovery, stale startup binding, heartbeat timeout, restart bound exhaustion, and shutdown drain over limit.

#### Scenario: Generated service trace rejects illegal restart
- GIVEN a generated node-control service trace exceeds the configured restart bound
- WHEN the supervisor restart event is evaluated
- THEN the transition decision is deny
- AND the state remains unchanged until a reviewed recovery path is supplied.

### Requirement: HTTP3-over-Iroh readback adapter is optional and non-authority
r[molten.node_runtime.http3_iroh_readback_adapter] Molten MAY provide an optional HTTP3-over-Iroh adapter for operator readback, but the adapter MUST translate requests into canonical Molten gateway read, range, or index requests and MUST treat the gateway receipt as the normative evidence.

#### Scenario: HTTP readback delegates to canonical gateway request
- GIVEN an admitted HTTP3-over-Iroh session asks to read a visible artifact range
- WHEN the adapter handles the request
- THEN it builds the same canonical operator gateway range request used by non-HTTP paths
- AND the response is emitted only after the gateway range receipt passes.

#### Scenario: HTTP session does not grant authority
- GIVEN a remote client has an HTTP3-over-Iroh connection, route, header, or endpoint identity
- WHEN the client requests protected, hidden, destructive, or authority-scoped content
- THEN the adapter requires the normal capability, policy, resource, retention, redaction, and gateway visibility evidence
- AND the HTTP session evidence alone is insufficient.

#### Scenario: HTTP adapter cannot bypass Preserves identity
- GIVEN an HTTP route maps to a Molten artifact or chunk manifest
- WHEN Molten evaluates readback
- THEN canonical Preserves or chunk manifest refs remain the identity boundary
- AND HTTP paths, status codes, headers, and rendered MIME metadata are non-normative views.

### Requirement: HTTP3-over-Iroh adapter tests cover pass and denial
r[molten.node_runtime.http3_iroh_readback_tests] Molten SHOULD cover the optional HTTP3-over-Iroh adapter with positive tests for admitted read-only gateway requests and negative tests for unauthorized routes, hidden refs, unsupported methods, oversized requests, malformed ranges, and attempts to use HTTP transport as authority.

#### Scenario: Unauthorized HTTP route denies before read
- GIVEN an HTTP3-over-Iroh request targets a protected artifact without reveal or visibility evidence
- WHEN the adapter translates the request
- THEN the canonical gateway gate denies
- AND the adapter returns only diagnostic readback without exposing protected bytes.

#### Scenario: Unsupported method cannot mutate state
- GIVEN an HTTP3-over-Iroh request uses a method intended to pin, delete, install, execute, or mutate policy
- WHEN the optional readback adapter validates the request
- THEN it denies before invoking any destructive or privileged subsystem
- AND emits diagnostics that the adapter is read-only.
