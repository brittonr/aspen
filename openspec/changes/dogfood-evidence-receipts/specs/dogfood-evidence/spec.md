## ADDED Requirements

### Requirement: Dogfood Run Receipt Schema [r[dogfood-evidence.receipt-schema]]

The dogfood orchestrator MUST define a versioned run receipt schema that records run identity, command identity, selected mode flags, project directory, cluster directory, and ordered stage receipts.

#### Scenario: Receipt includes run identity [r[dogfood-evidence.receipt-schema.run-identity]]

- **GIVEN** a dogfood run receipt
- **WHEN** it is serialized
- **THEN** the JSON includes a schema name, run id, command, created timestamp, mode flags, project directory, cluster directory, and stages array

#### Scenario: Receipt is bounded [r[dogfood-evidence.receipt-schema.bounded]]

- **GIVEN** a receipt with stage and artifact collections
- **WHEN** validation is applied
- **THEN** the receipt rejects empty run ids, empty schema names, too many stages, and too many artifacts per stage

### Requirement: Dogfood Stage Receipts [r[dogfood-evidence.stage-receipts]]

The dogfood orchestrator MUST model each pipeline step as a stage receipt with a stable stage kind, status, started timestamp, optional finished timestamp, and optional failure summary.

#### Scenario: Completed stage records success [r[dogfood-evidence.stage-receipts.success]]

- **GIVEN** a completed push, build, deploy, verify, or stop stage
- **WHEN** the stage receipt is serialized
- **THEN** it records `succeeded` status and omits failure detail

#### Scenario: Failed stage records failure cause [r[dogfood-evidence.stage-receipts.failure]]

- **GIVEN** a failed stage
- **WHEN** the stage receipt is serialized
- **THEN** it records `failed` status and includes an operation, category, and message that can be shown after the run exits

### Requirement: Dogfood Artifact Receipts [r[dogfood-evidence.artifact-receipts]]

The dogfood orchestrator MUST model stage outputs as artifact receipts with stable names, kinds, optional store/blob identifiers, optional digest, optional size, and optional relative path.

#### Scenario: CI artifact linkage [r[dogfood-evidence.artifact-receipts.ci-linkage]]

- **GIVEN** a build stage that produced a CI artifact
- **WHEN** its stage receipt is inspected
- **THEN** the artifact receipt identifies the artifact by name and kind and can include blob/hash identifiers without requiring log scraping

### Requirement: Canonical Receipt Serialization [r[dogfood-evidence.canonical-serialization]]

The dogfood orchestrator MUST provide a deterministic JSON serialization path for run receipts so saved receipts can later be hashed, uploaded, or compared.

#### Scenario: Round-trip serialization [r[dogfood-evidence.canonical-serialization.roundtrip]]

- **GIVEN** a valid dogfood run receipt
- **WHEN** it is serialized to JSON and parsed back
- **THEN** the parsed receipt equals the original receipt

#### Scenario: Stable field names [r[dogfood-evidence.canonical-serialization.field-names]]

- **GIVEN** a serialized dogfood receipt
- **WHEN** an operator opens the JSON
- **THEN** stable field names such as `schema`, `run_id`, `command`, `stages`, `artifacts`, `status`, and `failure` are present
