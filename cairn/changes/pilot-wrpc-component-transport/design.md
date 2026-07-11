## Context

wRPC encodes WIT component values on the wire and can expose runtime imports/exports through transport adapters. Molten's current network and evidence spine encodes canonical Preserves envelopes over Iroh and separates transport identity from peer admission, capability authority, policy, resources, provenance, and delivery/replay decisions.

The pilot must therefore be an adapter between two explicit representations, not a new canonical transport or implicit conversion layer.

## Decisions

### 1. The pilot is an optional fabric adapter

**Choice:** Implement wRPC behind a separately admitted component-RPC profile and fabric transport port. No existing remote dataspace, node-control, job, or service protocol is rerouted through it.

**Rationale:** A pilot should test interoperability without changing the stack's canonical transport or evidence path.

### 2. WIT wire values and Preserves envelopes keep separate identities

**Choice:** The bridge core maps a narrowly declared WIT call DTO to one canonical Preserves request envelope and maps the admitted result back. Receipts retain BLAKE3 identities for WIT source/world/function, encoded wRPC arguments/results, Preserves request/result, and the mapping profile. A successful mapping does not claim general semantic equivalence between the encodings.

**Rationale:** Losing either byte representation would make replay and interoperability failures impossible to localize.

### 3. Authority is resolved before transport effects

**Choice:** Before opening or using a wRPC call, Molten validates peer/session admission, operation identity, component/world/function scope, policy, Basalt/UCAN authority, resource budget, and delivery/replay context. Transport connectivity and WIT compatibility never satisfy those gates.

**Rationale:** Remote invocation is an effect and must follow the same authority separation as existing Iroh paths.

### 4. The first profile excludes unstable async surface

**Choice:** Phase one supports bounded unary request/result calls and explicitly denies WIT stream/future values, cancellation semantics, and transport-specific extensions unless a later profile versions their lifecycle and replay behavior.

**Rationale:** wRPC supports evolving unreleased WIT features whose cancellation and replay semantics should not become implicit product contracts.

### 5. Transcripts are canonical bridge evidence

**Choice:** Emit a call transcript that binds invocation identity, caller/callee component refs, WIT function, request/result wire digests, Preserves envelope refs, authority/resource refs, transport session, delivery outcome, retry/dedup identity, and terminal result. Replay consumes recorded transcript effects rather than live transport timing.

**Rationale:** A socket-level success is weaker than evidence that the intended invocation and result crossed all boundaries.

### 6. OpenTelemetry-WASI remains diagnostic

**Choice:** Optional component telemetry is collected through an explicitly admitted interface, redacted before export, bounded by resource policy, and labeled diagnostic-only. Trace/span IDs may link to a canonical receipt ref, but telemetry cannot replace or mutate that receipt and raw payloads are excluded by default.

**Rationale:** Telemetry is useful for operations but can be lossy, sampled, reordered, remote, or sensitive.

### 7. Pilot graduation has explicit exit criteria

**Choice:** Promotion requires stable pinned-cohort conformance, deterministic replay of recorded calls, negative authority/schema/resource coverage, bounded failure handling, and a written decision about overlap with existing Iroh protocols. Otherwise the adapter remains experimental or is removed.

## Functional core / imperative shell split

- **Pure core**: profile validation, WIT/Preserves mapping, invocation identity, authority/resource request construction, stream/future rejection, transcript validation, replay comparison, telemetry redaction plans, and pilot classification.
- **Imperative shell**: instantiate wRPC/Wasmtime adapters, open Iroh transport sessions, send/receive bytes, invoke admitted components, export telemetry, and persist receipts.

## Risks / Trade-offs

- Maintaining two wire representations increases complexity and payload overhead.
- wRPC/WIT async evolution may require cohort upgrades or make the pilot obsolete.
- Mapping bugs can produce well-typed but semantically wrong requests; positive and negative golden mappings are mandatory.
- Telemetry exporters can introduce network effects or sensitive data; telemetry is opt-in and independently admitted.

## Non-Goals

- No replacement of Iroh, Preserves, Molten delivery/idempotency, or existing protocols.
- No general-purpose RPC framework claim outside the pinned pilot world.
- No support for streams/futures in the first profile.
- No promotion of OpenTelemetry data into canonical evidence, authority, provenance, or release eligibility.
