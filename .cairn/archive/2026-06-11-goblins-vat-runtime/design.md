## Context

Molten now has requirements for canonical Preserves communication, Synit/SAM-style dataspace semantics, Trellis-backed choreography, and Trellis-backed Raft consensus. The next layer is object execution: how a single actor or service structures its internal authority-bearing objects, how local object calls compose inside a turn, how references cross boundaries, and how object state persists or upgrades.

Spritely Goblins is useful prior art for this layer. Molten should not import Goblins as an implementation dependency. Instead, Molten should adopt the conceptual split:

```text
machine / process
  vat
    transactional actormap
      object reference -> object behavior/state
```

Molten maps this to an optional internal execution structure hosted by a SAM-style actor or service:

```text
runtime process or node
  Syndicate/SAM actor or service
    optional vat
      turn-scoped transactional object map
        object refs, capabilities, proxies, promises, snapshots
```

The public local runtime model remains actors, entities, facets, assertions, retractions, `Observe` patterns, and turns. A vat is not the public actor model itself; it is a fine-grained object-capability implementation technique that an actor may use internally. The vat/object layer sits beneath the local dataspace adapter. Dataspace assertions can carry references to objects or proxies, while object methods can publish assertions or request adapter effects only through the enclosing turn and policy gates.

## Goals

- Provide a fine-grained object-capability execution model inside Molten actors/services.
- Distinguish near synchronous object calls from far asynchronous object calls.
- Make local object state transactional within actor turns.
- Treat object references as authority, with no ambient authority by default.
- Support asynchronous promise/vow results and bounded promise pipelining.
- Support revocation, attenuation, accountable proxies, and rights amplification.
- Preserve authority graphs across snapshots, persistence, and upgrades.
- Make object-level failures replayable and inspectable with trace records and snapshots.
- Keep distributed object references transport-neutral and encoded through Molten envelopes.

## Non-Goals

- Do not adopt Guile Goblins, Racket Goblins, Scheme syntax, or Goblins APIs as Molten's runtime API.
- Do not require OCapN or CapTP wire compatibility in the first implementation.
- Do not replace the Synit/SAM dataspace layer, Trellis choreography layer, or Trellis Raft consensus layer.
- Do not allow object references to bypass Basalt/UCAN, Nickel/Steel contracts, Trellis predicates, Cairn receipts, or Preserves canonical boundaries.
- Do not make object snapshots trusted just because an object self-described them; snapshot authority must be bounded by the authority the object already held.

## Architecture

```text
Molten actor/service
  receives one runtime event
        |
        v
Vat turn
  transactional actormap snapshot/delta
  near synchronous object calls
  pending far sends, assertions, adapter requests
        |
        v
Admission and commit
  pure validation, policy gates, receipts
  commit object-state delta + pending actions
        |
        v
Dataspace / transport / store adapters
  assertions, messages, far calls, snapshots, traces
```

## Benefits of Vats

A vat-like internal object layer is useful because it provides:

- local synchronous programming ergonomics for near objects without pretending that remote objects are local;
- cheap transactional rollback for object state and pending outbound actions when a turn fails;
- explicit authority boundaries because object references are capabilities and there is no ambient path to unheld authority;
- failure containment for chains of near calls, avoiding half-mutated object graphs;
- efficient high-latency operation through promise pipelining for far references;
- revocation and attenuation as ordinary object/proxy structure;
- persistence and upgrade hooks that can preserve object state and the authority graph together;
- time-travel and distributed-debugging hooks based on actormap snapshots, turn deltas, and causal traces.

These benefits are internal to actor implementation. They do not replace the SAM dataspace model, the Preserves envelope boundary, Trellis choreography, or Trellis Raft consensus.

## Near and Far References

A reference is near when it denotes an object in the same vat and can be called synchronously during the current turn. A reference is far when it crosses a vat, actor, process, machine, transport, persisted object, or sandbox boundary. Far calls are always asynchronous and return promises/vows.

The runtime must not accidentally upgrade a far reference to synchronous call semantics. This preserves locality, avoids hidden blocking, and keeps network failure visible through promise failure or timeout policy.

## Transactional Actormap

Each vat owns an actormap: a mapping from local object ids to behavior/state. During a turn, near calls operate on a transactional view. Object state changes, spawned objects, object removals, and pending far sends commit only if the turn completes and admission succeeds.

