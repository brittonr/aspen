# project Specification Delta

## ADDED Requirements

### Requirement: Inherited Tracey debt has an exact baseline
r[molten.project.inherited_tracey_debt.baseline] Molten MUST store the sorted inherited uncovered requirement set with typed metadata, an exact count, and a BLAKE3 digest.

#### Scenario: Baseline matches the source tree
- GIVEN the accepted requirements, admitted evidence roots, and checked-in baseline
- WHEN the inherited Tracey debt guard evaluates them
- THEN the actual uncovered set equals the baseline and its typed identity.

### Requirement: Verified marker defects are repaired directly
r[molten.project.inherited_tracey_debt.marker_repair] Molten MUST repair malformed or stale requirement markers only when accepted requirement text and existing source evidence establish the exact identity.

#### Scenario: An inline requirement marker is not silently omitted
- GIVEN an accepted requirement with verified source evidence
- WHEN the marker placement prevents requirement discovery
- THEN the marker moves to the accepted standalone form without changing requirement semantics.

### Requirement: Traceability growth fails closed
r[molten.project.inherited_tracey_debt.growth_denial] Molten MUST deny new uncovered requirements, dangling evidence references, malformed baselines, duplicate entries, unsorted entries, and unreviewed baseline reductions.

#### Scenario: A new uncovered requirement appears
- GIVEN a source tree with one requirement absent from the reviewed baseline and no evidence reference
- WHEN the debt guard evaluates the tree
- THEN it fails and identifies the unexpected requirement.

### Requirement: The debt guard has positive and negative tests
r[molten.project.inherited_tracey_debt.fixtures] Molten MUST test exact baseline admission and MUST test malformed markers, duplicate baselines, unsorted baselines, new gaps, stale gaps, and dangling references.

#### Scenario: Exact baseline passes
- GIVEN a valid requirement set, valid evidence references, and an exact sorted baseline
- WHEN the self-test evaluates the inputs
- THEN admission passes.

#### Scenario: Traceability drift fails
- GIVEN a new gap, stale gap, duplicate baseline, unsorted baseline, malformed marker, or dangling reference
- WHEN the self-test evaluates the inputs
- THEN admission fails with the expected diagnostic class.

### Requirement: Validation evidence is reproducible
r[molten.project.inherited_tracey_debt.validation] Molten MUST record the scanner profile, source revision, baseline identity, focused commands, and final lifecycle receipts.

#### Scenario: A reviewer repeats validation
- GIVEN the archived validation evidence and source revision
- WHEN the reviewer runs the recorded commands
- THEN the guard, metadata, lifecycle, and repository results can be compared with the recorded identities.

### Requirement: The debt baseline does not claim coverage
r[molten.project.inherited_tracey_debt.non_claims] Molten MUST state that the baseline does not exempt uncovered requirements or prove marker truth, behavior, release readiness, or whole-system correctness.

#### Scenario: Baseline admission passes
- GIVEN the actual inherited uncovered set equals the baseline
- WHEN the guard emits a passing result
- THEN the result states that inherited requirements remain uncovered and require direct evidence.
