# Proposal: Promote message passing to the Molten runtime boundary

## Why

Molten already has canonical transport commands, system-extension callbacks, typed effects, deterministic adapters, scheduler choices, and same-core simulation. These mechanisms do not yet state one runtime law that connects them.

A core can avoid ambient network access while still observing a live connection, borrowed stream buffer, channel handle, adapter-local retry state, or callback path that bypasses the scheduler. Such paths weaken deterministic replay and adapter substitution.

Molten must make explicit messages the only semantic communication primitive across independent state owners and effect boundaries. Stateful connections must remain shell-adapter mechanisms.

## What Changes

- Define the canonical message-boundary contract for system extensions, vats, services, fabric ports, and effect completions.
- Require bounded owned message values with versioned schemas and explicit behavior-affecting facts.
- Treat request, message, stream, timer, lifecycle, health, checkpoint, recovery, and effect-completion callbacks as canonical inbound messages.
- Keep live sockets, Iroh connections, channels, clients, executors, tasks, and borrowed buffers inside declared shells or adapters.
- Permit logical session and stream state only as explicit handle-free values derived from messages.
- Put message delivery and every modeled nondeterministic completion under the deterministic simulation scheduler.
- Require live and deterministic adapters to preserve one application message and event contract while keeping adapter-specific facts explicit.
- Adopt the published Octet message-boundary architecture mechanism as a strict source gate.
- Audit active runtime Cairns so connection-shaped wake or recovery behavior enters through messages rather than hidden connection state.

## Impact

- **Specs**: `fabric-transport`, `fabric-simulation`, `system-extension-runtime`, and `project` deltas.
- **Core**: canonical boundary messages, state-owner declarations, transition results, logical session state, and scheduler events.
- **Shells and adapters**: live-handle containment, owned frame conversion, reconnect and uncertainty events, deterministic adapters, and composition roots.
- **Evidence**: Octet static evidence, same-core identity, scheduler closure, differential traces, replay, and claim-profile refs.
- **Testing**: positive message flow plus negative handle escape, shared-state bypass, borrowed-buffer escape, callback bypass, hidden retry, missing scheduler choice, and live/simulation drift.

## Dependencies

Implementation depends on a published immutable Octet revision that contains `enforce-message-only-core-boundaries`.

The existing fabric-transport, fabric-simulation, system-extension-runtime, fabric-time, durable-state, authority, resource, and evidence contracts remain owners of their current semantics.

## Non-goals

- Do not create a second actor core, universal message enum, mandatory broker, or global queue.
- Do not ban Iroh, QUIC, sockets, streams, channels, connection pooling, or async execution inside shells and adapters.
- Do not treat a logical session identifier or explicit protocol state as a live connection handle.
- Do not claim exact-once delivery, durable delivery, global ordering, complete determinism, protocol correctness, security, availability, or production readiness.
- Do not replace ChaosControl VM or live-network evidence with in-process deterministic simulation.

## Completion Evidence

The change is complete when all declared state-owner boundaries use canonical messages, Octet rejects handle escape, deterministic scheduling closes modeled choices, live and simulated adapters pass shared conformance, and negative fixtures fail at the exact bypass boundary.
