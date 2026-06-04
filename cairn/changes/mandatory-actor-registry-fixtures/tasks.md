## Phase 1: Mandatory registry enforcement

- [x] [serial] r[molten.testing.mandatory_actor_registry.explicit_fixture] Track whether suites provided an explicit actor registry fixture.
- [x] [serial] r[molten.testing.mandatory_actor_registry.no_inferred_execution] Reject evidence-bearing execution when the actor registry fixture is omitted.
- [x] [serial] r[molten.testing.mandatory_actor_registry.validation] Reject report validation when the embedded suite lacks explicit actor registry evidence.

## Phase 2: Executor boundary receipts

- [x] [serial] r[molten.testing.mandatory_actor_registry.executor_boundary] Keep actor executor kind selection fail-closed and prohibit unsupported-kind fallback to native execution.
- [x] [serial] r[molten.testing.mandatory_actor_registry.gate_checks] Add `explicit-actor-registry`, `no-inferred-actors`, and `executor-boundary` to pass-evidence gate receipts.

## Phase 3: Examples and tests

- [x] [serial] r[molten.testing.mandatory_actor_registry.examples] Ensure examples and positive tests declare explicit actor registries.
- [x] [serial] r[molten.testing.mandatory_actor_registry.negative_tests] Add negative coverage for omitted registries, explicit empty registries, unknown actors, and unsupported executor kinds.

## Phase 4: Future executor seam

- [x] [parallel] r[molten.testing.mandatory_actor_registry.future_executor_evidence] Document that future Steel, Wasm, adapter, and remote-proxy actors require explicit executor-boundary evidence before satisfying deterministic gates.
