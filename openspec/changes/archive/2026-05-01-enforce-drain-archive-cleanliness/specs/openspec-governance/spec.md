## ADDED Requirements

### Requirement: Drain completion proves archive cleanliness
OpenSpec drain completion SHALL include durable evidence that active changes are empty and archived paths are consistent.

ID: openspec-governance.drain-archive-cleanliness

#### Scenario: Clean drain passes
ID: openspec-governance.drain-archive-cleanliness.clean-drain-passes

- **GIVEN** all completed changes have been archived
- **WHEN** drain completion audit runs
- **THEN** no active change directories SHALL remain outside `archive` or `_done`
- **AND** `.drain-state.md` SHALL be absent

#### Scenario: Leftover active path fails
ID: openspec-governance.drain-archive-cleanliness.leftover-active-path-fails

- **GIVEN** an active change directory remains after archive is claimed
- **WHEN** drain completion audit runs
- **THEN** it SHALL fail and print the remaining path

#### Scenario: Archived verification uses archive paths
ID: openspec-governance.drain-archive-cleanliness.archive-paths-used

- **GIVEN** a change has moved to `openspec/changes/archive/<date>-<name>`
- **WHEN** final preflight runs
- **THEN** task coverage and changed-file entries SHALL use current archive-relative paths
