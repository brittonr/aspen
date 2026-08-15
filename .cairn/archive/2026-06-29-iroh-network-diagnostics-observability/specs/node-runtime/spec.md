## ADDED Requirements

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
