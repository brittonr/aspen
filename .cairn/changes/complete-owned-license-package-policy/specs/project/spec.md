## MODIFIED Requirements

### Requirement: License boundary validation is deterministic

r[molten.project.license_boundary.final_validation] The repository MUST validate the selected package expressions, required license artifacts, current documentation, and absence of contradictory project-owned license declarations. The license policy MUST contain exactly one package rule for every current workspace package.

#### Scenario: A project-owned declaration drifts

- GIVEN one current project-owned metadata or documentation surface declares a conflicting project license
- WHEN the focused license audit runs
- THEN the audit MUST fail without treating dependency license expressions as project drift.

#### Scenario: A workspace package is missing from license policy

- GIVEN a current Molten-owned workspace package inherits the selected AGPL expression but lacks its package policy rule
- WHEN the focused license audit runs
- THEN the audit MUST fail until exactly one package rule covers that workspace package.
