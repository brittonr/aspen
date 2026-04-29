## ADDED Requirements

### Requirement: Verification indexes are fresh
Verification indexes SHALL cite artifacts and paths that reflect the final reviewed state of the change.

ID: openspec-governance.verification-index-freshness

#### Scenario: Fresh verification index passes
ID: openspec-governance.verification-index-freshness.fresh-index-passes

- **GIVEN** `verification.md` cites current tracked artifacts and final preflight output
- **WHEN** preflight runs
- **THEN** it SHALL pass freshness checks

#### Scenario: Stale active path fails after archive
ID: openspec-governance.verification-index-freshness.stale-active-path-fails

- **GIVEN** an archived change's task coverage cites `openspec/changes/<name>` instead of the archive path
- **WHEN** preflight runs
- **THEN** it SHALL fail with the stale path

#### Scenario: Saved diff artifact may contain historical paths
ID: openspec-governance.verification-index-freshness.saved-diff-may-contain-historical-paths

- **GIVEN** a saved diff artifact contains old active change paths as historical diff context
- **WHEN** freshness validation scans archived evidence
- **THEN** the stale path SHALL be allowed only inside the saved diff artifact and not in current task coverage or verification artifact pointers

#### Scenario: Placeholder preflight fails
ID: openspec-governance.verification-index-freshness.placeholder-preflight-fails

- **GIVEN** a cited preflight artifact still contains placeholder text
- **WHEN** preflight runs
- **THEN** it SHALL fail before completion is claimed

#### Scenario: Evidence generated before final diff fails
ID: openspec-governance.verification-index-freshness.generated-before-final-diff-fails

- **GIVEN** a verification index cites a final-diff or preflight artifact captured before the final source or documentation edits
- **WHEN** freshness validation runs
- **THEN** it SHALL fail or require the artifact to be regenerated after staging the final reviewed state
