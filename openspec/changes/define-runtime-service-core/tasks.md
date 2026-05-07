## Phase 1: Spec foundation

- [x] Create proposal, design, tasks, and delta spec for runtime service core.

## Phase 2: Pure runtime service model

- [x] Add portable runtime service model types for service specs, instances, minimal application ownership references, lifecycle state, health, route declarations, placement hints, restart/upgrade policy, capability bindings, resources, and redacted receipts. ✅ 2m 29s (2026-05-07T00:16:30Z → 2026-05-07T00:18:59Z; evidence: `evidence/runtime-service-model-types.md`)
- [x] Add model invariant tests for service identity, lifecycle transitions, route ownership, restart/upgrade policy bounds, health receipts, host-loading-reference boundaries, and receipt redaction. ✅ 1m 27s (2026-05-07T00:19:21Z → 2026-05-07T00:20:48Z; evidence: `evidence/runtime-service-model-invariant-tests.md`)

## Phase 3: Native built-in service boundary

- [x] Define the linked native built-in service factory/manifest contract without introducing dynamic native plugin loading. ✅ 59s (2026-05-07T00:21:29Z → 2026-05-07T00:22:28Z; evidence: `evidence/native-built-in-contract.md`)
- [x] Add tests proving built-in service declarations use `NativeBuiltIn` host loading and redacted capability handles. ✅ 29s (2026-05-07T00:22:52Z → 2026-05-07T00:23:21Z; evidence: `evidence/native-built-in-redacted-handles.md`)

## Phase 4: Forge first service slice

- [ ] Add a Forge runtime service wrapper that exposes manifest, route declarations, health, and lifecycle receipts while preserving current Forge internals.
- [ ] Add focused tests or source-anchor tests for Forge startup wiring, route registration, and secret-safe receipts.

## Phase 5: Documentation and validation

- [ ] Update `docs/runtime-applications.md` to distinguish implemented host-loading/model pieces from the active runtime-service-core implementation track.
- [ ] Run focused runtime-core/Forge/docs tests, strict OpenSpec validation, and whitespace checks.
