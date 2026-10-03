# Following a protocol session

Mode: Walkthrough

This walkthrough follows the checked-in `request_response_lifecycle` source fixture from manifest construction to lifecycle evidence. It is a source-reading exercise, not a network tutorial: no commands or runtime checks were executed for this page. Use the [Handbook](../README.md) for other practical guides and the [typed facade boundary](../../choreography-typed-facade.md) for the governing distinction between pure transitions and effects.

The concrete input is `proto:request-response`, with `client` and `server` roles, `request` and `response` labels, and matching payload tags. The fixture constructs synthetic schema, policy, capability, resource, and authority references. Those references make deterministic fixture inputs; they are not credentials suitable for a live request.

## 1. Construct the manifest, not a connection

Open [the fixture constructor](../../../src/protocol/parts/session/p003/body.rs), starting at `request_response_manifest_value`. Its global script contains exactly two communications: client to server carrying `request`, then server to client carrying `response`. Both payload tags have schema references. The manifest also carries policy, capability, and resource reference collections.

The observable boundary is the canonical manifest value returned by `protocol_manifest_value`. Keep that value alongside its canonical reference. Do not identify the protocol using a Rust debug dump, memory layout, or the friendly protocol identifier alone: canonical Preserves bytes and BLAKE3 define content identity.

## 2. Install and inspect projected endpoints

`request_response_lifecycle` passes the value to `install_protocol_manifest_value`. The [installation implementation](../../../src/protocol/parts/session/p001/body.rs) parses the manifest, builds registries, compiles the global choreography, checks Trellis projectability, and projects each role. Unsupported projected shapes can also produce a denial.

The boundary to inspect is the install receipt: decision, diagnostics, manifest, registries, and endpoints. The checked-in test expects a passing install with two endpoints. That is a test assertion inspected in source, not a reported test execution. If installation denies, stop here; `start_protocol_session` explicitly rejects a denied install. A usable transport cannot repair projectability.

## 3. Start two local endpoint states

The fixture starts both roles under `session:request-response:1`. [Session startup](../../../src/protocol/parts/session/p002/body.rs) selects the endpoint for the named role, begins at sequence zero, and initializes an empty seen-message list. Each state binds the installed protocol reference, session identifier, endpoint, local state, authority references, and resource references.

Preserve both initial states. They are separate role-local positions, not two copies of one global cursor. For this script, the client initially sends while the server initially receives. A future gate needs the actual initial values, not only a statement that startup succeeded.

## 4. Follow the request across the pure boundary

The client sends label/tag `request` to `server` with the body record containing `hello`. The send input includes the install receipt reference as evidence. The operation first checks admission inputs and the projected edge, then constructs a protocol message and advances the client's state.

The server receives that exact message value against its initial state. The [receive transition](../../../src/protocol/parts/session/p007/body.rs) checks duplicate identity before matching the expected action; matching includes protocol, session, sender, recipient, label, payload tag, and sequence. A successful receive records the message reference in the server's seen-message list.

At this boundary, collect the operation receipt, message value, and next-state value. Nothing in this fixture sends bytes over a socket. Its receive call directly consumes the value returned by send, and its carrier references are empty.

## 5. Follow the response and collect the trace

The server's next state sends label/tag `response` to the client with body `ok`, binding the receive-request receipt as evidence. The client receives using its post-send state, not its original state. The returned lifecycle contains two initial states and four ordered operations: send request, receive request, send response, receive response.

The [fixture test's gate-input helper](../../../src/protocol/parts/session/tests/m000/p000/body.rs) demonstrates collection: take each operation receipt, collect available message values, and collect available next states. It does not reconstruct values from log text.

## 6. Check completion without expanding its claim

`gate_protocol_session_lifecycle` recomputes installation evidence and checks operation evidence. Its [terminal trace walk](../../../src/protocol/parts/session/p005/body.rs) follows passing successors from each supplied initial state; missing, ambiguous, nonterminal, or over-bound traces produce diagnostics. Terminal means no remaining actions and `End`, not merely an `End` marker beside pending actions.

For a concrete failure comparison, the existing test attempts the client's first send with `response`. It expects denial without an output message. Correcting a label is appropriate only when the intended request and admitted endpoint agree; editing recorded state to force progress destroys the evidence chain.

The completed gate describes this supplied finite lifecycle. It does not establish delivery, durable remote effects, current authority, Raft membership, or production readiness. The [transport session companion](../../technical/transport/session-admission-and-transitions.md) explains why transport and peer sessions must remain separate from this choreography session.

## Sources

- [Handbook](../README.md)
- [Architecture: choreography and consensus scope](../../architecture.md#choreography-layer-trellis-backed-protocol-shape)
- [Typed choreography facade](../../choreography-typed-facade.md)
- [Transport session theory and distinctions](../../technical/transport/session-admission-and-transitions.md)
- [Concrete lifecycle constructor](../../../src/protocol/parts/session/p003/body.rs)
- [Installation and projection](../../../src/protocol/parts/session/p001/body.rs)
- [Operation implementation](../../../src/protocol/parts/session/p002/body.rs)
- [Existing lifecycle and denial tests](../../../src/protocol/parts/session/tests/m000/p000/body.rs)
