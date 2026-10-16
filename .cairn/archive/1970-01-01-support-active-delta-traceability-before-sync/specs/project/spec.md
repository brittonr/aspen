## MODIFIED Requirements

### Requirement: Traceability growth fails closed

r[molten.project.inherited_tracey_debt.growth_denial] Molten MUST deny new uncovered accepted requirements, evidence references absent from both accepted requirements and admitted active delta specifications, malformed baselines, duplicate entries, unsorted entries, and unreviewed baseline reductions. Recognizing a pending reference MUST NOT add its proposed requirement to the accepted coverage denominator or inherited debt.

#### Scenario: A new uncovered requirement appears

- GIVEN a source tree with one accepted requirement absent from the reviewed baseline and no evidence reference
- WHEN the inherited debt guard evaluates the tree
- THEN it fails and identifies the unexpected accepted requirement, even if active deltas declare unrelated pending identifiers.

## ADDED Requirements

### Requirement: Pending references are separate from accepted coverage

r[molten.project.inherited_tracey_debt.active_delta_scope] Molten MUST use accepted requirements exclusively for accepted reference counts, uncovered requirements, and exact inherited-baseline comparison. It MAY resolve an otherwise unknown reference through an admitted active delta identifier and MUST classify that reference as pending rather than accepted coverage.

#### Scenario: Pending implementation validates before sync

- GIVEN an accepted requirement set whose uncovered set exactly matches the reviewed baseline and a source reference declared only in an active delta
- WHEN the inherited debt guard evaluates the tree
- THEN that reference resolves as pending and does not make the otherwise exact baseline fail.
- AND accepted coverage and uncovered counts are unchanged by adding the proposed requirement.

#### Scenario: Accepted and pending identity overlap

- GIVEN an identifier appears in both an accepted requirement and a MODIFIED active requirement
- WHEN source references that identifier
- THEN it counts once as an accepted reference and not as pending coverage.

### Requirement: Only native active delta definitions admit pending identities

r[molten.project.inherited_tracey_debt.active_delta_admission] Pending identity discovery MUST be restricted to `.cairn/changes/<change>/specs/**/spec.md` native delta files and valid requirement markers under ADDED or MODIFIED requirement sections. Removed requirements, rename labels, archives, proposals, designs, tasks, evidence, and unrelated Markdown MUST NOT admit pending identities. Native Cairn package validation remains separately required.

#### Scenario: Non-delta text cannot suppress an unknown reference

- GIVEN a source identifier appears only in an archive, proposal, task, or unrelated document
- WHEN the inherited debt guard evaluates that reference
- THEN the identifier remains dangling and normal execution and baseline-write mode both deny.

#### Scenario: Removed pending identity becomes dangling

- GIVEN an unaccepted identifier is declared only under REMOVED requirements or is removed from all active ADDED/MODIFIED delta definitions
- WHEN source still references that identifier
- THEN the reference is dangling and the guard denies it.

### Requirement: Pending resolution does not authorize lifecycle acceptance

r[molten.project.inherited_tracey_debt.active_delta_non_claims] The guard MUST report pending references separately and state that identity resolution establishes neither accepted coverage, marker truth, package validity, runtime behavior, sync eligibility, archive eligibility, nor release readiness. Existing all-task and passing-gate lifecycle requirements MUST remain unchanged.

#### Scenario: Incomplete package remains incomplete

- GIVEN an active package has pending references that resolve successfully but required implementation or validation tasks remain unfinished
- WHEN source identity validation passes
- THEN no requirement is treated as accepted and the package gains no sync/archive authority.

### Requirement: Pre-sync traceability validation is reproducible

r[molten.project.inherited_tracey_debt.active_delta_validation] Molten MUST retain positive and negative behavioral coverage for pending resolution, accepted overlap, unchanged accepted debt, unknown/removed identities, and non-delta exclusion; exercise the actual guard CLI and repository check; and record focused formatting, native Cairn gate results, commands, ownership, and non-claims before acceptance.

#### Scenario: A reviewer reproduces the boundary

- GIVEN the checked-in guard, tests, and recorded owned fixture commands
- WHEN a reviewer executes the focused validation
- THEN the pending-positive case succeeds while unknown identities and accepted debt drift fail for their observed diagnostic classes.
- AND unrelated lifecycle or provider blockers are preserved rather than waived.
