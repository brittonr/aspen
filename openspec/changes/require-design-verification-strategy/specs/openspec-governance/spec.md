## ADDED Requirements

### Requirement: Designs map requirements to verification rails
Design artifacts SHALL include a verification strategy that maps changed requirements to concrete checks before tasks are approved.

ID: openspec-governance.design-verification-strategy

#### Scenario: Design includes mapped verification strategy
ID: openspec-governance.design-verification-strategy.mapped-strategy-present

- **GIVEN** a change has delta specs
- **WHEN** the design gate runs
- **THEN** `design.md` SHALL include `## Verification Strategy`
- **AND** the strategy SHALL cite relevant requirement or scenario IDs

#### Scenario: Missing strategy fails design gate
ID: openspec-governance.design-verification-strategy.missing-strategy-fails

- **GIVEN** a change modifies specs but omits verification strategy
- **WHEN** the design gate runs
- **THEN** the gate SHALL fail with a design-stage finding

#### Scenario: Negative paths are planned
ID: openspec-governance.design-verification-strategy.negative-paths-planned

- **GIVEN** a delta spec defines rejection, failure, timeout, malformed, or unauthorized behavior
- **WHEN** design verification is reviewed
- **THEN** the strategy SHALL name a negative check or explicitly defer it with rationale
