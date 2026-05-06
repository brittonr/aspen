## Phase 1: Spec foundation

- [x] Create proposal, design, tasks, and delta spec for runtime service core.

## Phase 2: Pure runtime service model

- [ ] Add portable runtime service/application model types for service specs, instances, lifecycle state, health, route declarations, placement hints, restart policy, capability bindings, resources, and redacted receipts.
- [ ] Add model invariant tests for service identity, lifecycle transitions, route ownership, restart policy bounds, and receipt redaction.

## Phase 3: Native built-in service boundary

- [ ] Define the linked native built-in service factory/manifest contract without introducing dynamic native plugin loading.
- [ ] Add tests proving built-in service declarations use `NativeBuiltIn` host loading and redacted capability handles.

## Phase 4: Forge first service slice

- [ ] Add a Forge runtime service wrapper that exposes manifest, route declarations, health, and lifecycle receipts while preserving current Forge internals.
- [ ] Add focused tests or source-anchor tests for Forge startup wiring, route registration, and secret-safe receipts.

## Phase 5: Documentation and validation

- [ ] Update `docs/runtime-applications.md` to distinguish implemented host-loading/model pieces from the active runtime-service-core implementation track.
- [ ] Run focused runtime-core/Forge/docs tests, strict OpenSpec validation, and whitespace checks.
