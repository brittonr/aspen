# Component RPC Pilot Specification

## Purpose

Evaluate a pinned wRPC adapter for WIT-native remote component calls without replacing Molten's Iroh transport, Preserves envelopes, authority gates, delivery semantics, or canonical evidence.

## Requirements

### Requirement: Component RPC is an explicit pilot profile

r[molten.component_rpc.profile] Molten MUST expose wRPC only through a versioned experimental profile that binds the wRPC/toolchain cohort, WIT package/world/function set, transport port, resource envelope, supported call shape, and non-claims.

#### Scenario: Pilot profile is explicitly selected
- GIVEN a caller selects the supported profile and every cohort fact is present
- WHEN component-RPC admission runs
- THEN Molten MAY construct a pilot call plan under that exact profile identity.

#### Scenario: Caller omits or drifts the profile
- GIVEN the wRPC version, WIT world, transport profile, or supported call shape differs from the admitted cohort
- WHEN admission runs
- THEN Molten MUST deny before opening a transport session.

### Requirement: Wire values and Preserves envelopes are both retained

r[molten.component_rpc.bridge] The pilot MUST use a declared mapping between WIT component-value wire bytes and canonical Preserves request/result envelopes, MUST retain both identities, and MUST NOT claim general semantic equivalence between them.

#### Scenario: Declared unary mapping succeeds
- GIVEN a supported WIT function and canonical Preserves request satisfy the mapping profile
- WHEN the bridge encodes and later decodes the call
- THEN the transcript MUST bind the WIT function, wire argument/result BLAKE3, and Preserves request/result refs.

#### Scenario: Mapping is malformed or stale
- GIVEN a field, variant, world, function, schema, or mapping-profile identity does not match
- WHEN bridge validation runs
- THEN Molten MUST deny without sending the call or publishing a result.

### Requirement: Transport remains an admitted fabric mechanism

r[molten.component_rpc.transport] The wRPC adapter MUST use a separately admitted fabric transport session and MUST NOT replace or silently reroute existing remote dataspace, node-control, job, service, or Iroh protocol paths.

#### Scenario: Pilot transport is available
- GIVEN a matching admitted fabric port and peer session exist
- WHEN the call plan reaches the shell
- THEN the adapter MAY carry the bounded wRPC call over that session and MUST retain its transport evidence ref.

### Requirement: Remote calls require full authority admission

r[molten.component_rpc.authority] Every pilot call MUST bind peer/session admission, operation identity, caller/callee component refs, WIT world/function scope, policy, Basalt/UCAN authority, resources, delivery, retry/dedup, and replay context before transport effects.

#### Scenario: Call has complete authority
- GIVEN every required admission record is current and scope-matching
- WHEN call admission runs
- THEN Molten MAY authorize only the declared invocation.

#### Scenario: WIT-compatible peer lacks authority
- GIVEN the peer is connected and implements the world but lacks a current scope-matching authority or resource record
- WHEN call admission runs
- THEN Molten MUST deny before sending bytes and MUST NOT treat connectivity as authority.

### Requirement: First pilot excludes WIT streams and futures

r[molten.component_rpc.async_scope] The first component-RPC profile MUST support only bounded unary request/result calls and MUST deny WIT streams, futures, implicit cancellation, and transport extensions unless a later versioned profile defines their lifecycle and replay semantics.

#### Scenario: Component requests stream or future value
- GIVEN a call type or negotiated capability includes a stream, future, or unsupported async extension
- WHEN profile validation runs
- THEN Molten MUST reject it as unsupported before invocation.

### Requirement: Call transcripts are replayable evidence

r[molten.component_rpc.transcript] The pilot MUST emit canonical transcripts binding invocation, component/WIT, wire, Preserves, authority/resource, transport-session, delivery, retry/dedup, and terminal result identities, and replay MUST consume recorded effects rather than live transport timing.

#### Scenario: Recorded call is replayed
- GIVEN a complete passing call transcript and identical component/profile inputs
- WHEN replay runs without live network effects
- THEN the canonical bridge decisions, request/result identities, delivery class, and terminal result MUST match.

#### Scenario: Transcript is reordered or cross-call
- GIVEN transcript entries have stale invocation ids, missing parent links, reordered terminal events, or bytes from another call
- WHEN transcript validation runs
- THEN validation MUST fail closed.

### Requirement: Component telemetry is diagnostic only

r[molten.component_rpc.telemetry] Optional OpenTelemetry-WASI export MUST be separately admitted, resource-bounded, redacted, payload-minimized, and labeled diagnostic-only; telemetry MUST NOT replace canonical receipts or grant authority, provenance, policy, or release eligibility.

#### Scenario: Redacted telemetry links to receipt
- GIVEN telemetry export is admitted and contains no denied payload fields
- WHEN a span is emitted
- THEN it MAY reference the canonical receipt id but MUST remain auxiliary diagnostic data.

#### Scenario: Telemetry contains sensitive payload
- GIVEN a span or attribute contains a secret, credential, private payload, raw canonical message, or unapproved high-cardinality field
- WHEN telemetry redaction runs
- THEN export MUST deny or replace it according to the declared redaction plan.

### Requirement: Pilot decisions have a functional core

r[molten.component_rpc.functional_core] Profile admission, WIT/Preserves mapping, invocation identity, authority/resource planning, unsupported-async rejection, transcript validation, replay comparison, telemetry redaction planning, and pilot classification MUST be pure deterministic logic.

#### Scenario: Identical facts produce identical call plan
- GIVEN identical profile, mapping, peer, authority, resource, request, and transport facts
- WHEN the core plans a call
- THEN it MUST return the same plan or blockers without filesystem, network, process, clock, Wasmtime, wRPC, Iroh, telemetry, or output effects.

### Requirement: Pilot non-claims and validation remain explicit

r[molten.component_rpc.nonclaims] The pilot MUST state that successful calls do not prove protocol stability, semantic equivalence, transport security, production readiness, component correctness, or replacement eligibility for existing Molten protocols.

#### Scenario: Pilot requests production claim
- GIVEN pilot evidence claims general RPC correctness or production readiness from loopback fixtures
- WHEN evidence validation runs
- THEN Molten MUST reject the overclaim.

### Requirement: Pilot includes positive and negative evidence

r[molten.component_rpc.validation] The pilot MUST include positive unary loopback/replay cases and negative cohort, mapping, authority, resource, transcript, unsupported-async, telemetry-redaction, and live-timing cases plus focused lifecycle gates.

#### Scenario: Pilot is reviewed for graduation
- GIVEN implementation and evidence are proposed for promotion
- WHEN the graduation gate runs
- THEN every declared positive and negative case MUST pass and the overlap decision with existing Iroh protocols MUST be recorded.
