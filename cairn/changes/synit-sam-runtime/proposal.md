## Why

Molten's runtime spine needs concrete local interaction semantics beyond "actors exchange envelopes". Synit and the Syndicated Actor Model provide useful prior art: actors publish assertions with lifetimes, subscribe through `Observe` patterns, process events in turns, use retractions for failure signalling, attenuate authority over both messages and assertions, resolve long-lived credentials through gatekeepers, express services as dependency assertions, and capture interaction traces as data.

Molten should adopt those architectural patterns while keeping Molten's own canonical envelope spine, Preserves boundary, Basalt/UCAN authority model, Nickel/Steel contract selection, Trellis predicates, Cairn receipts, Iroh transport adapters, and Redb storage boundaries.

## What Changes

- Define turn semantics for actor event handling: one event enters, pending actions accumulate, policy gates run, and actions commit or roll back as a unit.
- Define assertion lifetimes: assertions are maintained by their asserting actor/session and are automatically retracted on termination, crash, session close, or capability revocation.
- Define `Observe`-style subscriptions over Preserves patterns for local dataspace routing.
- Define Preserves pattern matching and deterministic binding semantics for subscription, routing, and policy-visible matching.
- Define capability attenuation over both assertions/subscriptions and messages, implemented through Molten policy gates rather than Synit sturdyrefs as authority.
- Define a gatekeeper resolver pattern for converting long-lived credentials, UCANs, tickets, or invites into live scoped references.
- Define service dependency assertions for demand-driven startup, readiness, restart, and shutdown of Molten services/actors.
- Define interaction tracing as canonical Preserves records for actor, dataspace, policy, choreography, and consensus activity.
- Treat Synit and SAM as non-normative design references; do not claim Synit wire protocol, sturdyref, PID1, service-manager, or scripting-language compatibility.

## Impact

This change turns the local runtime from a generic message bus into a reactive dataspace runtime with explicit conversational state, turn atomicity, failure retraction, authority attenuation, and traceability. It also gives later choreography and consensus work a cleaner substrate: choreography steps become admitted protocol assertions/messages within a turn, and Raft-backed control-plane updates can publish committed facts into the dataspace with the same lifetime and tracing rules.
