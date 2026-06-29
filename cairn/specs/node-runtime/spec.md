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
