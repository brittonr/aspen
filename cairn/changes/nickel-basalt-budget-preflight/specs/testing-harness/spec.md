# Testing Harness Delta: Nickel/Basalt budget preflight

### Requirement: Budget gates use Nickel resource-policy normalization
r[molten.testing.nickel_basalt_budget_preflight.nickel_resource_policy] Evidence-bearing reports MUST include budget-gate evidence with deterministic Nickel resource-policy source and export refs derived from the embedded suite budget fixture.

#### Scenario: Valid Nickel-normalized budget evidence
- GIVEN a suite with an explicit budget fixture
- WHEN the harness runs the suite
- THEN the report includes `<budget-gate-v1 ...>` with Nickel resource-policy source and export refs bound to the canonical budget ref
- AND report validation recomputes the same Nickel export evidence

#### Scenario: Tampered Nickel resource export fails validation
- GIVEN a report whose budget-gate Nickel export JSON or export ref has been modified
- WHEN report validation runs
- THEN validation fails closed before accepting the report as pass evidence

### Requirement: Budget gates include Basalt resource preflight receipts
r[molten.testing.nickel_basalt_budget_preflight.basalt_resource_receipt] Evidence-bearing budget gates MUST include a Basalt resource contract envelope and preflight receipt bound to the Nickel normalized source ref and canonical budget ref.

#### Scenario: Missing budget gate fails validation
- GIVEN a report whose embedded suite has an explicit budget fixture but whose report lacks `<budget-gate-v1 ...>`
- WHEN report validation runs
- THEN validation fails closed with a budget gate diagnostic

#### Scenario: Tampered Basalt resource receipt fails validation
- GIVEN a report whose Basalt resource preflight decision, reason, envelope ref, budget ref, or normalized source ref has been modified
- WHEN report validation runs
- THEN validation rejects the report rather than trusting local budget limits alone

### Requirement: Budget usage remains bound to limits and report bytes
r[molten.testing.nickel_basalt_budget_preflight.usage_binding] Budget evidence MUST still record actual usage and validation MUST check usage against observations, effect logs, canonical report bytes, and declared limits.

#### Scenario: Usage within preflighted limits passes
- GIVEN a deterministic report with explicit budget, budget gate, observations, effect log, and canonical report bytes within limits
- WHEN validation runs
- THEN the budget usage binding check passes

#### Scenario: Usage over limits fails closed
- GIVEN a suite whose steps, effects, events, or report bytes exceed explicit limits
- WHEN the harness runs or validation checks report evidence
- THEN execution or validation fails as resource divergence or invalid budget usage

### Requirement: Gate receipts expose resource preflight refs
r[molten.testing.nickel_basalt_budget_preflight.gate_receipts] Successful pass-evidence gate receipts MUST include checks and artifact refs for resource policy preflight, Nickel resource policy/export, Basalt resource receipt, and budget usage binding.

#### Scenario: Successful gate receipt includes resource checks
- GIVEN a deterministic report that validates and replays successfully
- WHEN `molten test gate check` emits a receipt
- THEN the receipt includes `resource-policy-preflight`, `nickel-resource-policy`, `nickel-resource-export`, `basalt-resource-receipt`, and `budget-usage-binding` checks
