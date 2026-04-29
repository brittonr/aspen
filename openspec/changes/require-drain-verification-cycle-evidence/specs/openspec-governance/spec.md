## ADDED Requirements

### Requirement: Drain verification cycle is explicit
OpenSpec drain completion SHALL record build, test, and format verification scope for implementation changes.

ID: openspec-governance.drain-verification-cycle

#### Scenario: Full verification cycle recorded
ID: openspec-governance.drain-verification-cycle.full-cycle-recorded

- **GIVEN** a drain implements code changes
- **WHEN** verification is finalized
- **THEN** `cargo build`, `cargo nextest run`, and `nix fmt` or project equivalents SHALL have status and artifact entries

#### Scenario: Scoped alternative recorded
ID: openspec-governance.drain-verification-cycle.scoped-alternative-recorded

- **GIVEN** full verification is too expensive or blocked
- **WHEN** scoped verification is used
- **THEN** the verification matrix SHALL include rationale, command, status, and next best check

#### Scenario: Doc-only bypass is explicit
ID: openspec-governance.drain-verification-cycle.doc-only-bypass-explicit

- **GIVEN** a drain changes only documentation or OpenSpec artifacts and no source code
- **WHEN** full build, nextest, or format checks are skipped
- **THEN** the verification matrix SHALL mark the rail as doc-only with rationale and changed-file evidence

#### Scenario: Final verification tasks require matrix first
ID: openspec-governance.drain-verification-cycle.final-verification-requires-matrix-first

- **GIVEN** a drain change is about to check the final verification task complete
- **WHEN** the verification matrix is missing or incomplete
- **THEN** preflight or done-review SHALL fail before completion is claimed

#### Scenario: Missing verification cycle fails
ID: openspec-governance.drain-verification-cycle.missing-cycle-fails

- **GIVEN** implementation tasks are checked
- **WHEN** no full, scoped, or explicit doc-only verification matrix exists
- **THEN** preflight or done-review SHALL fail
