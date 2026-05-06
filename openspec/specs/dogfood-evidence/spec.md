# dogfood-evidence Specification

## Purpose
Defines the Dogfood Evidence capability requirements preserved by Aspen's archived OpenSpec records, including dogfood run receipt schema, dogfood stage receipts, dogfood artifact receipts.
## Requirements
### Requirement: Dogfood Run Receipt Schema [r[dogfood-evidence.receipt-schema]]

The dogfood orchestrator MUST define a versioned run receipt schema that records run identity, command identity, selected mode flags, project directory, cluster directory, and ordered stage receipts, and the Rust receipt type that owns canonical serialization MUST generate a typed Nickel contract for validating saved and published receipt JSON.

#### Scenario: Receipt includes run identity [r[dogfood-evidence.receipt-schema.run-identity]]

- **GIVEN** a dogfood run receipt
- **WHEN** it is serialized
- **THEN** the JSON includes a schema name, run id, command, created timestamp, mode flags, project directory, cluster directory, and stages array

#### Scenario: Generated contract validates local and published receipts [r[dogfood-evidence.receipt-schema.generated-nickel-contract]]

- GIVEN a dogfood receipt is saved locally or published into Aspen KV
- WHEN the generated Nickel contract validates the receipt JSON
- THEN valid receipts MUST pass and malformed, stale-schema, missing-stage, unbounded-artifact, or invalid-status receipts MUST fail

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

The dogfood orchestrator MUST provide a read-only CLI command that displays one validated dogfood run receipt by run id or explicit path, including per-stage elapsed duration when present.

#### Scenario: Show receipt by run id [r[dogfood-evidence.receipt-inspection.show.run-id]]

- GIVEN a receipt exists in the configured receipts directory for a run id
- WHEN an operator runs `aspen-dogfood receipts show <run-id>`
- THEN the command prints run identity, mode, receipt path, and every stage with status, timestamps, elapsed duration when present, artifacts, and failure summary when present

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

### Requirement: Full Runs Auto-Publish Final Success Receipts [r[dogfood-evidence.full.auto-publish]]

A successful dogfood `full` run MUST publish its validated canonical run receipt into the running Aspen cluster KV before automatic cleanup begins.

#### Scenario: Successful full run publishes before stop [r[dogfood-evidence.full.auto-publish.success]]

- GIVEN a dogfood `full` run has successfully completed push, build, deploy, and verify stages
- WHEN the run prepares to finish
- THEN it MUST write the canonical local receipt to `dogfood/receipts/<run-id>.json` in Aspen KV before running the stop stage
- AND the local receipt MUST include a succeeded `publish_receipt` stage with the cluster KV key as a receipt artifact
- AND the local receipt file MUST remain the durable fallback evidence after cleanup

#### Scenario: Publication failure fails acceptance [r[dogfood-evidence.full.auto-publish.failure]]

- GIVEN push, build, deploy, and verify have succeeded
- WHEN automatic receipt publication to Aspen KV fails
- THEN the run MUST return a dogfood failure instead of reporting full acceptance success
- AND the local receipt MUST record a failed `publish_receipt` stage
- AND the local receipt MUST remain available for diagnosis

### Requirement: Full Runs Can Leave Cluster Running For Evidence Readback [r[dogfood-evidence.full.leave-running]]

The dogfood CLI MUST provide an explicit `full` mode that leaves a successfully verified cluster running after receipt auto-publication so operators can query the published receipt from Aspen KV.

#### Scenario: Operator leaves cluster running [r[dogfood-evidence.full.leave-running.success]]

- GIVEN an operator runs `aspen-dogfood full --leave-running`
- WHEN push, build, deploy, verify, and automatic receipt publication succeed
- THEN the command MUST exit without running the stop stage
- AND the cluster state MUST remain usable by `receipts cluster-show <run-id> --json`
- AND operator documentation MUST tell the operator to run `stop` after inspection

#### Scenario: Default full still cleans up [r[dogfood-evidence.full.leave-running.default-cleanup]]

- GIVEN an operator runs `aspen-dogfood full` without `--leave-running`
- WHEN push, build, deploy, verify, and automatic receipt publication complete
- THEN the command MUST run the stop stage and clean up the dogfood cluster as before

