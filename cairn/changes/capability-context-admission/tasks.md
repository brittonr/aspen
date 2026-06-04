## Phase 1: Capability fixture and request binding

- [x] [serial] r[molten.testing.capability_context.fixture_schema] Define canonical `<capabilities-v1 ...>` suite fixture parsing and rendering helpers.
- [x] [serial] r[molten.testing.capability_context.context_ref] Normalize explicit capability fixtures into deterministic capability context refs and track omitted fixtures separately for fail-closed mandatory-fixture gates.
- [x] [serial] r[molten.testing.capability_context.request_binding] Bind each admission request to the capability context and matching grant or denial evidence.

## Phase 2: Authorization semantics

- [x] [serial] r[molten.testing.capability_context.deny_by_default] Make local harness authorization deny by default when no matching grant exists.
- [x] [serial] r[molten.testing.capability_context.policy_composition] Compose capability authorization with static policy so either missing authority or policy denial denies the step before side effects.
- [x] [serial] r[molten.testing.capability_context.effect_authority] Require explicit capability grants for clock/random and future ambient effect steps before effect request records can be emitted.

## Phase 3: Fail-closed validation and replay

- [x] [serial] r[molten.testing.capability_context.validation] Recompute capability authorization during report validation and reject stale, missing, malformed, or tampered capability evidence.
- [x] [serial] r[molten.testing.capability_context.capability_divergence] Classify replay mismatches at the authority boundary as `capability-decision` or equivalent first-divergence detail before downstream trace/effect/state drift.
- [x] [parallel] r[molten.testing.capability_context.failure_artifacts] Emit canonical failure diagnostics for missing grants, stale capability refs, and unauthorized effect requests.

## Phase 4: Gate receipts and tests

- [x] [serial] r[molten.testing.capability_context.gate_receipts] Add `capability-context`, `capability-grants`, `deny-without-capability`, and `authority-ref-binding` to pass-evidence gate receipts.
- [x] [serial] r[molten.testing.capability_context.negative_tests] Add negative suites for send without grant, effect without grant, tampered grant, stale capability ref, and denied effect with response.
- [x] [parallel] r[molten.testing.capability_context.basalt_ucan_path] Document and structure the fixture so Basalt/UCAN proof refs can replace the local grant matcher without removing fail-closed validation.
