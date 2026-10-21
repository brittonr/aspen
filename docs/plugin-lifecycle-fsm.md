# Plugin lifecycle FSM

Plugin lifecycle admission is a finite transition relation over current plugin state, requested lifecycle event, active manifest ref, and explicit guard facts. Receipts for install, permission, activation, hostcall, health, removal, upgrade, extension negotiation, compatibility, rollback, cleanup, and recovery are inputs to the relation, not ambient authority by possession alone.

The pure FSM core returns a decision, prior state, event, next or preserved state, selected guard refs, side-effect class, authority-closed flag, diagnostics, and canonical `plugin-lifecycle-fsm-decision-v1` evidence. Shell code may invoke callbacks, hostcalls, upgrade cutover, or removal cleanup only after this decision admits the event and names a non-`none` side-effect class.

Removal and successful cleanup close plugin-owned hostcall authority. Later hostcall or upgrade events over the same manifest preserve the removed state and deny before plugin code can run. Failed health produces a degraded prior state for use/upgrade requests unless recovery evidence is explicitly present.

## Service state assertions

The lifecycle state machine remains the transition authority. Committed service state projects into one shared
assertion set so that dependents, operator readbacks, and configuration read the same facts.

| Assertion | Meaning |
|---|---|
| `started` | the service passed `starting` and did not fail terminally |
| `ready` | the service reported readiness; `ready` implies `started` |
| `complete` | a one-shot service finished normally, which is distinct from an intentional stop |
| `failed` | the service failed |
| user-defined | a user-defined state value that satisfies no built-in requirement |

`up` is a derived alias over `ready` or `complete`. A dependent names the assertion it requires (`ready`, `complete`, or
`up`), and a user-defined value never satisfies a built-in requirement.

The single mapping table from the lifecycle state machine, the system-extension phase vocabulary, and the production
readiness report lives in `src/lifecycle/parts/mod/p008/body.rs`, and the projection that reads it lives in
`src/lifecycle/parts/mod/p007/body.rs`. The service dependency predicate reads the projected set instead of a single
state value, and the service runtime projects a dependency status before it counts as ready. The projection is a pure
function over committed state: it never drives a transition, grants no authority, and replaces no lifecycle evidence.
