## ADDED Requirements

### Requirement: Policy preflight is required before side effects
r[molten.testing.policy_boundary.preflight_receipt] Evidence-bearing harness runs MUST perform static policy preflight before any runtime turn, semantic commit, or ambient effect request can execute. The run report MUST include canonical policy preflight evidence bound to the policy used for admission decisions.

#### Scenario: Report contains policy gate evidence
r[molten.testing.policy_boundary.preflight_receipt.report]
- GIVEN a deterministic harness suite with no explicit policy fixture or with a static deny-rule policy fixture
- WHEN the local harness executes the suite
- THEN the report contains a canonical `<policy-gate-v1 "molten.harness.policy-gate.v1" ...>` record before step observations are accepted as pass evidence

#### Scenario: Missing policy gate fails validation
r[molten.testing.policy_boundary.preflight_receipt.missing]
- GIVEN a harness report whose observations contain admission decisions but whose report lacks policy preflight evidence
- WHEN `molten test report validate` evaluates the report
- THEN validation fails closed before accepting the admission decisions as pass evidence

### Requirement: Policy snapshots are canonical and bound to suites
r[molten.testing.policy_boundary.policy_snapshot] The policy preflight gate MUST reference a canonical policy snapshot ref derived from the embedded suite policy. Omitted policies MUST normalize to an explicit allow-all policy snapshot, and explicit policies MUST normalize to canonical Preserves values whose refs are checked during report validation.

#### Scenario: Stale policy ref fails validation
r[molten.testing.policy_boundary.policy_snapshot.stale]
- GIVEN a report whose embedded suite policy or policy gate ref has been tampered after execution
- WHEN the report is validated or gated
- THEN validation rejects the report because the policy gate ref no longer matches the embedded policy snapshot

### Requirement: Nickel static boundary is explicit
r[molten.testing.policy_boundary.nickel_static] Static declarative policy/config/schema gates MUST be represented as Nickel-compatible static boundary evidence. Until the Nickel evaluator is fully integrated, the local harness MUST mark the current Preserves deny-rule fixture as a static Nickel-compatible subset rather than treating parser success as sufficient evidence.

#### Scenario: Static subset is identified
r[molten.testing.policy_boundary.nickel_static.marker]
- GIVEN a local harness report using the current Preserves deny-rule policy fixture
- WHEN a pass-evidence gate inspects the policy preflight evidence
- THEN the evidence identifies the static engine, Nickel-compatible contract marker, canonical policy snapshot ref, and static-boundary check result

### Requirement: Basalt policy gate evidence is explicit
r[molten.testing.policy_boundary.basalt_gate] Policy preflight decisions MUST be represented as Basalt gate evidence or a local harness Basalt-preflight marker until real Basalt/UCAN context refs are integrated. Missing, unsupported, or stale Basalt policy gate evidence MUST fail closed.

#### Scenario: Gate receipt lists Basalt policy check
r[molten.testing.policy_boundary.basalt_gate.receipt]
- GIVEN a deterministic report that validates, replays, and passes policy preflight
- WHEN `molten test gate check` emits a pass receipt
- THEN the receipt includes a `basalt-policy-gate` check and artifact refs for the policy snapshot and policy gate evidence

### Requirement: Steel predicates require reviewed callable receipts
r[molten.testing.policy_boundary.steel_review] Steel predicates, dynamic predicates, or trusted callables MUST NOT be accepted as ordinary static policy data. Any policy that references Steel/dynamic predicates MUST include reviewed callable receipt evidence before it can participate in admission; until that review path exists, local harness policy fixtures MUST reject such predicates fail-closed.

#### Scenario: Unreviewed Steel predicate is rejected
r[molten.testing.policy_boundary.steel_review.unreviewed]
- GIVEN a suite policy fixture containing an unreviewed `<steel-predicate ...>` or `<dynamic-predicate ...>` record
- WHEN the harness parses or preflights the suite
- THEN the suite is rejected before runtime execution and no side-effect-bearing report is produced

### Requirement: Gate receipts include policy boundary checks
r[molten.testing.policy_boundary.gate_receipts] Successful pass-evidence gate receipts MUST include checks for policy preflight, Nickel static policy boundary, Basalt policy gate evidence, and Steel predicate review in addition to admission-decision, deny-rollback, denied-effect-suppression, budget, actor-registry, effect-log, report-schema, and deterministic replay checks.

#### Scenario: Policy boundary checks are receipt evidence
r[molten.testing.policy_boundary.gate_receipts.checks]
- GIVEN a valid deterministic harness report
- WHEN the gate accepts it as pass evidence
- THEN the canonical gate receipt lists `policy-preflight`, `nickel-static-policy`, `basalt-policy-gate`, and `steel-predicate-review` as passed checks
