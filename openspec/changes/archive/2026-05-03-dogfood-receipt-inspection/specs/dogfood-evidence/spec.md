## ADDED Requirements

### Requirement: Dogfood Receipt Listing [r[dogfood-evidence.receipt-inspection.list]]

The dogfood orchestrator MUST provide a read-only CLI command that lists valid dogfood run receipts from the configured receipts directory.

#### Scenario: List receipts summarizes runs [r[dogfood-evidence.receipt-inspection.list.summarizes-runs]]

- **GIVEN** a receipts directory containing valid dogfood receipt JSON files
- **WHEN** an operator runs `aspen-dogfood receipts list`
- **THEN** the command prints each valid run id with created timestamp, command, final status, stage count, and receipt path

#### Scenario: List handles missing receipts directory [r[dogfood-evidence.receipt-inspection.list.missing-directory]]

- **GIVEN** no receipts directory exists for the configured cluster directory
- **WHEN** an operator runs `aspen-dogfood receipts list`
- **THEN** the command succeeds with an empty result rather than requiring a running cluster

### Requirement: Dogfood Receipt Show [r[dogfood-evidence.receipt-inspection.show]]

The dogfood orchestrator MUST provide a read-only CLI command that displays one validated dogfood run receipt by run id or explicit path.

#### Scenario: Show receipt by run id [r[dogfood-evidence.receipt-inspection.show.run-id]]

- **GIVEN** a receipt exists in the configured receipts directory for a run id
- **WHEN** an operator runs `aspen-dogfood receipts show <run-id>`
- **THEN** the command prints run identity, mode, receipt path, and every stage with status, timestamps, artifacts, and failure summary when present

#### Scenario: Show receipt as JSON [r[dogfood-evidence.receipt-inspection.show.json]]

- **GIVEN** a valid receipt exists
- **WHEN** an operator runs `aspen-dogfood receipts show <run-id-or-path> --json`
- **THEN** the command emits validated canonical JSON for that receipt

#### Scenario: Show rejects invalid receipt evidence [r[dogfood-evidence.receipt-inspection.show.invalid]]

- **GIVEN** the selected receipt file is missing, malformed, or uses an unexpected schema
- **WHEN** an operator runs `aspen-dogfood receipts show <run-id-or-path>`
- **THEN** the command fails with an operator-visible receipt error instead of printing partial evidence
