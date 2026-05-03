## MODIFIED Requirements

### Requirement: Dogfood Receipt Listing [r[dogfood-evidence.receipt-inspection.list]]

The dogfood orchestrator MUST provide a read-only CLI command that lists valid dogfood run receipts from the configured receipts directory, and the listed final status MUST represent aggregate run acceptance rather than only the last recorded stage.

#### Scenario: List receipts summarizes runs [r[dogfood-evidence.receipt-inspection.list.summarizes-runs]]

- **GIVEN** a receipts directory containing valid dogfood receipt JSON files
- **WHEN** an operator runs `aspen-dogfood receipts list`
- **THEN** the command prints each valid run id with created timestamp, command, aggregate final status, stage count, and receipt path

#### Scenario: List surfaces failed acceptance despite cleanup [r[dogfood-evidence.receipt-inspection.list.failed-before-stop]]

- **GIVEN** a receipt with a failed build, deploy, or verify stage followed by a succeeded stop stage
- **WHEN** an operator runs `aspen-dogfood receipts list`
- **THEN** the listed final status is `failed`
