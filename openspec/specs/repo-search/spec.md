## ADDED Requirements

### Requirement: Repo list has a filter input

The repo list page SHALL include a text input that filters visible repos by name as the user types.

#### Scenario: Filter matches some repos

- **WHEN** user types "hello" into the filter input
- **THEN** only repos whose name contains "hello" (case-insensitive) SHALL be visible

#### Scenario: Filter matches no repos

- **WHEN** user types a string that matches no repo name
- **THEN** a "No matching repositories" message SHALL be shown

#### Scenario: Empty filter

- **WHEN** the filter input is empty
- **THEN** all repos SHALL be visible
