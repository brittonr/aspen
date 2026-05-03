## ADDED Requirements

### Requirement: Dogfood Receipt Diagnosis [r[dogfood-evidence.receipt-inspection.diagnose]]

The dogfood orchestrator MUST provide a read-only CLI command that interprets one validated dogfood run receipt and prints deterministic operator triage guidance.

#### Scenario: Diagnose failed receipt [r[dogfood-evidence.receipt-inspection.diagnose.failed]]

- **GIVEN** a valid dogfood receipt with a failed stage and failure summary
- **WHEN** an operator runs `aspen-dogfood receipts diagnose <run-id-or-path>`
- **THEN** the command prints the run id, failed stage, failure category, failure message, and stage/category-specific first checks

#### Scenario: Diagnose successful receipt [r[dogfood-evidence.receipt-inspection.diagnose.success]]

- **GIVEN** a valid dogfood receipt with no failed stage
- **WHEN** an operator runs `aspen-dogfood receipts diagnose <run-id-or-path>`
- **THEN** the command reports that no failed stage was found and does not require a running cluster

#### Scenario: Diagnose rejects invalid receipt evidence [r[dogfood-evidence.receipt-inspection.diagnose.invalid]]

- **GIVEN** the selected receipt file is missing, malformed, or uses an unexpected schema
- **WHEN** an operator runs `aspen-dogfood receipts diagnose <run-id-or-path>`
- **THEN** the command fails with an operator-visible receipt error instead of printing unvalidated guidance
