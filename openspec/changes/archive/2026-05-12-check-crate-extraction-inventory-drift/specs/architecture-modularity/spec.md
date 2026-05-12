## ADDED Requirements

### Requirement: Crate-extraction readiness checks inventory consistency [r[architecture-modularity.crate-extraction-readiness-checks-inventory-consistency]]
The crate-extraction readiness checker SHALL validate that the selected candidate family's typed policy entry, family manifest, evidence index, and broader inventory row remain synchronized.

ID: architecture-modularity.crate-extraction-readiness-checks-inventory-consistency

#### Scenario: Selected family inventory row matches typed policy [r[architecture-modularity.crate-extraction-readiness-checks-inventory-consistency.selected-family-row-matches-policy]]
- GIVEN a crate-extraction readiness check is run for a selected candidate family
- WHEN `scripts/check-crate-extraction-readiness.rs` reads `docs/crate-extraction.md` and `docs/crate-extraction/policy.ncl`
- THEN the check MUST fail if the inventory omits the selected family row
- AND it MUST fail if the row omits the expected family manifest link
- AND it MUST fail if the row omits the selected family's assigned owner group
- AND it MUST fail if the row omits the policy readiness state for a selected candidate.

ID: architecture-modularity.crate-extraction-readiness-checks-inventory-consistency.selected-family-row-matches-policy

#### Scenario: Completed first-blocker text is rejected for ready candidates [r[architecture-modularity.crate-extraction-readiness-checks-inventory-consistency.rejects-stale-next-action]]
- GIVEN every policy candidate in a selected family is marked `extraction-ready-in-workspace`
- WHEN the broader inventory row is checked
- THEN the check MUST fail if the row's next-action text still asks contributors to complete the already-finished first blocker, rerun completed readiness evidence, or assign an owner.

ID: architecture-modularity.crate-extraction-readiness-checks-inventory-consistency.rejects-stale-next-action
