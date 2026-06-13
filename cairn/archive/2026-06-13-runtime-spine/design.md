## Context

Molten should become a programmable distributed runtime rather than a pile of unrelated integrations. The shared integration point is a versioned envelope that can be serialized canonically, routed locally, replicated remotely, inspected by tools, admitted or rejected by policy, and passed to sandboxed code.

## Goals

- Make `molten-core` deterministic and free of filesystem, network, process, and clock access.
- Make all external crates adapters around the same envelope model.
- Preserve canonical hashes for messages, content references, policies, and receipts.
- Let runtime surfaces be added incrementally without changing the core data model.

## Non-Goals

- Do not implement a full distributed consensus layer in this change.
- Do not expose ambient filesystem or network access to Wasmtime actors.
- Do not make Nickel evaluation part of message dispatch hot paths.
- Do not use Steel contracts or scripts to bypass envelope admission, capability checks, or receipt emission.
- Do not claim that policy gates prove semantic correctness; they only admit bounded operations according to explicit predicates and evidence.

## Runtime Inspirations

These systems are design references, not compatibility targets. Molten should borrow proven runtime patterns while keeping its own envelope, evidence, policy, and networking boundaries explicit.

### BEAM / Erlang / Elixir / OTP

Borrow:

- Isolated lightweight processes with no shared mutable state.
- Per-actor mailboxes and explicit receive/dispatch behavior.
- Links and monitors for failure propagation and observation.
- Supervision trees and restart strategies for resilient runtime structure.
- Registries and OTP-style behavior templates for named actors and services.
- Reduction-budget or work-budget scheduling ideas for fair native and Wasm actor execution.

Avoid for the first runtime milestone:

- BEAM distribution wire-protocol compatibility.
- Full selective receive semantics before fairness and backpressure are explicit.
- Hot code upgrade semantics before module identity, evidence, and policy gates are stable.

Trellis already includes BEAM-oriented verified primitives; those can serve as formal reference material for mailbox, process, link, monitor, registry, and scheduler invariants when choosing bounded admission predicates.

### Lunatic

Lunatic is useful prior art for a Wasm-first actor runtime. Use it as a reference for:

- Wasm actor lifecycle and module loading ergonomics.
- Spawn, link, monitor, and message APIs.
- Narrow hostcall design and sandbox defaults.
- Capability-like host resource exposure.
- Distributed-process ergonomics around sandboxed actors.

Molten should differ deliberately: the stable boundary is the Preserves envelope spine; remote transport is Iroh/Syndicate-backed; admission uses Basalt/Trellis/Cairn/Octet evidence; and WASI or host resources remain deny-by-default.

## Architecture

```text
molten-core
  Envelope, ActorId, ContentRef, Capability, EvidenceRef
  Serde DTOs, Preserves conversion, and Blake3 canonical hashing
  Snafu error types at adapter/core boundaries
  no I/O, no networking, no scripting

molten-config
  Nickel-authored config and static contracts evaluated into typed MoltenConfig/policy artifacts

molten-runtime
  local actor/dataspace adapter using Syndicate concepts
  subscription, assertion, and dispatch boundaries
  Tracing spans/events at runtime and adapter boundaries

molten-net
  Iroh endpoint integration
  gossip topic bridge for envelope-sized messages
  blob store bridge for content-addressed payloads
  docs bridge for replicated mutable document/state surfaces

molten-exec
  Wasmtime sandbox actor adapter
  WASI capability wiring for explicitly admitted host resources
  WIT/component interface bindings and Wasm validation/inspection tooling
  Steel trusted operator/repl orchestration and dynamic contract backend adapter

molten-policy
  Basalt UCAN/Nickel/Steel contract enforcement
  Trellis admission predicates
  Cairn action-envelope and receipt validation
  Octet/Valence provenance and evidence references

molten-store
  Redb-backed local durable metadata, receipts, indexes, and replay caches

molten-cli
  Clap-based imperative shell for running nodes, loading config, joining swarms, and inspecting state

molten-tests
  Hegel property-based tests for envelope, admission, and adapter invariants
```

## Core Envelope

The first implementation should model an envelope with at least:

- `version`: schema version for compatibility checks.
- `from`: stable actor or node identity.
- `subject`: routable topic or assertion subject.
- `body`: Preserves value.
- `blobs`: content references for large immutable payloads.
- `capabilities`: explicit authority presented with the message.
- `evidence`: references to receipts, function objects, module provenance, or policy decisions.

Preserves is the canonical representation at every communication boundary. Actor envelopes, dataspace assertions/messages, choreography protocol messages, Raft command/message envelopes, Iroh gossip payloads, blob metadata, Wasmtime hostcall messages, Steel/runtime API boundary values, policy decisions, receipts, evidence references, and persisted records that need stable identity all need a canonical Preserves representation. Rust structs and adapter-native types may be ergonomic internal wrappers, but hashing, signing, persistence, routing, policy admission, and interop are defined over canonical Preserves bytes or over Preserves metadata that references content-addressed blobs for large payloads.

