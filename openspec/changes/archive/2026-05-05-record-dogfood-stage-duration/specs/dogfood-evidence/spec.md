## ADDED Requirements

### Requirement: Dogfood Stage Duration Receipts [r[dogfood-evidence.stage-receipts.elapsed-ms]]

Dogfood stage receipts MUST support explicit elapsed millisecond evidence for completed or failed stages while preserving compatibility with receipts that do not include the field.

#### Scenario: New full-run stage records elapsed milliseconds [r[dogfood-evidence.stage-receipts.elapsed-ms.recorded]]

- GIVEN a dogfood `full` stage finishes successfully or fails
- WHEN the dogfood run receipt is written
- THEN the stage receipt includes an `elapsed_ms` value measured by the orchestrator
- AND the value is serialized as a non-negative integer

#### Scenario: Legacy receipt remains valid [r[dogfood-evidence.stage-receipts.elapsed-ms.legacy-compatible]]

- GIVEN a valid v1 dogfood receipt created before `elapsed_ms` existed
- WHEN an operator runs `receipts show` or `receipts diagnose`
- THEN the receipt MUST still parse and validate
- AND the missing duration MUST render as unavailable rather than making the receipt invalid

## MODIFIED Requirements

### Requirement: Dogfood Receipt Show [r[dogfood-evidence.receipt-inspection.show]]

The dogfood orchestrator MUST provide a read-only CLI command that displays one validated dogfood run receipt by run id or explicit path, including per-stage elapsed duration when present.

#### Scenario: Show receipt by run id [r[dogfood-evidence.receipt-inspection.show.run-id]]

- GIVEN a receipt exists in the configured receipts directory for a run id
- WHEN an operator runs `aspen-dogfood receipts show <run-id>`
- THEN the command prints run identity, mode, receipt path, and every stage with status, timestamps, elapsed duration when present, artifacts, and failure summary when present
