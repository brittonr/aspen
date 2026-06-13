## Why

Molten's runtime spine needs concrete local interaction semantics beyond "actors exchange envelopes". Synit and the Syndicated Actor Model provide useful prior art: actors publish assertions with lifetimes, subscribe through `Observe` patterns, process events in turns, use retractions for failure signalling, attenuate authority over both messages and assertions, resolve long-lived credentials through gatekeepers, express services as dependency assertions, and capture interaction traces as data.

Molten should adopt those architectural patterns while keeping Molten's own canonical envelope spine, Preserves boundary, Basalt/UCAN authority model, Nickel/Steel contract selection, Trellis predicates, Cairn receipts, Iroh transport adapters, and Redb storage boundaries.

## What Changes

- Define turn semantics for actor event handling: one event enters, pending actions accumulate, policy gates run, and actions commit or roll back as a unit.
- Define assertion lifetimes: assertions are maintained by their asserting actor/session and are automatically retracted on termination, crash, session close, or capability revocation.
- Define `Observe`-style subscriptions over the implemented Preserves pattern subset for local dataspace routing.
- Define deterministic exact-value and wildcard Preserves pattern matching for subscription, routing, and policy-visible matching; richer compound matching remains future admitted work.
- Define capability attenuation over assertions/subscriptions and messages through Molten policy/authority gates rather than Synit sturdyrefs as authority; rewrite transforms require explicit future rule evidence.
- Define a gatekeeper resolver pattern for converting long-lived credentials, UCANs, tickets, invites, or authority contexts into live scoped references.
- Define service dependency assertions and supervision evidence for demand-driven startup, readiness, restart, shutdown, and cleanup of Molten services/actors.
- Define interaction tracing as canonical Preserves records/reports for actor, dataspace, policy, choreography, consensus, service, and replay activity.
- Treat Synit and SAM as non-normative design references; do not claim Synit wire protocol, sturdyref, PID1, service-manager, or scripting-language compatibility.

## Impact

This change turns the local runtime from a generic message bus into a reactive dataspace runtime with explicit conversational state, turn atomicity, failure cleanup, authority attenuation, and traceability. The archived scope reflects the implemented local runtime: pending turn commit/rollback, owner-scoped assertions and observers, exact/wildcard pattern predicates, scoped authority/live refs, service demand/readiness/supervision reports, and canonical trace/report evidence. Richer compound pattern matching, generic attenuation rewrites, and durable/replicated dataspace semantics remain explicit future extensions.
