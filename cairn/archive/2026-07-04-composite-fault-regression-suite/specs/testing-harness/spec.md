## ADDED Requirements

### Requirement: Composite distributed fault regressions are named and bounded
r[molten.testing.distributed_simulation.composite_fault_regression_suite] Molten SHOULD maintain a named deterministic composite fault regression suite for high-value distributed interleavings, including duplicate-after-restart, partition-with-stale-evidence, reorder-with-reconcile, crash-during-dispatch, and resource-pressure-during-quorum cases.

#### Scenario: Composite case binds deterministic inputs
- GIVEN a named composite fault case
- WHEN the simulation run receipt is emitted
- THEN it binds the case id, seed ref, topology ref, scheduler ref, fault-plan ref, command refs, invariant name, event refs, final-state ref, replay ref, diagnostics, and evidence-only caveats.

#### Scenario: Composite denial preserves no-side-effect evidence
- GIVEN a composite fault case expected to deny before side effects
- WHEN the simulation evaluates the case
- THEN denied operation ids, denial diagnostics, and final-state refs show that no semantic commit was accepted for the invalid operation.

### Requirement: Generated interleaving failures have promotion and budget evidence
r[molten.testing.distributed_simulation.generated_case_promotion_budget] Molten MUST require explicit promotion metadata, traceability coverage, profile eligibility, retry policy, variance declarations, and cost budget before a generated distributed case becomes a named regression fixture or release-review claim.

#### Scenario: Generated failure is promoted with stable refs
- GIVEN a generated distributed failure with stable seed, topology, scheduler, fault-plan, command, invariant, replay, and diagnostic refs
- WHEN it is promoted to a named regression fixture
- THEN the promotion evidence binds those refs and adds positive or negative traceability coverage for the new invariant.

#### Scenario: Retry-only success cannot satisfy composite pass evidence
- GIVEN a composite or generated case that only passes after a retry or undeclared variance
- WHEN the distributed evidence gate evaluates the case
- THEN the gate rejects it as deterministic pass evidence
- AND diagnostics identify retry-only or undeclared-variance status.
