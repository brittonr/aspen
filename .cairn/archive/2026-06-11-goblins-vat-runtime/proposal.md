## Why

Molten's Synit/SAM-inspired dataspace semantics define reactive conversational state, but Molten also needs a fine-grained object-capability execution model inside actors and services. Spritely Goblins provides strong prior art for this layer: vats containing near objects, transactional actormaps, object capabilities as ordinary references, asynchronous far-object calls, promise pipelining, revocation, rights amplification, safe serialization, time-travel debugging, and transport-agnostic distributed object references.

Molten should adopt these concepts as design patterns while preserving Molten's own boundaries: canonical Preserves envelopes, Basalt/UCAN authority, Nickel/Steel contracts, Trellis predicates, Cairn receipts, Synit/SAM-style dataspaces, Trellis choreography, Trellis Raft consensus, Iroh transport, Wasmtime sandboxing, and Redb/content-addressed storage.

## What Changes

- Define a vat/object layer inside Molten actors or services with near synchronous calls only within a vat and far asynchronous calls across vat, actor, process, or machine boundaries.
- Define a transactional actormap for local object state so near synchronous calls and object state changes commit only when the enclosing turn commits.
- Treat object references as capability-bearing authority, with reference passing as the ordinary authority-transfer mechanism.
- Define promise/vow values for far-object calls and support bounded promise pipelining to reduce round trips while preserving failure propagation.
- Define revocable and attenuated proxies for narrowing or cancelling authority and for retracting dependent assertions/subscriptions on revocation.
- Define sealer/unsealer or branded-token rights-amplification patterns for private cooperation between objects without ambient authority.
- Define safe serialization and upgrade for vat/object snapshots that preserve the authority graph and allow objects to describe persistence using only authority they already hold.
- Define time-travel and distributed-debugging hooks based on turn traces, actormap snapshots, and replayable inputs.
- Borrow OCapN/CapTP concepts such as session-scoped distributed references, handoff/bootstrap, promise pipelining, and distributed lifetime tracking without requiring OCapN wire compatibility.
- Adopt portable encrypted storage principles for content-addressed, encrypted, chunked, provider-independent blob/container storage.
- Treat Goblins as non-normative design reference material; do not depend on Guile/Racket Goblins or claim Goblins/OCapN compatibility in the first implementation.

## Impact

This adds the missing fine-grained object model beneath Molten's dataspace and protocol layers. A Molten actor can host transactional local objects, expose attenuated references through dataspaces or protocol sessions, call remote/far objects asynchronously, persist and upgrade object graphs safely, and debug failures by replaying turn-level object-state changes. The result keeps Molten's core design coherent: Synit/SAM informs the dataspace layer, Goblins informs the vat/object layer, Trellis informs choreography and consensus, and Preserves remains the canonical boundary for all communication and evidence.
