## Context

A system extension needs to own wire protocols and long-lived sessions without owning raw node sockets or depending directly on Iroh APIs. The transport boundary also has to survive substitution by the deterministic simulator so protocol state machines can be tested without rewritten networking logic.

## Decisions

### 1. The transport port is event-oriented and adapter-neutral

**Choice:** Define canonical commands and events for protocol registration, dialing, accepting, session state, bidirectional and unidirectional streams, framed messages, optional datagrams, flow-control credits, cancellation, close, and failures. Adapter handles are opaque canonical ids scoped to service generation.

**Rationale:** Extension protocol cores need deterministic events, not implementation-specific connection objects or async runtimes.

### 2. Protocol ownership is capability-gated and generation-fenced

**Choice:** A system extension registers an admitted protocol identity and version under its active generation. Conflicting registration denies. Drain or generation replacement stops new accepts before existing sessions close or reach their bounded grace policy.

**Rationale:** Protocol dispatch must never race between stale and replacement service instances.

### 3. Framing and bounds are part of the contract

**Choice:** Every message or stream profile declares framing, maximum frame and session budgets, queue/credit behavior, timeout policy, and malformed-input handling. The host enforces outer bounds before delivering events to extension code.

**Rationale:** Raw unbounded byte streams would move denial-of-service and parser ambiguity into each extension.

### 4. Transport identity is not authority

**Choice:** Transport events expose authenticated adapter identity refs and connection metadata, but membership, application principals, capabilities, trust, and service authorization require separate policy evaluation.

**Rationale:** Cryptographic peer connectivity must not become ambient cluster or application authority.

### 5. Live and simulation adapters share observable semantics

**Choice:** Both adapters consume the same commands and emit the same event algebra. Profile metadata may expose adapter-specific capabilities and non-claims, but protocol cores cannot branch on hidden live implementation state.

**Rationale:** This makes deterministic simulation a genuine execution profile rather than a separate mock implementation.

### 6. Delivery guarantees are minimal and explicit

**Choice:** Base transport provides bounded session and stream events with declared ordering and close semantics. It does not claim exact-once delivery, durable delivery, transactional messaging, global ordering, or automatic retries. Extensions or higher ports may implement those semantics.

**Rationale:** Stronger claims require storage and protocol state that do not belong in the universal transport layer.

## Functional core / imperative shell split

- Pure core: protocol registry validation, generation routing, command/event transition checks, frame validation, credit accounting, timeout classification, retry decisions when selected, and canonical evidence payloads.
- Shell: bind Iroh protocols, dial or accept peers, move bytes, translate adapter events, enforce cancellation, and emit bounded lifecycle evidence.

## Risks / Trade-offs

- Lowest-common-denominator events could hide useful transport features. Expose optional versioned capabilities rather than leaking adapter handles.
- Stream flow control can deadlock if credits and cancellation are underspecified. Define terminal events and test every blocked state.
- ALPN conflicts or stale listeners can route traffic incorrectly. Registration, activation, drain, and cleanup must be atomic at the registry boundary.
