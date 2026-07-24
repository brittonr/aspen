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

`src/fabric_transport/` provides adapter shells over the same transition core:

- `IrohTransportAdapter` owns canonical live-Iroh state transitions. Its same-process loopback remains a diagnostic conformance rail, not distributed-runtime evidence.
- `RegisteredCrossProcessTransportEffectPort` routes an admitted `SendFrame` through the typed `FabricEffectPort`, an exact imported endpoint descriptor, and a distinct Iroh peer without exposing Iroh-specific branches to the consuming extension.
- `DeterministicTransportAdapter` runs the same command algebra while explicitly injecting overload, refusal, partition, timeout, disconnect, and adapter-failure observations.

The registered ports require the exact bound port, profile, request ref, service identity, and active generation before execution. Cross-process sends additionally require request-bound payload bytes registered under the profile's queue/frame limits. Each request is consumed once; the shell never retries automatically. Successful exchange returns the canonical acknowledgement transition. Adapter or malformed-exchange failure returns a canonical failed-session transition.

The live rails hash and check the frame before socket I/O, parse the length prefix before bounded allocation, record pending delivery at submission, and record delivered only after an explicit acknowledgement boundary. Failure after submission and before acknowledgement remains uncertain delivery.

## Cross-process endpoint handoff

The pure endpoint contract is in `crates/molten-core/src/fabric_transport/cross_process/`. A handoff is canonical Preserves that binds profile, protocol/ALPN, extension/service owner, generation, endpoint identity, listener identity, peer context, locator and validity cohorts, disclosure policy, framing/resources, and non-claims. Private keys, capabilities, runtime handles, and ambient paths are never serialized.

The initial shell intentionally supports an explicit loopback bind only. Handoffs may contain the Iroh direct IP locators returned by that endpoint; relay/custom/private locator use is not part of the admitted initial operator profile. Default status emits locator classes and refs, not raw locator values. Import checks every expected binding before dial and has no ambient endpoint, socket, protocol, profile, simulation, or locator fallback.

A listener publishes only after endpoint setup, exact ALPN activation, registration/capability/profile admission, and readiness. It can accept bounded sessions repeatedly. Drain stops admission, requires active sessions to reach terminal state, awaits endpoint close, and emits cleanup evidence. Client endpoint close is awaited and chained into its terminal cleanup ref.

## Flow control and cancellation

Send credit, session and stream in-flight bytes, queue slots, framing bounds, and deadlines are explicit. Exhausted credit emits a canonical backpressure event without mutating transport state. Progress resumes only through an explicit credit, acknowledgement, cancellation, or terminal command. Unknown, terminal, wrong-service, or wrong-generation handles deny before extension callback delivery.

Timeout, disconnect, malformed framing, partial I/O, and adapter failure terminate the affected session without automatic retry. Submission without definitive acknowledgement remains uncertain. Listener cancellation/revocation follows explicit drain and cleanup transitions; stale-generation callbacks deny before protocol delivery.

## Identity and authority

An authenticated transport identity is connectivity evidence only. Normal service admission requires separate membership, application-principal, trust-decision, and capability-authority refs. An explicit bootstrap policy may authorize a bounded exception. No transport event silently promotes a peer into membership or grants application authority.

## Distinct-process evidence and operator workflow

Run the checked two-process conformance path and then verify it offline:

```console
molten cluster fabric-transport-run --run-dir target/fabric-transport-run
molten cluster fabric-transport-verify --run-dir target/fabric-transport-run
```

The parent starts separate listener/client child commands, observes both starts, waits for the canonical handoff before client launch, bounds both waits, reaps both children, and writes a fixed-membership artifact directory. `parent-run.preserves` binds distinct invocation refs, child terminal refs, descriptor/profile/protocol/ALPN/service/generation, request/payload/ack refs, bounded resource totals, and cleanup. `artifact-index.tsv` hashes every admitted artifact; `verification.preserves` is a companion offline result. Unexpected, missing, oversized, non-regular, symlinked, non-canonical, re-bound, stale, child-only, or index-mismatched artifacts deny verification.

`failure.preserves` is emitted on parent-shell failure. It is always non-pass, binds only a hash of the raw error, records that child lifetimes are scope-bound without claiming cleanup success, and does not claim process separation. Diagnostic logs are bounded run members, not authority or success receipts.

Canonical Preserves/BLAKE3 artifacts cover profile identity, endpoint handoff, parent-observed process starts, protocol registration and transfer, session acknowledgement/failure, listener drain, cleanup, and bounded aggregate status. Default readback excludes payload bytes, secrets, raw process ids, raw locators, adapter handles, and packet-per-receipt requirements. Same-process loopback cannot satisfy distinct-process admission.

## Non-claims

The base transport port does not prove or provide:

- durable or exactly-once delivery;
- transactional messaging or global ordering;
- automatic retry safety;
- membership or application authority;
- consensus; or
- protocol compatibility or application-level success;
- cross-host, relay, WAN, NAT-traversal, fleet, or availability qualification; or
- adversarial process attestation, sandboxing, confidentiality, or production key provisioning.

The distinct-process harness establishes one bounded same-host connectivity observation for its exact inputs. It does not establish a production deployment or any stronger distributed-system claim.

Higher-level extensions must select and evidence any stronger semantics.
