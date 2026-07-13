# Fabric transport session runtime

Aspen exposes extension-owned protocols through a canonical, adapter-neutral transport port. Protocol cores receive generation-scoped ids, commands, and events; they do not receive Iroh connections, sockets, executors, or simulator handles.

## Contract boundary

The pure contract lives in `crates/molten-core/src/fabric_transport/` and owns:

- versioned profiles, capabilities, framing, and finite listener/session/stream/byte/deadline limits;
- unique protocol and ALPN registration plus atomic generation transfer;
- separate transport, membership, application-principal, trust, capability, and bootstrap refs;
- legal session, stream, frame, datagram, credit, half-close, close, cancellation, and failure transitions;
- explicit pending, delivered, not-delivered, and uncertain delivery outcomes; and
- the rule that base transport performs no automatic retry.

Every protocol descriptor binds the extension and service identity, active generation, registration authority, profile, framing limit, listener limit, requested optional capabilities, and cleanup policy. Duplicate or conflicting registration denies. Replacement must advance the generation and present cleanup evidence for the prior owner. Once a listener drains, it refuses new sessions; cleanup waits until its generation has no live session.

## Adapter shells

`src/fabric_transport/` provides two shells over the same transition core:

- `IrohTransportAdapter` performs bounded live Iroh frame exchange and translates the result back into canonical events.
- `DeterministicTransportAdapter` runs the same command algebra while explicitly injecting overload, refusal, partition, timeout, disconnect, and adapter-failure observations.

Both adapters use the existing typed `FabricEffectPort` dispatcher through `RegisteredTransportEffectPort`. A registered effect must match the exact bound port, profile, request ref, service identity, and active generation before adapter execution.

The live loopback rail hashes and checks the frame before socket I/O, limits the read before allocation, records pending delivery at submission, and records delivered only after an explicit acknowledgement boundary. A connection failure after submission becomes uncertain delivery.

## Flow control and cancellation

Send credit, session and stream in-flight bytes, queue slots, framing bounds, and deadlines are explicit. Exhausted credit emits a canonical backpressure event without mutating transport state. Progress resumes only through an explicit credit, acknowledgement, cancellation, or terminal command. Unknown, terminal, wrong-service, or wrong-generation handles deny before extension callback delivery.

## Identity and authority

An authenticated transport identity is connectivity evidence only. Normal service admission requires separate membership, application-principal, trust-decision, and capability-authority refs. An explicit bootstrap policy may authorize a bounded exception. No transport event silently promotes a peer into membership or grants application authority.

## Evidence and readback

Canonical Preserves/BLAKE3 artifacts cover profile identity, protocol registration and transfer, lifecycle transitions, failures, and bounded aggregate status. Default readback includes counts, resource usage, failure/cancellation totals, the latest evidence ref, and non-claims. It excludes payload bytes, secrets, raw adapter handles, and packet-per-receipt requirements.

## Non-claims

The base transport port does not prove or provide:

- durable or exactly-once delivery;
- transactional messaging or global ordering;
- automatic retry safety;
- membership or application authority;
- consensus; or
- protocol compatibility or application-level success.

Higher-level extensions must select and evidence any stronger semantics.
