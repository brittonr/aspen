## Phase 1: Mandatory fixture enforcement

- [x] [serial] r[molten.testing.mandatory_capabilities.explicit_fixture] Track whether suites provided an explicit capability fixture.
- [x] [serial] r[molten.testing.mandatory_capabilities.explicit_fixture] Reject evidence-bearing execution when the capability fixture is omitted.
- [x] [serial] r[molten.testing.mandatory_capabilities.validation] Reject report validation when the embedded suite lacks explicit capability evidence.

## Phase 2: Receipts and examples

- [x] [serial] r[molten.testing.mandatory_capabilities.gate_checks] Add `explicit-capability-fixture` and `no-implicit-authority` to pass-evidence gate receipts.
- [x] [serial] r[molten.testing.mandatory_capabilities.examples] Update example and positive tests to use explicit least-privilege grants.
- [x] [serial] r[molten.testing.mandatory_capabilities.explicit_fixture.empty] Add negative coverage for omitted fixtures and keep explicit empty fixtures as valid deny-by-default authority contexts.

## Phase 3: Future authority seam

- [x] [parallel] r[molten.testing.mandatory_capabilities.basalt_ucan_invariant] Document that future Basalt/UCAN proof bundles must preserve the no-implicit-authority invariant.
