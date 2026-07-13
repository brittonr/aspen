## Why

Aspen has Iroh-backed exchange surfaces and peer bootstrap models, but system extensions cannot register an extension-owned protocol, accept sessions or streams, exchange framed messages, or observe transport lifecycle through a generic admitted port. Direct use of Iroh internals would couple extensions to one live adapter, bypass capability accounting, and prevent the same protocol core from running under deterministic simulation.

## What Changes

- Add a versioned fabric transport port independent of Iroh-specific runtime types.
- Add capability-gated protocol and ALPN registration with unique ownership, generation fencing, bounded listeners, and deterministic cleanup.
- Add canonical connection, session, stream, datagram, message, close, error, cancellation, and backpressure events.
- Bind peer transport identity separately from membership, application identity, authorization, and trust decisions.
- Provide live Iroh and deterministic-simulation adapters with one observable port contract.
- Define failure, retry, ordering, delivery, evidence, and non-claim semantics explicitly.

## Impact

- **Files**: transport port models, protocol registry, Iroh adapter shell, system-extension dispatcher integration, simulation adapter integration, operator readback, fixtures, and a new `fabric-transport` accepted spec.
- **Testing**: protocol registration, live/sim adapter contract tests, framing, stream lifecycle, bounded flow control, cancellation, identity separation, cleanup, and malformed/unauthorized event tests.
- **Safety**: transport identity and successful I/O do not confer application authority, membership, consensus, durability, exact-once delivery, or protocol compatibility.
