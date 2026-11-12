# Inbound admission rate-limit review

Status: finding recorded; no typed receive gate implemented.

## Question

Does an inbound fabric-transport session pass a per-source rate limit before
Molten resolves membership, application principal, trust decision, and
capability refs for service admission?

## Finding

No Molten-owned per-source rate limit was found in
`crates/molten-core/src/fabric_transport/` or `src/fabric_transport/`. The
current limits are concurrency, credit, in-flight bytes, queue slots, and frame
size bounds:

- `crates/molten-core/src/fabric_transport/cross_process/listener.rs` admits a
  session only while `active_sessions < max_sessions`; this is a concurrency cap,
  not a rate over time.
- The session contracts bound credit, in-flight bytes, queued frames, and frame
  length. They do not meter new sessions or admission attempts per source.
- The live cross-process shell binds loopback only. WAN, relay, and NAT traversal
  are explicit non-claims in
  [fabric transport sessions](fabric-transport-session-runtime.md).

Iroh/QUIC may apply transport-level address validation or connection limits, but
Molten does not currently rely on them as an admission budget, and this review
does not claim they cover identity-ref resolution cost.

## Decision

Defer a typed receive gate while exposure remains an explicit loopback-only
profile. Reopen the gate when a non-loopback direct-LAN or relay profile is
admitted, or when measurements show ref resolution is expensive enough to
justify a pre-admission budget.

If implemented, the gate should follow the existing functional-core pattern:

- one `admit_inbound(source, observation, now)` entry point that applies a
  deterministic per-source token bucket before any ref resolution;
- one `GatedInbound` type with crate-private construction, accepted by the costly
  admission handlers instead of raw `PeerIdentityRefs`;
- injected time in the pure core, canonical deny/backpressure evidence, and
  byte-for-byte preservation of in-budget transitions;
- a compile-fail test proving the expensive handlers cannot be called with an
  ungated value.

The gate must not change the rule that transport identity grants no membership
or authority. It also must not invent a failure detector, consensus semantics, or
wall-clock reads in the pure core.
