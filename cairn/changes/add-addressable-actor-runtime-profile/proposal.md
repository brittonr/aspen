## Why

Molten already owns durable state, placement, scheduling, supervised system extensions, and delivery mechanisms. It does not define one addressable actor profile that composes those mechanisms into keyed lifecycle, sleep, wake, and recovery behavior.

Without this profile, each workload can invent different rules for actor identity, wake triggers, mailbox delivery, checkpoint survival, and uncertain in-flight work. Rivet Actors provide a useful product reference for this composition, but Molten must keep Preserves identity, Basalt/UCAN authority, explicit generations, and bounded evidence as the canonical boundary.

## What Changes

- Add a versioned system-extension profile for actors addressed by canonical actor keys.
- Compose admitted placement, durable state, coordination delivery, logical time, resources, supervision, and evidence ports without adding a second actor core.
- Add explicit dormant, starting, running, draining, stopped, degraded, and recovery states with generation fencing.
- Define wake triggers for admitted messages, timers, connections, and operator requests.
- Define a survival matrix for durable state, mailbox entries, completed semantic events, checkpoints, processes, streams, sessions, and partial callbacks.
- Deny automatic replay when an external effect has an unknown outcome.
- Add read-only status and deterministic simulation fixtures for sleep, wake, restart, stale generations, duplicates, and failed recovery.

## Impact

- **Files**: Molten core lifecycle models, system-extension profiles, schemas, operator status, simulation fixtures, documentation, and `cairn/specs/addressable-actor-runtime/spec.md`.
- **Dependencies**: The profile depends on `coordination-delivery-system-extension` for lease, acknowledgement, retry, dead-letter, and redrive semantics.
- **Testing**: Positive sleep/wake and checkpoint recovery cases plus negative stale-key, stale-generation, duplicate wake, unknown-outcome, missing-authority, and unsupported-survival cases.
- **Boundary**: The profile records bounded actor lifecycle facts. It does not prove exactly-once effects, global address uniqueness, process survival, transport delivery, or production readiness.
