## ADDED Requirements

### Requirement: Saga executor supports branch-backed step mode

The saga executor in the coordination/jobs layer SHALL accept a `branch_backed` flag per saga step definition. When enabled, the step SHALL execute inside a `BranchOverlay` wrapping the saga's `KeyValueStore`. This is an additional execution mode alongside the existing compensation-based model.

#### Scenario: Branch-backed step definition

- **WHEN** a saga definition includes a step with `branch_backed: true`
- **THEN** the saga executor SHALL create a `BranchOverlay` before executing the step
- **AND** the step's KV operations SHALL go through the overlay

#### Scenario: Compensation-backed step unchanged

- **WHEN** a saga definition includes a step with `branch_backed: false` (or unset)
- **THEN** the saga executor SHALL execute the step using the existing compensation model
- **AND** no `BranchOverlay` SHALL be created
