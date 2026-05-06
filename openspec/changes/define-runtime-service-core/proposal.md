## Why

The runtime host-loading baseline defines how Aspen will resolve and start runtime artifacts, but it does not implement the Aspen runtime itself. Aspen still lacks the durable service/application contract that turns post-KV subsystems such as Forge, Executioner, cache, hooks, docs, and federation into supervised runtime units with lifecycle, placement, capabilities, routes, health, and receipts.

This change defines the first implementation slice for runtime service core so host loading is no longer mistaken for a completed runtime.

## What Changes

- **Runtime service types**: Add portable service/application model types for service specs, service instances, lifecycle state, health, routes, placement, restart policy, capability bindings, and receipts.
- **Native built-in service registry**: Define the first service factory contract for linked Aspen services rather than dynamic native plugins.
- **Forge first slice**: Model Forge as the first native built-in runtime service without rewriting Forge internals.
- **Route and receipt contract**: Require runtime service startup, route registration, health transitions, and stop/failure paths to emit redacted receipts.

## In Scope

- Data-only runtime service model in or near `crates/aspen-runtime-core`.
- Built-in service registration/lifecycle trait boundaries.
- Forge service manifest/route/health/receipt wrapper as the first migration target.
- Tests and docs that distinguish host loading from runtime service orchestration.

## Out of Scope

- Dynamic third-party service marketplace.
- Full placement scheduler across all host types.
- Full Executioner/CI rename and durable workflow event-history implementation.
- Live process migration.
- Podman/Docker-style production container runtime.

## Verification

- `openspec validate define-runtime-service-core --strict`
- Runtime-core unit tests for pure service model invariants.
- Focused tests for built-in service manifest/route/receipt behavior.
- Docs/source-anchor tests proving `docs/runtime-applications.md` describes service core as future/active implementation rather than completed runtime.
- `git diff --check`