## Adapter Boundaries

### Syndicate

The local runtime adapter maps envelopes to local assertions, messages, and subscriptions. The adapter owns runtime scheduling and dataspace interaction; `molten-core` only owns typed data and pure validation.

### Iroh

The network adapter sends small envelopes over gossip topics. Envelopes that exceed a configured threshold carry blob references instead of inline data. Blob payloads are fetched through Iroh blobs and checked against the content reference before admission. Replicated mutable document/state surfaces use Iroh docs and must still emit envelope-level evidence for application-visible mutations.

### Wasmtime, WASI, and components

The sandbox adapter uses Wasmtime for execution, Wasmtime-WASI only for explicitly admitted host resources, WIT bindings for typed component interfaces, and wasmparser for pre-admission module inspection. The adapter exposes narrow hostcalls:

- `send(envelope)`
- `subscribe(pattern)`
- `blob_get(content_ref)`
- `blob_put(bytes)`
- `now()` only if represented as an explicit host capability

All hostcalls pass through policy admission before side effects occur. WASI access is deny-by-default: ambient filesystem, clocks, environment, and sockets are unavailable unless represented as explicit capabilities and receipts.

### Steel

Steel is trusted orchestration and experimentation glue: spawn actors, inspect state, patch subscriptions, and run local workflows. Steel scripts should call runtime APIs rather than bypassing the envelope spine. Steel contracts are appropriate only when a reviewed dynamic predicate or trusted callable is required; they must still emit contract envelopes, admission receipts, and evidence references.

### Nickel

Nickel owns declarative configuration for nodes, swarms, topics, actor declarations, adapter options, static contract policy, schemas, and policy bundles. Nickel evaluation happens at configuration load/export time and yields typed Rust config plus reviewed policy artifacts. Prefer Nickel contracts whenever a gate can be represented as static declarative data.

### Nickel and Steel contract selection

Molten should use Nickel and Steel contracts wherever a runtime action crosses a trust boundary: spawning actors, granting capabilities, joining topics, mutating Iroh docs, exposing WASI resources, installing Wasm modules, attaching evidence, or persisting receipt indexes.

Selection rule:

- Use a Nickel contract for static declarative policy, schemas, resource prefixes, allowed abilities, adapter options, and default reviewable configuration.
- Use a Steel contract only for explicitly reviewed dynamic predicates or trusted callables that cannot be represented as static Nickel data.
- Route both contract backends through Basalt contract envelopes and UCAN enforcement before side effects occur.
- Record the selected backend, contract id, normalized source hash, input/output schema ids, decision, and receipt reference in envelope evidence or operation receipts.

### Basalt, Trellis, Cairn, and Octet/Valence

Basalt enforces UCAN-backed contract boundaries over Nickel policy artifacts and Steel backends. Trellis provides bounded admission predicates for capability containment, delegation, replay prevention, leases, routing limits, and content integrity checks. Cairn validates action envelopes and lifecycle receipts. Octet/Valence evidence references bind function/module provenance and bounded local replay evidence.

### Redb

Redb is the first local embedded store for durable node metadata, receipt indexes, replay/admission caches, and content-reference bookkeeping. The store adapter owns filesystem effects; core validation remains pure and receives explicit snapshots or records.

## Implementation Order

1. Add `molten-core` modules inside the current crate: envelope, ids, refs, capability, evidence, and deterministic validation.
2. Add Serde DTOs, Preserves conversion, and canonical hash tests for positive and negative fixtures.
3. Map BEAM/OTP and Lunatic reference patterns to Molten actor lifecycle, supervision, mailbox, and Wasm hostcall boundaries without claiming protocol or API compatibility.
4. Add an in-process runtime prototype with two native Rust actors and no network.
5. Add Nickel config loading for those actors and subscriptions.
6. Add Iroh gossip/blob bridge behind an adapter trait.
7. Add Wasmtime actor hostcalls, WASI capability wiring, WIT/component bindings, and wasmparser-based pre-admission inspection with deny-by-default policy admission.
8. Add Steel orchestration on top of public runtime APIs.
9. Add Nickel and Steel contract artifacts for applicable trust-boundary actions, with Basalt enforcement and receipt emission.
10. Add Basalt/Trellis/Cairn/Octet policy and evidence gates around runtime actions.
11. Add a Redb store adapter for local durable metadata and receipt indexes.
12. Add Hegel property-based tests for envelope, admission, and adapter invariants.

## Open Questions

- Should `ActorId` be human-named, key-derived, or both?
- Should envelope subjects be strings initially or Preserves patterns from the start?
- Which Trellis predicates are stable enough for the first policy gate?
- Should CLI inspection show canonical Preserves, JSON projections, or both?
