## Why

Main's Sans-IO protocol pattern is a strong fit for Molten's replay and evidence goals. Molten already has protocol sessions, node-control ingress, Iroh transports, dataspace turns, Trellis gates, and deterministic replay, but protocol logic can still become coupled to async shells, transport sessions, clocks, stores, or logging if the boundary is not named.

Molten should adapt the pattern as a native runtime rule: protocol state machines are pure deterministic cores that return explicit envelopes, effect intents, state updates, alarms, and receipts for shells to admit and perform.

## What Changes

- Define a Sans-IO protocol-core contract for Molten-owned protocol state machines.
- Require protocol cores to consume explicit state, event/message, limits, policy/admission facts, and deterministic time/seed inputs rather than ambient IO.
- Require shells to perform Iroh sends, Redb writes, dataspace publication, receipt storage, and tracing only after the pure core returns explicit outputs and the normal admission gates pass.
- Add in-memory harness fixtures so protocol behavior can be tested without live Iroh, Redb, clocks, or async runtimes.

## Impact

- **Files**: runtime-patterns specs, testing-harness specs, docs, future protocol/session cores, Iroh adapter shells, and replay fixtures.
- **Testing**: positive fixtures for deterministic state transitions and envelope output; negative fixtures for ambient IO, hidden clock/random use, shell-side mutation before admission, malformed messages, and illegal transitions.
- **Security**: transport, storage, and runtime shells do not gain authority from protocol code. Authority, policy, resource, provenance, replay, and receipt gates remain explicit and fail closed before side effects.