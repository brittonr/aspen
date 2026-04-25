## ADDED Requirements

### Requirement: Extraction inventory tracks coordination family
The crate extraction inventory at `docs/crate-extraction.md` SHALL include the coordination family with current readiness state, manifest link, owner status, and next action for both `aspen-coordination` and `aspen-coordination-protocol`.

ID: architecture-modularity.extraction-inventory-tracks-coordination

#### Scenario: Inventory row exists for coordination
- **WHEN** `docs/crate-extraction.md` is read
- **THEN** the broader candidate inventory table SHALL include a row for the coordination family
- **AND** the row SHALL link to `docs/crate-extraction/coordination.md`
- **AND** the readiness state SHALL reflect the verified extraction-readiness status
