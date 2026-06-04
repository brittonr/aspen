## ADDED Requirements

### Requirement: Admission evidence records are mandatory
r[molten.testing.admission_evidence.records] Evidence-bearing harness reports MUST record exactly one canonical admission decision event for every step observation. The admission decision event MUST precede semantic runtime trace records, effect request/response records, rollback records, or output records for that step.

#### Scenario: Missing admission decision fails validation
r[molten.testing.admission_evidence.records.missing]
- GIVEN a harness report whose observation contains committed trace records but no admission decision event
- WHEN `molten test report validate` evaluates the report
- THEN validation fails closed with canonical failure evidence rather than treating the step as implicitly allowed

#### Scenario: Duplicate admission decisions fail validation
r[molten.testing.admission_evidence.records.duplicate]
- GIVEN a harness report whose observation contains two admission decision events
- WHEN the report is validated or gated as pass evidence
- THEN the validator rejects the report as malformed admission evidence

### Requirement: Admission requests are bound to suite steps
r[molten.testing.admission_evidence.step_binding] The validator MUST derive the expected admission request from the embedded suite step and MUST compare the recorded admission request against that derived request using canonical Preserves equality.

#### Scenario: Tampered admission request fails validation
r[molten.testing.admission_evidence.step_binding.tampered]
- GIVEN a suite step `<send "producer" "consumer" "hello">`
- WHEN the report records an admission request for a different actor, action, target, value, or effect metadata
- THEN validation rejects the report before accepting any committed trace or effect records for that step

### Requirement: Admission decisions are recomputed from embedded policy
r[molten.testing.admission_evidence.policy_recompute] The validator MUST recompute each admission decision from the embedded static policy fixture or policy refs and MUST reject reports whose recorded allow/deny decision or reason does not match the recomputed decision.

#### Scenario: Tampered deny-to-allow fails validation
r[molten.testing.admission_evidence.policy_recompute.deny_to_allow]
- GIVEN a suite policy that denies an assertion by a producer
- WHEN a report records that step as allowed
- THEN validation fails with an admission decision mismatch before considering the report pass evidence

#### Scenario: Stale policy fixture fails validation
r[molten.testing.admission_evidence.policy_recompute.stale_policy]
- GIVEN a report whose embedded policy fixture does not match the admission decisions recorded in its observations
- WHEN the report is validated or replayed
- THEN the validator rejects the stale report rather than trusting the previous runner output

### Requirement: Denied turns do not commit semantic actions
r[molten.testing.admission_evidence.deny_rollback] A denied non-effect turn MUST roll back before committing semantic runtime actions. Validation MUST reject a denied observation that contains committed message delivery, assertion commit, assertion retraction, observation side effects, storage mutation, adapter mutation, or any other committed action evidence after the denial.

#### Scenario: Denied assertion cannot commit
r[molten.testing.admission_evidence.deny_rollback.assertion]
- GIVEN a suite policy that denies `<assert "producer" "service.ready">`
- WHEN a report contains both a deny decision and an `<assertion-committed ...>` event for that step
- THEN validation rejects the report with denied-commit diagnostics

#### Scenario: Denied send cannot deliver
r[molten.testing.admission_evidence.deny_rollback.send]
- GIVEN a suite policy that denies a send from one actor to another
- WHEN a report contains both a deny decision and a message-delivered trace for that step
- THEN validation rejects the report and the pass-evidence gate refuses it

### Requirement: Denied effects are suppressed
r[molten.testing.admission_evidence.denied_effect_suppression] A denied effect step MUST NOT issue ambient effect request or effect response records. Validation MUST reject denied clock, random, storage, network, filesystem, process, blob, or adapter-effect steps that contain effect request/response evidence.

#### Scenario: Denied clock has no effect response
r[molten.testing.admission_evidence.denied_effect_suppression.clock]
- GIVEN a suite policy that denies a clock request
- WHEN a report records an effect request or response for that denied clock step
- THEN validation fails because the ambient observation crossed the effect boundary after denial

### Requirement: Admission divergence is first-class replay evidence
r[molten.testing.admission_evidence.policy_divergence] Deterministic replay MUST classify mismatches at the admission decision boundary as `policy-decision` divergence and MUST report the first divergent step before downstream trace, effect, output, or state-hash differences.

#### Scenario: Replay stops at changed admission decision
r[molten.testing.admission_evidence.policy_divergence.changed_decision]
- GIVEN a recorded report with an admission decision event
- WHEN replay recomputes a different request or decision for the same step
- THEN replay fails with `policy-decision` divergence at that step and includes expected/actual admission evidence refs or diagnostics

### Requirement: Gate receipts include admission checks
r[molten.testing.admission_evidence.gate_checks] Successful pass-evidence gate receipts MUST include explicit checks for admission policy schema/support, per-step admission decision validation, denied turn rollback, and denied effect suppression.

#### Scenario: Gate receipt lists admission checks
r[molten.testing.admission_evidence.gate_checks.receipt]
- GIVEN a deterministic harness report that validates and replays successfully
- WHEN `molten test gate check` emits a gate receipt
- THEN the receipt includes passed checks named `admission-policy`, `admission-decisions`, `deny-rollback`, and `denied-effect-suppression` in addition to schema, effect-log, budget, actor-registry, and deterministic-replay checks

### Requirement: Static policy fixtures remain declarative and replaceable
r[molten.testing.admission_evidence.static_policy_boundary] The initial static Preserves policy fixture MUST remain declarative, canonical, and side-effect free, and the validator MUST be structured so Nickel contracts, Basalt/UCAN authority context, and reviewed Steel predicates can augment or replace the fixture without removing fail-closed admission evidence validation.

#### Scenario: Dynamic predicate cannot bypass admission evidence
r[molten.testing.admission_evidence.static_policy_boundary.dynamic_predicate]
- GIVEN a future reviewed Steel predicate participates in an admission decision
- WHEN the decision is recorded in a harness report
- THEN the report still contains canonical admission request/decision evidence plus predicate receipt refs, and validation fails closed if either the decision or predicate evidence is missing
