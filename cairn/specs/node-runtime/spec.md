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