If a turn fails, the runtime discards the actormap delta and pending actions. Trace records may still record the failed attempt if tracing policy permits, but failed state changes are not visible to later turns.

## Object Capabilities

Object references are capabilities. An object can exercise only references it was given, created, or obtained through an admitted resolver. Authority transfer is ordinary reference passing through method arguments, dataspace assertions, messages, protocol payloads, or restored snapshots. References crossing boundaries must be represented in canonical Preserves form as scoped reference descriptors or content/evidence refs.

Molten's policy layer remains authoritative for cross-boundary use. The object-capability discipline controls reachability; Basalt/Nickel/Steel/Trellis/Cairn control admission, contracts, bounded predicates, and evidence.

## Promises, Vows, and Pipelining

A far call returns a promise/vow. The caller may register success/failure/finally handlers or send additional pipelined messages to the promised future reference. Pipelined messages remain pending until the promise resolves. If it resolves to a reference, queued messages are forwarded in order subject to policy. If it breaks, queued messages fail with causal failure propagation.

Pipelining must be bounded by configured limits for queue length, lifetime, payload size, and authority scope. Pipelined operations must remain traceable and policy-visible before side effects occur.

## Revocation and Attenuated Proxies

Molten should support proxy references that narrow authority, log use, impose policy checks, transform payloads, or revoke access. Revocation invalidates the proxy and triggers cleanup of dependent assertions, subscriptions, pending far calls, and live references where applicable.

A revoked reference should not be silently reactivated by replay, snapshot restore, or transport reconnection unless a new admitted credential or resolver step grants replacement authority.

## Rights Amplification

Some cooperation requires two objects to prove a private relationship without making that relationship globally visible. Molten should support a sealer/unsealer or branded-token pattern:

- a private sealer creates opaque sealed values,
- a private unsealer reveals contents only to authorized holders,
- a public brand predicate may test provenance without revealing contents,
- sealed values carry only authority that was explicitly sealed.

This is useful for verifying shared origin, linking public/read references with private/admin references, proving registry membership, and implementing reviewed delegation workflows without ambient identity checks.

## Safe Serialization and Upgrade

Vat/object snapshots must preserve both state and authority graph. Objects may provide self-portraits or snapshot recipes, but only using references and authority already available to them. A serializer/unserializer authority should be sealed off from ordinary object access.

A snapshot artifact should include:

- vat id and snapshot id,
- object ids, behavior/schema versions, and state portraits,
- reference graph with scoped capabilities and attenuations,
- pending promise/vow state if admitted for persistence,
- upgrade recipe ids and schema versions,
- content hashes, receipts, and evidence references.

Restore applies upgrade code at explicit version boundaries and must not grant new authority unless policy admits it.

## Time Travel and Distributed Debugging

Turn traces and actormap deltas make replay possible. A debugging surface should be able to reconstruct object state at a prior turn, inspect reference graphs, show causality between far calls and promise resolution, and correlate object-level events with dataspace assertions, choreography sessions, Raft commits, policy decisions, and receipts.

Debugging surfaces must respect authority. A trace viewer should not reveal secret object state or references unless the viewer has an admitted debugging capability.

## OCapN/CapTP Concepts

Molten can borrow these concepts:

- distributed object references,
- secure session-scoped reference descriptors,
- promise pipelining,
- handoff/bootstrap of references,
- distributed lifetime/garbage tracking,
- transport abstraction over live and delayed connections.

Molten should encode these through its own Preserves envelope spine and Iroh/dataspace adapters. OCapN compatibility can be a later adapter if desired, not a first-version runtime contract.

## Portable Encrypted Storage

Object snapshots, large payloads, documents, and content artifacts should follow provider-independent storage principles:

- immutable content addressed by hash,
- mutable containers built from immutable chunks and signed state,
- encryption before storage so providers do not learn plaintext,
- chunking to reduce size leakage and support deduplication/streaming,
- read/write authority represented as capabilities,
- network/provider independence across Iroh blobs, local stores, or other adapters.

## Open Questions

- Should the first vat implementation be per actor, per service, or a separate runtime primitive actors can host?
- Should object behavior be native Rust only initially, or should Wasmtime objects participate in vats?
- How should promise pipelining interact with Trellis choreography sessions?
- Which reference descriptor format should represent far refs at the Preserves boundary?
- What is the minimal safe snapshot format before full Redb/content-addressed persistence exists?
