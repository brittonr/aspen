# Project specification delta

## ADDED Requirements

### Requirement: Inherited Tracey debt has a complete classification inventory
r[molten.project.inherited_tracey_classification.inventory] Molten MUST classify every identifier in the reviewed inherited Tracey debt baseline in a deterministic inventory.

#### Scenario: Every baseline entry appears once
- GIVEN the reviewed baseline and accepted project specifications
- WHEN the classifier runs
- THEN every baseline identifier appears exactly once in the classification inventory.

### Requirement: Classification defaults remain conservative
r[molten.project.inherited_tracey_classification.conservative_default] Molten MUST classify a baseline entry as `accepted-implementation-unestablished` unless reviewed evidence establishes implementation, replacement, obsolescence, or invalidity.

#### Scenario: No evidence supports a stronger class
- GIVEN an accepted requirement without a direct source reference or lifecycle disposition
- WHEN the classifier emits its row
- THEN it uses the conservative class and makes no implementation claim.

### Requirement: Duplicate accepted definitions fail classification
r[molten.project.inherited_tracey_classification.duplicate_denial] Molten MUST reject classification when a baseline identifier has no accepted definition or has more than one accepted definition.

#### Scenario: Duplicate identifier is present
- GIVEN two accepted specifications define the same baseline identifier
- WHEN the classifier runs
- THEN it fails with both definition locations and does not write a passing inventory.

### Requirement: Classification output is grouped deterministically
r[molten.project.inherited_tracey_classification.deterministic_grouping] Molten MUST group classification rows by accepted specification path and source area with deterministic ordering.

#### Scenario: Input order changes
- GIVEN the same baseline and definitions in a different discovery order
- WHEN the classifier runs
- THEN the emitted inventory remains byte-identical.

### Requirement: Classification has positive and negative fixtures
r[molten.project.inherited_tracey_classification.fixtures] Molten MUST test valid inventory generation and reject missing definitions, duplicate definitions, malformed baselines, and foreign namespaces.

#### Scenario: Invalid classification input is tested
- GIVEN a duplicate accepted definition or malformed baseline
- WHEN the negative fixture runs
- THEN classification fails for the expected invariant.

### Requirement: Classification preserves non-claims
r[molten.project.inherited_tracey_classification.non_claims] Molten MUST state that classification does not establish implementation, replacement, obsolescence, invalidity, behavioral correctness, or release readiness.

#### Scenario: Operator reads the inventory
- GIVEN a passing classification report
- WHEN the operator reviews its meaning
- THEN the report states its bounded routing purpose and non-claims.

### Requirement: Proven inherited links use direct production evidence
r[molten.project.inherited_tracey_classification.verified_repair] Molten MUST remove an inherited debt entry only when existing production logic or documentation and a relevant test directly support the accepted requirement.

#### Scenario: High-confidence repair is applied
- GIVEN a requirement has matching production behavior and a positive or negative test
- WHEN its direct source markers are added
- THEN the exact baseline shrinks and the classifier reports only the remaining entries.
