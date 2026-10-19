# Public resource lifecycle count admission

## ADDED Requirements

### Requirement: Placement bounds caller-built constraint intake

r[molten.resources.placement_constraint_preflight] The public `evaluate_placement` function MUST reject requests containing more than the existing `MAX_CONSTRAINTS = 64` placement constraints before allocating or formatting constraint diagnostics or cloning target taints. It MUST retain the `PlacementDecision` shape and existing placement decisions and validation precedence for admitted requests, including exactly 64 constraints.

#### Scenario: Exactly 64 constraints remain eligible

- GIVEN 64 preferred placement constraints
- WHEN the caller evaluates placement
- THEN the result passes without diagnostics and retains its target.

#### Scenario: Admitted required constraint still denies for its original reason

- GIVEN 64 constraints including an unsatisfied required one
- WHEN the caller evaluates placement
- THEN the result denies with the existing required-constraint diagnostic, not a count denial.

#### Scenario: One extra constraint is denied before formatting

- GIVEN 65 constraints including an unsatisfied required one
- WHEN the caller evaluates placement
- THEN the result denies with one `constraint count 65 exceeds maximum 64` diagnostic before formatting the unsatisfied constraint.

### Requirement: Reconciliation completion bounds required effect intents

r[molten.resources.reconcile_required_effect_intent_preflight] The public `validate_reconcile_completion` function MUST reject more than the existing `MAX_EFFECT_INTENTS = 128` required effect intents before reserving or formatting per-effect diagnostics. It MUST retain the `ReconcileCompletionDecision` shape, exactly 128 intent eligibility, and generation, admitted-plan, effect, and status diagnostic precedence for admitted input.

#### Scenario: Exactly 128 receipt-backed effects pass

- GIVEN 128 required effect intents all backed by receipts with matching generation, admitted plan, and status update
- WHEN the caller validates completion
- THEN the result passes without diagnostics.

#### Scenario: Admitted invalid completion preserves diagnostic order

- GIVEN 128 required intents with no receipts and a stale generation
- WHEN the caller validates completion
- THEN stale generation appears before the bounded missing-effect diagnostics.

#### Scenario: One extra required intent fails before per-effect formatting

- GIVEN 129 required intents, a stale generation, and no effect receipts
- WHEN the caller validates completion
- THEN it denies with one `required effect intent count 129 exceeds maximum 128` diagnostic.

### Requirement: Explicit target property sizing remains a separate owner contract

r[molten.resources.placement_target_property_contract_missing] This count-only change MUST NOT claim a cardinality or byte-size policy for caller-supplied `explicit_target_properties`; that input requires a separate owner-selected contract before a bounded-target-properties claim can be made.

#### Scenario: Constraint count admission is not target property admission

- GIVEN a placement request within the 64-constraint limit and caller-supplied target properties
- WHEN the caller evaluates placement
- THEN this change only guarantees the constraint-count boundary; it does not claim a target-property count or byte limit.
