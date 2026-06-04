## Phase 1: Mandatory budget enforcement

- [x] [serial] r[molten.testing.mandatory_budget.explicit_fixture] Track whether suites provided an explicit budget fixture.
- [x] [serial] r[molten.testing.mandatory_budget.no_default_execution] Reject evidence-bearing execution when the budget fixture is omitted.
- [x] [serial] r[molten.testing.mandatory_budget.validation] Reject report validation when the embedded suite lacks explicit budget evidence.

## Phase 2: Receipts and examples

- [x] [serial] r[molten.testing.mandatory_budget.gate_checks] Add `explicit-budget-fixture` and `no-default-resource-policy` to pass-evidence gate receipts.
- [x] [serial] r[molten.testing.mandatory_budget.examples] Ensure examples and positive tests declare explicit budget fixtures.
- [x] [serial] r[molten.testing.mandatory_budget.negative_tests] Add negative coverage for omitted budgets and legacy default-budget reports while keeping explicit tight budgets as resource divergence evidence.

## Phase 3: Future policy seam

- [x] [parallel] r[molten.testing.mandatory_budget.basalt_resource_policy] Document that future Nickel/Basalt resource policy refs must preserve the no-default-resource-policy invariant.