### Requirement: Native CI Run Receipts [r[dogfood-evidence.ci.run-receipt]]

Aspen MUST expose a native CI run receipt that operators can query by pipeline run ID without using the dogfood receipt wrapper, and the canonical Rust receipt type MUST generate a typed Nickel contract that validates exported receipt JSON.

#### Scenario: Operator queries a CI run receipt [r[dogfood-evidence.ci.run-receipt.query]]

- GIVEN Aspen has a persisted CI pipeline run in Raft-backed KV
- WHEN an operator requests the CI run receipt by run ID
- THEN Aspen MUST return a schema-versioned receipt with schema `aspen.ci.run-receipt.v1`
- AND the receipt MUST include run ID, repository ID, ref, commit hash, pipeline name, status, created/completed timestamps, stages, and jobs
- AND jobs MUST include their job IDs when available so operators can use log/output commands as follow-up handles

#### Scenario: Receipt has a generated Nickel contract [r[dogfood-evidence.ci.run-receipt.generated-nickel-contract]]

- GIVEN the Rust type that owns native CI run receipt serialization changes
- WHEN the receipt contract freshness check runs
- THEN the generated Nickel contract for `aspen.ci.run-receipt.v1` MUST be updated or the check MUST fail

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

### Requirement: CI Run Receipts Include Artifact Metadata [r[dogfood-evidence.ci.run-receipt.artifacts]]

Native CI run receipts MUST include operator-safe artifact metadata for jobs that produced artifacts.

#### Scenario: Job artifact metadata appears in receipt [r[dogfood-evidence.ci.run-receipt.artifacts.present]]

- GIVEN Aspen has a persisted CI pipeline run with a job ID
- AND CI artifact metadata exists under the job artifact KV prefix
- WHEN an operator requests the CI run receipt by run ID
- THEN the receipt job entry MUST include the artifact name, blob hash, size, content type, creation timestamp, and metadata
- AND the receipt MUST NOT include blob download tickets or credential material

#### Scenario: Artifact metadata is deterministic and bounded [r[dogfood-evidence.ci.run-receipt.artifacts.deterministic]]

- GIVEN a CI job has multiple artifact metadata records
- WHEN Aspen renders the CI run receipt
- THEN artifact entries MUST be ordered deterministically
- AND artifact scanning MUST use a bounded per-job limit

#### Scenario: Artifact scan failures are explicit [r[dogfood-evidence.ci.run-receipt.artifacts.scan-failure]]

- GIVEN Aspen cannot scan artifact metadata for a job while constructing a CI run receipt
- WHEN an operator requests the CI run receipt
- THEN the request MUST fail explicitly rather than returning a partial receipt that appears complete

### Requirement: Current head dogfood receipt is durable
A current-head dogfood acceptance run MUST produce a durable schema-versioned receipt tied to the exact git commit and run id.
ID: r[dogfood-evidence.current-head-receipt-durable]

#### Scenario: Current head dogfood receipt is durable evidence
The current-head dogfood evidence scenario MUST record the produced receipt and redacted diagnostics.
ID: r[dogfood-evidence.current-head-receipt-durable.evidence]
- GIVEN `main` is clean and synced before the dogfood run
- WHEN the full dogfood loop completes or fails
- THEN the receipt SHALL record git commit, schema version, run id, stage outcomes, timings, artifact identifiers, failure category when any, and redacted diagnostics.

### Requirement: Receipt readback proves operator evidence
Dogfood receipt acceptance MUST prove that operators can inspect local receipt data and any cluster-published receipt without scraping logs.
ID: r[dogfood-evidence.receipt-readback-operator-evidence]

#### Scenario: Receipt readback proves operator evidence evidence
The receipt readback evidence scenario MUST include local receipt readback and diagnostics.
ID: r[dogfood-evidence.receipt-readback-operator-evidence.evidence]
- GIVEN a dogfood receipt is produced
- WHEN `receipts show`/`diagnose` or equivalent documented commands are run
- THEN the evidence SHALL include local receipt path, cluster KV key when published, readback result, and redaction confirmation for secrets.
