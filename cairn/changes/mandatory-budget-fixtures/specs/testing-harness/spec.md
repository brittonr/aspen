## ADDED Requirements

### Requirement: Evidence-bearing suites require explicit budget fixtures
r[molten.testing.mandatory_budget.explicit_fixture] Evidence-bearing harness suites MUST include an explicit budget fixture or equivalent resource-policy proof refs. Omitted budget fixtures MUST NOT be normalized to default resource policy for execution, validation, or pass-evidence gates.

#### Scenario: Omitted budget fails execution
r[molten.testing.mandatory_budget.explicit_fixture.omitted]
- GIVEN a harness suite with explicit actor registry, explicit capabilities, actor steps, and no `<budget-v1 ...>` fixture
- WHEN the evidence-bearing local runner attempts to execute the suite
- THEN the runner rejects it before runtime turns, admission decisions, ambient effect requests, or report generation occur

#### Scenario: Explicit standard budget is valid
r[molten.testing.mandatory_budget.explicit_fixture.standard]
- GIVEN a harness suite with `<budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>`
- WHEN the suite stays within those limits
- THEN the budget fixture is explicit and may satisfy pass-evidence gates

#### Scenario: Explicit tight budget remains resource divergence
r[molten.testing.mandatory_budget.explicit_fixture.tight]
- GIVEN a harness suite with an explicit tight budget
- WHEN execution exceeds the declared resource limit
- THEN the runner reports deterministic `resource` divergence with expected, actual, and step diagnostics rather than treating the suite as malformed

### Requirement: Report validation rejects default resource policy
r[molten.testing.mandatory_budget.validation] Report validation MUST reject embedded suites that omitted explicit budget evidence, even if the report contains default `<budget-v1 ...>` evidence produced by an older runner.

#### Scenario: Legacy report with default budget fails validation
r[molten.testing.mandatory_budget.validation.legacy]
- GIVEN a report produced by an older runner whose embedded suite omitted the budget fixture
- WHEN `molten test report validate` evaluates the report
- THEN validation fails closed with missing explicit budget fixture diagnostics

#### Scenario: Budget usage still matches explicit evidence
r[molten.testing.mandatory_budget.validation.usage]
- GIVEN a report whose embedded suite declares an explicit budget fixture
- WHEN report usage counts differ from observations, effect-log entries, event counts, canonical report bytes, or declared limits
- THEN validation rejects the report with budget evidence diagnostics

### Requirement: Pass-evidence receipts prove no default resource policy
r[molten.testing.mandatory_budget.gate_checks] Successful pass-evidence gate receipts MUST include checks proving the accepted report used an explicit budget fixture and no default resource policy.

#### Scenario: Receipt includes explicit budget checks
r[molten.testing.mandatory_budget.gate_checks.receipt]
- GIVEN a deterministic report with an explicit budget fixture that validates and replays successfully
- WHEN `molten test gate check` emits a pass receipt
- THEN the receipt includes `explicit-budget-fixture` and `no-default-resource-policy` checks in addition to budget, actor-registry, capability, policy, admission, effect-log, and replay checks

### Requirement: Examples declare budgets
r[molten.testing.mandatory_budget.examples] Repository examples and positive harness tests MUST declare explicit budget fixtures. Negative resource tests MUST use explicit tight budgets, not omitted budgets, unless the test specifically targets omitted-budget failure.

#### Scenario: Two-actor example declares budget
r[molten.testing.mandatory_budget.examples.two_actor]
- GIVEN the repository two-actor example suite
- WHEN it is run through the harness and gated as pass evidence
- THEN the suite includes an explicit budget fixture covering step, effect, event, and report byte limits

### Requirement: Future resource policy evidence remains explicit
r[molten.testing.mandatory_budget.basalt_resource_policy] Future Nickel/Basalt resource policy integration MUST preserve the invariant that missing resource-policy evidence fails closed. Nickel policy snapshots, Basalt receipts, resource profiles, and budget refs MAY replace the first local static fixture, but they MUST be explicit and bound to run identity.

#### Scenario: Missing future resource proof fails closed
r[molten.testing.mandatory_budget.basalt_resource_policy.missing]
- GIVEN a future evidence-bearing report whose resource policy comes from a Nickel/Basalt proof ref
- WHEN that proof ref or receipt is omitted
- THEN validation rejects the report rather than treating missing resource policy as the default budget
