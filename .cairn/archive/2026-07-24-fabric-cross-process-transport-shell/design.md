## Context

The accepted fabric transport core already owns canonical protocol registration, scoped transport ids, session and stream transitions, framing, credit, cancellation, delivery classification, identity separation, and bounded evidence. The deterministic adapter and same-process Iroh loopback implement useful conformance rails, but the live shell has no reusable listener or client lifecycle that separate supervised processes can bind.

The node runtime already has Iroh endpoint and ALPN routing patterns, while the receipt-first cluster harness already provides distinct child processes, run directories, bounded teardown, and offline evidence conventions. This change composes those existing boundaries instead of introducing a second transport contract or moving consensus semantics into transport.

## Decisions

### 1. Cross-process endpoint descriptors are canonical connectivity artifacts

**Choice:** Define a versioned endpoint descriptor that binds the exact transport profile, protocol and ALPN, endpoint public identity, owning extension and service, active generation, admitted locator candidates, locator-disclosure policy, framing/resource profile, validity cohort, and descriptor ref. The descriptor excludes private key material, bearer authority, raw runtime handles, ambient paths, and implicit trust. Import validates the exact expected profile, protocol, owner, generation, endpoint identity, and declared peer context before dialing. Default evidence and status bind locator refs and redacted classes rather than rendering raw direct or relay locators; explicit handoff artifacts disclose only the locators admitted by their profile.

**Rationale:** A separate process needs a portable handoff, but a locator or authenticated endpoint must not become membership, capability, or application authority.

### 2. Listener and client semantics stay in the pure transport core

**Choice:** Extend `molten-core` with deterministic endpoint admission, listener/session planning, generation and ownership checks, lifecycle transitions, terminal classification, resource accounting, and canonical receipt payload construction. Inputs are typed descriptors, commands, events, profile bounds, and observed outcomes. The core performs no endpoint creation, network I/O, clock reads, environment access, process inspection, or receipt persistence.

**Rationale:** Protocol and failure decisions must remain testable without Iroh, Tokio, sockets, clocks, or child processes.

### 3. The Iroh runtime is a thin capability-scoped shell

**Choice:** Add a shell that creates or opens an endpoint only from explicit admitted inputs, registers the exact ALPN, publishes the canonical public descriptor, accepts or dials bounded sessions, reads outer framing before bounded allocation, translates outcomes into canonical events, and delegates every semantic decision to the pure core. It cannot use ambient socket discovery, select an undeclared protocol, silently replace a profile, or expose Iroh endpoint/connection/stream handles to extension code.

**Rationale:** Iroh owns live mechanics; Aspen owns admission and observable semantics.

### 4. Long-lived lifecycle is supervised and generation-fenced

**Choice:** Listener start, ready, accept, drain, cancellation, close, cleanup, failure, and replacement are explicit states bound to one service generation. A descriptor cannot be published until endpoint setup, exact ALPN activation, registration ownership, and readiness all pass atomically. Drain stops new accepts before existing sessions receive their bounded grace policy. Registration, capability, or profile revocation initiates the declared cancellation/drain policy before further accepts. Replacement requires the prior generation to be terminal or to provide explicit cleanup evidence. Stale callbacks and imported stale descriptors deny before protocol delivery.

**Rationale:** A reusable listener must not race service replacement or keep hidden work alive after cancellation.

### 5. Framing, flow control, deadlines, and delivery claims remain unchanged

**Choice:** Cross-process sessions use the accepted canonical framing, session/stream ids, credit, queue, byte, deadline, cancellation, and failure contract. The live shell reads only the bounded outer frame before allocating payload storage. Submission remains pending until the declared acknowledgement boundary; failure after submission and before definitive acknowledgement remains uncertain delivery. The base shell performs no automatic retry.

**Rationale:** A new live topology must not strengthen delivery claims or create a second flow-control model.

### 6. Distinct-process evidence is an explicit admission boundary

**Choice:** The receipt-first fixture launches listener and client as separate child processes with distinct invocation identities and bounded teardown. The parent independently records both child starts, role bindings, endpoint handoff, terminal statuses, and cleanup outcomes; child-authored claims alone cannot establish process separation. Canonical evidence binds the run, participant roles, endpoint descriptor ref, transport profile, protocol/ALPN, service generations, request and payload refs, acknowledgement or failure boundary, resource summary, cancellation/drain state, cleanup outcomes, and non-claims. Raw process ids, raw locators, payload bytes, secrets, and runtime handles are excluded. Same-process loopback remains diagnostic evidence and cannot satisfy a distinct-process requirement.

**Rationale:** Process separation and cleanup must be observed rather than inferred from structurally plausible receipts.

### 7. Downstream consumers receive transport capability, not transport authority

**Choice:** Consensus and other system extensions may consume the admitted listener/session shell only through the canonical transport port. They still supply separate membership, application-principal, capability, policy, resource, provenance, and protocol-state admission. Transport evidence cannot satisfy consensus, durability, extension correctness, or production admission by itself.

**Rationale:** Unblocking a runtime mechanism must not collapse existing authority and claim boundaries.

## Functional core / imperative shell split

- **Pure core**: endpoint descriptor validation, listener/session plans, protocol/profile/peer compatibility, generation fencing, frame and credit checks, lifecycle transitions, failure and delivery classification, bounded status, receipt payloads, and deterministic diagnostics.
- **Imperative shell**: key and endpoint setup from admitted capability inputs, ALPN registration, locator publication, dial/accept, bounded byte movement, cancellation signals, child-process orchestration, teardown, receipt persistence, and operator rendering.

## Dependencies

- Accepted `fabric-transport-session-runtime` contract and shared adapter conformance.
- Accepted `iroh-alpn-routing-registry` ownership and collision rules.
- Accepted `system-extension-service-runtime` generation, supervision, and cleanup boundaries.
- Accepted `receipt-first-cluster-harness` separate-process and offline-evidence conventions.

## Risks / Trade-offs

- Iroh endpoint and connection APIs can tempt handle leakage into protocol code. Keep all adapter-owned values inside the shell and add structural guards.
- Listener tasks can survive failed tests or cancellation. Require bounded teardown, terminal events, and cleanup evidence on every positive and negative path.
- Endpoint descriptors can be mistaken for authority or freshness, and raw locators can expose local topology. Bind exact cohorts and expected peer context, enforce profile-controlled locator disclosure, and preserve explicit non-authority labels.
- A two-process exchange proves only the admitted transport path. It does not prove quorum behavior, consensus safety, durability, broad network compatibility, or production readiness.
- Relay and direct-address availability vary by environment. Record the admitted locator/profile and observed outcome without silently changing transport policy.

## Non-Goals

- Implementing Raft, consensus groups, replication, delivery retries, or application protocols.
- Granting membership, capability, policy, provenance, or application authority from endpoint possession or transport identity.
- Exposing raw Iroh, QUIC, socket, executor, or child-process handles to extensions.
- Adding ambient socket fallback, automatic retry, durable delivery, exact-once delivery, or global ordering.
- Treating same-process loopback or fabricated receipts as distinct-process live evidence.
