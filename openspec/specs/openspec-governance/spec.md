## Purpose

Govern OpenSpec authoring, validation, and archival workflows so active changes remain reviewable, traceable, and deterministic before implementation and archive.

## Requirements

### Requirement: Delta specs are structurally consistent
OpenSpec delta specs SHALL be deterministic, ID-bearing, and consistent with main specs for modifications.

ID: openspec-governance.delta-spec-consistency

#### Scenario: Added requirements have IDs
ID: openspec-governance.delta-spec-consistency.added-requirements-have-ids

- **GIVEN** a delta spec adds a requirement or scenario
- **WHEN** validation runs
- **THEN** every added requirement and scenario SHALL include an `ID:` line

#### Scenario: Modified requirement exists
ID: openspec-governance.delta-spec-consistency.modified-requirement-exists

- **GIVEN** a delta spec modifies a requirement
- **WHEN** validation compares it with the main spec
- **THEN** the target requirement SHALL exist by ID or heading

#### Scenario: Missing modified target fails
ID: openspec-governance.delta-spec-consistency.missing-modified-target-fails

- **GIVEN** a modified requirement has no matching main spec requirement
- **WHEN** validation runs
- **THEN** it SHALL fail with the missing requirement identifier

#### Scenario: Removed requirement exists
ID: openspec-governance.delta-spec-consistency.removed-requirement-exists

- **GIVEN** a delta spec removes a requirement by ID or heading
- **WHEN** validation compares it with the main spec
- **THEN** the target requirement SHALL exist or validation SHALL fail with the missing removal target

#### Scenario: Migration note permits intentional repair
ID: openspec-governance.delta-spec-consistency.migration-note-permits-repair

- **GIVEN** a modified requirement repairs or migrates a legacy spec entry that is missing from the current main spec
- **WHEN** the delta includes an explicit `Migration note:` with rationale
- **THEN** validation MAY accept the modification while preserving the rationale in review output

#### Scenario: Conflicting feature contracts warn
ID: openspec-governance.delta-spec-consistency.conflicting-feature-contracts-warn

- **GIVEN** proposal, design, and delta specs use conflicting feature-contract language for the same capability
- **WHEN** validation runs
- **THEN** it SHALL emit a warning naming the conflicting artifacts and phrases
