# Proposal: Unify service state assertions

## Why

Three vocabularies answer the same question. The service lifecycle FSM holds one value from a ten-variant enum
(`src/lifecycle/parts/mod/p000/body.rs`: `declared`, `spawning`, `starting`, `ready`, `degraded`, `stopping`,
`stopped`, `failed`, `restarting`, `cleaned`). System extensions use a phase vocabulary (`starting`, `ready`,
`draining`, `failed`, `stopped`) in `src/system_extension/`. Production readiness uses its own ready and failed
reporting.

The manual separates two facts that the single enum merges: "The overall state of the service is the union of asserted
`state`s" over `started`, `ready`, `failed`, `complete`, or a user-defined value
(`12-operation__service.md → Convey the current state of a service`). `ready` is asserted in addition to `started`, and
`complete` reports a one-shot program that finished normally. The config layer then derives an `up` alias from `ready`
or `complete` (`19-operation__synit-config.md → Synthesis of service state "up"`).

Molten has no `complete` state, so a dependent cannot wait for "finished successfully" and a normal one-shot exit looks
like `stopped`. The accepted requirement `molten.runtime_spine.service_dependency_assertions` already requires the
runtime to represent demand, readiness, failure, completion, restart, and shutdown; the tracey baseline lists it as
implementation-unestablished.

## What Changes

- Add a derived service state assertion set with `started`, `ready`, `complete`, `failed`, and user-defined values,
  where `ready` implies `started` and `complete` reports a normal one-shot finish. r[molten.runtime_spine.service_state_union]
- Keep the lifecycle FSM as the single transition authority. The assertion set is a projection of committed lifecycle
  and runtime state, and it MUST NOT drive transitions or grant authority.
- Derive an `up` alias over `ready` or `complete` so a dependent can wait for either.
- Map the system extension phase vocabulary and the production readiness vocabulary onto the shared set in one place,
  so the three surfaces agree without renaming their local types.
- A user-defined state MUST NOT satisfy a dependency that names `ready`, `complete`, or `failed`.

## Impact

- **Files**: `src/lifecycle/`, `src/system_extension/canonical.rs`, the production readiness module, the service
  dependency predicate, `docs/plugin-lifecycle-fsm.md`, `docs/architecture.md`, and the referenced tests.
- **Testing**: positive cases for a one-shot service reaching `complete`, a service holding `started` and `ready`
  together, and a dependent that waits for `up`; negative cases for a user-defined state satisfying a `ready`
  dependency, a process that exits before readiness reporting `ready`, and a projection that changes FSM state.
- **Non-goals**: no replacement of the lifecycle FSM, no OS process parentage change, no new authority, and no service
  graph hardcoding.
