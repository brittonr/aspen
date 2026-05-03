# dogfood-evidence Specification

## Purpose
TBD - created by archiving change dogfood-evidence-receipts. Update Purpose after archive.
## Requirements
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

### Requirement: Dogfood Receipt Listing [r[dogfood-evidence.receipt-inspection.list]]

The dogfood orchestrator MUST provide a read-only CLI command that lists valid dogfood run receipts from the configured receipts directory, and the listed final status MUST represent aggregate run acceptance rather than only the last recorded stage.

#### Scenario: List receipts summarizes runs [r[dogfood-evidence.receipt-inspection.list.summarizes-runs]]

- **GIVEN** a receipts directory containing valid dogfood receipt JSON files
- **WHEN** an operator runs `aspen-dogfood receipts list`
- **THEN** the command prints each valid run id with created timestamp, command, aggregate final status, stage count, and receipt path

#### Scenario: List handles missing receipts directory [r[dogfood-evidence.receipt-inspection.list.missing-directory]]

- **GIVEN** no receipts directory exists for the configured cluster directory
- **WHEN** an operator runs `aspen-dogfood receipts list`
- **THEN** the command succeeds with an empty result rather than requiring a running cluster

#### Scenario: List surfaces failed acceptance despite cleanup [r[dogfood-evidence.receipt-inspection.list.failed-before-stop]]

- **GIVEN** a receipt with a failed build, deploy, or verify stage followed by a succeeded stop stage
- **WHEN** an operator runs `aspen-dogfood receipts list`
- **THEN** the listed final status is `failed`

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

### Requirement: Dogfood Receipt Cluster Publication [r[dogfood-evidence.receipt-cluster-publication]]

The dogfood orchestrator MUST support publishing a validated local dogfood run receipt into the running Aspen cluster so the cluster can carry its own acceptance evidence while it is alive.

#### Scenario: Publish receipt to cluster KV [r[dogfood-evidence.receipt-cluster-publication.publish]]

- **GIVEN** a running dogfood cluster and a valid local dogfood receipt selected by run id or explicit path
- **WHEN** an operator runs `aspen-dogfood receipts publish <run-id-or-path>`
- **THEN** the command writes the canonical receipt JSON to the cluster KV store under `dogfood/receipts/<run-id>.json`
- **AND** the command prints the run id and cluster key without printing credential material

#### Scenario: Reject invalid local receipt before publishing [r[dogfood-evidence.receipt-cluster-publication.reject-invalid]]

- **GIVEN** the selected local receipt is missing, malformed, or uses an unexpected schema
- **WHEN** an operator runs `aspen-dogfood receipts publish <run-id-or-path>`
- **THEN** the command fails before writing to cluster KV

### Requirement: Dogfood Receipt Cluster Show [r[dogfood-evidence.receipt-cluster-show]]

The dogfood orchestrator MUST support loading one published dogfood receipt from the running Aspen cluster and MUST validate it before rendering it to an operator.

#### Scenario: Show published receipt by run id [r[dogfood-evidence.receipt-cluster-show.run-id]]

- **GIVEN** a running dogfood cluster has a value at `dogfood/receipts/<run-id>.json`
- **WHEN** an operator runs `aspen-dogfood receipts cluster-show <run-id>`
- **THEN** the command validates the stored value as a dogfood receipt and prints the same human-readable receipt summary used for local receipt files

#### Scenario: Show published receipt as JSON [r[dogfood-evidence.receipt-cluster-show.json]]

- **GIVEN** a running dogfood cluster has a valid published receipt value
- **WHEN** an operator runs `aspen-dogfood receipts cluster-show <run-id> --json`
- **THEN** the command emits validated canonical JSON for that receipt

#### Scenario: Reject missing or invalid cluster value [r[dogfood-evidence.receipt-cluster-show.invalid]]

- **GIVEN** the running dogfood cluster has no value or an invalid value at the selected receipt key
- **WHEN** an operator runs `aspen-dogfood receipts cluster-show <run-id>`
- **THEN** the command fails with a receipt error rather than printing unvalidated evidence
