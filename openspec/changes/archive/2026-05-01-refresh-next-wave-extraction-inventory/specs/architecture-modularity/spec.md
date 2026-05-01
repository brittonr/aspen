## ADDED Requirements

### Requirement: Extraction inventory tracks completed next-wave evidence [r[architecture-modularity.extraction-inventory-tracks-next-wave-evidence]]
The crate extraction inventory MUST keep rows for implemented next-wave families synchronized with the family manifests and archived evidence.

ID: architecture-modularity.extraction-inventory-tracks-next-wave-evidence

#### Scenario: Completed family manifests are linked from inventory [r[architecture-modularity.extraction-inventory-tracks-next-wave-evidence.manifest-links]]
- GIVEN a next-wave family has a committed manifest under `docs/crate-extraction/`
- WHEN the broader candidate inventory is reviewed
- THEN its inventory row MUST link to that manifest
- AND the row MUST name the current owner group rather than `owner needed` when the manifest has an owner.

ID: architecture-modularity.extraction-inventory-tracks-next-wave-evidence.manifest-links

#### Scenario: Completed blockers are not repeated as next actions [r[architecture-modularity.extraction-inventory-tracks-next-wave-evidence.current-next-actions]]
- GIVEN archived evidence records a completed first blocker for a next-wave family
- WHEN the broader candidate inventory states that family's next action
- THEN it MUST describe the current remaining blocker or policy hold
- AND it MUST NOT ask future contributors to redo completed first-blocker work.

ID: architecture-modularity.extraction-inventory-tracks-next-wave-evidence.current-next-actions
