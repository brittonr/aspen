## Context

Molten protocol work spans node-control workflows, peer/session negotiation, remote artifact sync, job worker coordination, Iroh framed streams, Trellis protocol gates, and dataspace-backed local endpoints. Those flows need the same logic to run in production, replay, property fixtures, and denial tests. If the logic calls async IO, reads clocks, inspects process state, or writes receipts directly, it becomes hard to replay and impossible to test as a functional core.

The adapted pattern is Sans-IO in Molten terms: a protocol core is synchronous and deterministic, while a shell performs IO after admission.

## Design

### Protocol core shape

Each protocol core should be a pure Rust module or type that consumes explicit inputs:

- current protocol state or state summary;
- incoming canonical message/event/ref facts;
- deterministic limit profile and resource facts;
- authority, policy, replay, and capability admission facts where the transition needs them;
- deterministic time or sequence facts when a transition depends on freshness;
- optional already-recorded replay/effect responses.

The core returns a transition result containing:

- next state or state delta;
- outbound canonical envelope descriptions;
- effect intents to be admitted by normal adapter gates;
- alarms or diagnostics;
- receipt input facts, not persisted receipts;
- denial decisions for malformed, stale, illegal, or unauthorised transitions.

The core must not import or call filesystem, network, Redb, Wasmtime, Steel, Nickel runtime, wall-clock, random, async runtime, process, tracing, stdout/stderr, or receipt-storage APIs.

### Shell boundary

The shell owns all effects:

```text
receive bytes / event from adapter
  -> parse and canonicalize boundary value
  -> call protocol core with explicit state and facts
  -> run policy/resource/authority/evidence gates for returned intents
  -> persist state and receipts
  -> send Iroh/dataspace/blob/store effects
```

Shells may translate core outputs into Iroh frames, dataspace assertions, Redb mutations, receipt files, or trace records only after admission. A shell must not perform a side effect speculatively unless a future reserve/commit/abort extension records the preparation and abort semantics explicitly.

### In-memory harness

Testing should use an in-memory protocol context or plain input/output fixtures that records generated envelopes, state deltas, effect intents, diagnostics, and receipt facts. The same core should run under:

- deterministic unit tests;
- Hegel/property generators;
- replay fixtures;
- live adapter shells;
- negative fixtures for illegal transitions and missing evidence.

### Canonical evidence

Messages crossing protocol boundaries should have canonical Preserves representations or typed wrappers that can produce canonical refs. Rendered logs are diagnostic only. Persisted replay fixtures should bind the input message refs, before/after state refs, output envelope refs, denied effect refs, and admission receipt refs.

### Non-goals

- Do not create a generic framework that every small helper must use.
- Do not bypass Trellis where a protocol requires checked choreography or finite-session proofs.
- Do not let a test context or handler profile grant authority by itself.
- Do not claim compatibility with Aspen's API or any external Sans-IO crate.