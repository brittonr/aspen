## ADDED Requirements

### Requirement: Deterministic drift comparison core
r[molten.testing.deterministic_drift.comparison_core] Molten MUST provide a pure deterministic drift comparator that accepts paired workflow evidence summaries, canonical receipt or report refs, and explicit allowed-variance declarations, then returns a pass or deny result with first-drift diagnostics.

#### Scenario: Equal canonical evidence passes drift comparison
- GIVEN two evidence summaries produced from the same declared deterministic inputs
- WHEN the drift comparator evaluates their canonical refs and normalized values
- THEN comparison passes only if all semantic refs and normalized canonical values match.

#### Scenario: Unexplained ref drift fails closed
- GIVEN two evidence summaries from the same declared inputs with different report or receipt refs
- WHEN no allowed-variance declaration accounts for the difference
- THEN comparison fails closed with a diagnostic naming the first differing ref or field.

### Requirement: Allowed variance is explicit and canonical
r[molten.testing.deterministic_drift.variance_declarations] Deterministic drift checks MUST allow volatile fields only when each variance is explicitly declared, justified by a reason class, and removed or normalized through canonical comparison rules before equality is evaluated.

#### Scenario: Declared volatile field is normalized
- GIVEN two workflow evidence summaries that differ only in a declared non-semantic volatile field
- WHEN the drift comparator applies the allowed-variance declaration
- THEN the normalized semantic evidence matches and the comparison may pass.

#### Scenario: Undeclared volatile field fails comparison
- GIVEN two workflow evidence summaries that differ in an undeclared field
- WHEN drift comparison runs
- THEN the comparison fails closed even if the field appears incidental in rendered logs.

### Requirement: Fresh rerun drift gate
r[molten.testing.deterministic_drift.fresh_rerun_gate] Molten MUST provide an explicit drift gate that runs selected evidence-bearing workflows in fresh isolated state roots with the same declared inputs and compares their canonical evidence through the drift comparator.

#### Scenario: Same workflow rerun produces same evidence
- GIVEN a deterministic evidence-bearing workflow and a declared input set
- WHEN the drift gate runs the workflow in separate fresh state roots
- THEN the gate compares canonical output refs from each run and passes only if semantic evidence is identical after declared normalization.

#### Scenario: Ambient state drift is denied
- GIVEN a workflow that reads undeclared ambient state and changes canonical evidence between fresh runs
- WHEN the drift gate compares the outputs
- THEN the gate fails closed with an ambient-state or unexplained-drift diagnostic.

### Requirement: Release workflows are covered by drift checks
r[molten.testing.deterministic_drift.release_workflows] Molten SHOULD cover dogfood local-node, sealed repro verify/unpack, release bundle verify, release promotion, release export verification, and deterministic VM child evidence with drift checks where those workflows claim deterministic evidence.

#### Scenario: Dogfood evidence is stable across fresh roots
- GIVEN the same source tree and declared dogfood inputs
- WHEN the drift gate runs dogfood local-node twice in fresh state roots
- THEN release-gate, replay-verify, replay-index, bundle-verify, promotion, and export-verify semantic evidence refs match or fail with declared variance diagnostics.

### Requirement: Drift gate has positive and negative fixtures
r[molten.testing.deterministic_drift.negative_fixtures] Molten SHOULD test deterministic drift validation with positive same-input/same-ref fixtures and negative fixtures for injected ref drift, undeclared volatile fields, ambient state use, unstable map ordering, and rendered-output-only changes.

#### Scenario: Injected drift fixture is rejected
- GIVEN a fixture pair whose second evidence summary has a changed canonical child ref
- WHEN the drift comparator evaluates the pair
- THEN validation fails closed with a first-drift diagnostic before accepting the workflow as deterministic evidence.

### Requirement: Drift gate has an explicit validation surface
r[molten.testing.deterministic_drift.gate_surface] Molten SHOULD expose deterministic drift validation through an explicit Nix check, app, or release-readiness command. The gate MUST NOT treat retry success as proof that drift was absent.

#### Scenario: Retry does not mask drift
- GIVEN a workflow that alternates between two canonical evidence refs across runs
- WHEN the drift validation surface is invoked
- THEN the gate reports drift instead of retrying until two matching outputs appear.

### Requirement: Drift workflow is documented
r[molten.testing.deterministic_drift.docs] User-facing documentation SHOULD describe which workflows are compared, what refs are authoritative, how allowed variance is declared, and how to diagnose first-drift failures.

#### Scenario: Operator diagnoses a drift failure
- GIVEN a drift gate failure in release evidence review
- WHEN an operator follows the documented workflow
- THEN they can identify the first differing canonical ref, the workflow step that emitted it, and whether a variance declaration or code fix is required.
