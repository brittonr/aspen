## ADDED Requirements

### Requirement: RPC message serialization snapshots

The system SHALL include insta snapshot tests for all RPC request and response message types to detect accidental wire format changes.

#### Scenario: Serialization format unchanged

- **WHEN** snapshot tests run after a code change
- **AND** no intentional wire format changes were made
- **THEN** all serialization snapshots SHALL pass without changes

#### Scenario: Intentional format change detected

- **WHEN** a developer changes an RPC message struct field
- **THEN** the corresponding insta snapshot test SHALL fail
- **AND** the developer SHALL run `cargo insta review` to approve the new format

#### Scenario: Non-deterministic fields redacted

- **WHEN** an RPC message contains timestamps, UUIDs, or node IDs
- **THEN** the snapshot test SHALL use insta redaction to mask those fields
- **AND** the snapshot SHALL be stable across test runs

### Requirement: CLI output format snapshots

The system SHALL include insta snapshot tests for CLI command output formatting to prevent accidental display regressions.

#### Scenario: Table output format preserved

- **WHEN** `aspen-cli cluster status` output is snapshot-tested
- **AND** no formatting changes were made
- **THEN** the snapshot SHALL match the recorded output

### Requirement: Error chain snapshots

The system SHALL include insta snapshot tests for snafu error chain formatting to verify error messages remain actionable.

#### Scenario: Error context chain preserved

- **WHEN** a multi-level snafu error is constructed (e.g., I/O error → storage error → Raft error)
- **THEN** the snapshot of its `Display` output SHALL show the full context chain
- **AND** changes to error messages SHALL require explicit snapshot approval
