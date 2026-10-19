# Reviewed benchmark phase count delta

## ADDED Requirements

### Requirement: Public benchmark paths preflight reviewed phase groups

r[molten.wasm_performance.phase_group_count_preflight] `build_benchmark_run` MUST reject caller phase-group counts above the validated suite's reviewed three phases before normalizing or formatting any supplied phase groups. `validate_benchmark_run` MUST reject forged runs with more than the same existing `PERFORMANCE_PHASE_COUNT` before scanning phase groups or formatting canonical run identity. The bound MUST derive from the existing suite/profile contract, not a new arbitrary cap. Inputs at or below the bound MUST preserve existing normalized phase order, duplicate handling, invalid-shape error precedence, and result types.

#### Scenario: Exactly reviewed phase count remains eligible

- GIVEN a validated suite and exactly three valid phase groups supplied in noncanonical order
- WHEN the public producer constructs a benchmark run and its public validator checks it
- THEN the producer retains its existing canonical phase ordering, and validation succeeds.

#### Scenario: Admitted malformed, missing, and duplicate groups retain errors

- GIVEN at most three phase groups with a malformed event, duplicate phase/event, missing phase, or no groups
- WHEN the public producer or validator checks those admitted inputs
- THEN the producer retains its applicable missing-phase and malformed/duplicate diagnostics; the validator retains its applicable empty-run and malformed/duplicate diagnostics, rather than replacing any with a new phase-count denial.

#### Scenario: Existing input precedence applies before producer phase preflight

- GIVEN a valid reviewed suite and four supplied phase groups but a malformed workload identity
- WHEN the producer validates its run inputs
- THEN the existing workload-identity denial takes precedence over the newly guarded phase count.

#### Scenario: Fourth producer group is denied before normalization

- GIVEN a valid suite, materialization and host with four phase groups, including an extra duplicate
- WHEN the public producer constructs a run
- THEN it returns one reviewed suite phase-count denial before sorting samples, normalizing groups, or emitting per-group diagnostics.

#### Scenario: Fourth forged validator group is denied before formatting

- GIVEN a caller-built run with four phase groups and an invalid run identity
- WHEN the public run validator examines the run
- THEN it returns one reviewed suite phase-count denial before traversing the groups or formatting a replacement identity.
