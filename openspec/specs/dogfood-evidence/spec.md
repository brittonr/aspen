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

### Requirement: VM-CI Dogfood Worker Readiness [r[dogfood-evidence.vmci-worker-readiness]]

VM-CI dogfood readiness MUST distinguish direct TAP privilege requirements from helper-backed TAP lifecycle requirements, and MUST emit bounded receipt evidence when either boundary is unavailable.

#### Scenario: Direct TAP mode requires runtime network administration

- GIVEN VM-CI dogfood is configured with `ASPEN_CI_NETWORK_MODE=tap`
- AND the current process lacks `CAP_NET_ADMIN`
- WHEN the dogfood run performs VM-CI readiness checks
- THEN readiness fails before waiting on the CI pipeline
- AND the receipt failure category is `vm_ci_readiness`
- AND the diagnostic says direct TAP mode requires `CAP_NET_ADMIN` or `tap-helper` mode.

#### Scenario: TAP helper mode requires an executable helper

- GIVEN VM-CI dogfood is configured with `ASPEN_CI_NETWORK_MODE=tap-helper`
- AND `ASPEN_CI_TAP_HELPER_PATH` is missing or not executable
- WHEN the dogfood run performs VM-CI readiness checks
- THEN readiness fails before waiting on the CI pipeline
- AND the receipt failure category is `vm_ci_readiness`
- AND the diagnostic names the missing helper path requirement.

#### Scenario: Helper-backed TAP lifecycle stays allowlisted

- GIVEN VM-CI runtime is configured with `NetworkMode::TapWithHelper`
- WHEN a VM TAP device is prepared or cleaned up
- THEN the runtime invokes the configured helper instead of direct `ip` TAP mutation
- AND the helper only accepts `ci-n*-vm*-tap` device names and bridge `aspen-ci-br0`
- AND invalid device names, invalid bridges, or unknown actions are rejected before invoking `ip`.

#### Scenario: Dogfood defaults to installed helper

- GIVEN `setup-ci-network` has installed an executable TAP helper at `/usr/local/libexec/aspen-ci-tap-helper`
- AND the operator did not explicitly set `ASPEN_CI_NETWORK_MODE`
- WHEN `dogfood-local-vmci` starts
- THEN it selects `tap-helper` mode and exports `/usr/local/libexec/aspen-ci-tap-helper` as the helper path
- AND the default avoids `nosuid` temporary mounts where file capabilities can be ignored
- AND the `aspen-node` process does not need ambient `CAP_NET_ADMIN` for TAP lifecycle operations.

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

### Requirement: Operator Receipt Redaction Hardening [r[dogfood-evidence.operator-redaction-hardening]]

Aspen MUST ensure operator-visible dogfood and runtime evidence receipt output remains useful while never displaying raw secret material.

#### Scenario: Secret markers are redacted from receipt rendering [r[dogfood-evidence.operator-redaction-hardening.rendering]]

- GIVEN a receipt contains fields or failure details with tokens, tickets, cookies, private keys, connection strings, or test secret markers
- WHEN receipt list, show, diagnosis, or summary rendering is produced
- THEN the rendered output SHALL NOT contain raw secret values
- AND it SHALL retain non-secret run ids, stage names, artifact identifiers, statuses, and bounded failure categories

#### Scenario: Protected references stay opaque [r[dogfood-evidence.operator-redaction-hardening.opaque-references]]

- GIVEN a receipt references protected owner-only files, capability handles, or cluster connection material
- WHEN the receipt is serialized or displayed
- THEN the output SHALL use opaque handles, content hashes, redacted summaries, or protected-path references rather than raw credentials

#### Scenario: Redaction failure blocks evidence publication [r[dogfood-evidence.operator-redaction-hardening.fail-closed]]

- GIVEN a receipt output path cannot prove that configured secret markers are absent
- WHEN evidence is prepared for operator-facing publication or archival
- THEN the preparation SHALL fail closed or leave the publication task incomplete rather than emitting unsafe evidence

### Requirement: Fresh Dogfood Acceptance Receipt [r[dogfood-evidence.fresh-acceptance-receipt]]

Aspen MUST treat a fresh dogfood full-loop acceptance claim as valid only when a current-HEAD dogfood run produces a durable, secret-safe receipt and operator readback evidence.

#### Scenario: Current HEAD full dogfood succeeds [r[dogfood-evidence.fresh-acceptance-receipt.current-head-success]]

- GIVEN the Aspen checkout is clean and `HEAD` is the intended source revision
- WHEN `nix run .#dogfood-local -- full` completes successfully
- THEN a dogfood run receipt SHALL identify the run, command, source context, ordered stages, final success status, and relevant artifact references
- AND the evidence SHALL be inspectable without relying on chat-only logs

#### Scenario: Receipt readback validates acceptance [r[dogfood-evidence.fresh-acceptance-receipt.readback]]

- GIVEN a successful dogfood run receipt exists
- WHEN an operator uses receipt list, show, or diagnose commands against the configured receipt store
- THEN the commands SHALL surface the accepted final status, stage summary, elapsed timing where available, and artifact references without requiring a running cluster

#### Scenario: Failed dogfood run is not acceptance [r[dogfood-evidence.fresh-acceptance-receipt.failure-boundary]]

- GIVEN a dogfood full run exits unsuccessfully or records a failed stage
- WHEN evidence is captured for the run
- THEN Aspen SHALL record diagnostic evidence and failure category without marking the run accepted
- AND the OpenSpec implementation tasks SHALL remain incomplete until a successful rerun or explicit scope change exists

### Requirement: VM-CI Post-Registration Diagnostics [r[dogfood-evidence.vmci.post-registration-diagnostics]]

VM-CI dogfood runs MUST classify failures after VM worker registration separately from bridge/firewall or ticket-scoping connectivity failures.

#### Scenario: Worker registers and receives a CI job [r[dogfood-evidence.vmci.post-registration-diagnostics.job-assigned]]

- GIVEN a VM-CI dogfood run where the guest worker connects directly to the host over `aspen-ci-br0`
- AND the host assigns a `ci_nix_build` job to that guest worker
- WHEN the job does not complete before the dogfood timeout
- THEN the dogfood evidence MUST classify the failure as post-registration CI execution rather than guest-to-host Iroh/QUIC connectivity
- AND the evidence MUST include stable handles for the host node log, guest serial log, and top-level dogfood run log

#### Scenario: Connectivity regression remains distinguishable [r[dogfood-evidence.vmci.post-registration-diagnostics.connectivity-regression]]

- GIVEN a VM-CI dogfood run where guest serial logs show repeated RPC connection timeouts before worker registration
- WHEN diagnostics summarize the run
- THEN the evidence MUST classify the failure as a connectivity/bootstrap regression
- AND it MUST include the bridge marker version, guest ticket address summary, and relay policy summary needed to re-check the bridge/firewall boundary

### Requirement: VM-CI Workspace and Blob Progress Evidence [r[dogfood-evidence.vmci.workspace-blob-progress]]

VM-CI dogfood diagnostics MUST expose enough bounded progress evidence to identify whether a stall occurs while creating or pushing the Forge source snapshot, creating or resolving the workspace source archive, resolving the workspace ticket, fetching workspace blobs, starting the guest executor, preparing the Nix build command, invoking Nix/Cargo/full CI commands, streaming logs, enforcing command timeout, draining output, publishing the job result, or waiting for CI run completion.

#### Scenario: Pre-registration source phase stalls [r[dogfood-evidence.vmci.workspace-blob-progress.source-phase-stall]]

- GIVEN a VMCI dogfood run has started the cluster and created the Forge repository
- WHEN source push, source archive creation, or CI trigger discovery does not complete before the configured timeout
- THEN diagnostics MUST identify the source push/archive/trigger phase as the last observed phase
- AND the failure MUST NOT be reported as guest VM registration, workspace blob materialization, or full CI timeout

#### Scenario: Workspace materialization stalls [r[dogfood-evidence.vmci.workspace-blob-progress.materialization-stall]]

- GIVEN a VM worker has accepted a CI job with workspace/blob inputs
- WHEN workspace materialization does not complete before the configured timeout
- THEN the job or dogfood diagnostic output MUST identify the workspace/blob phase as the last observed phase
- AND it MUST include a redacted workspace/blob identifier or count, timeout duration, and the guest log artifact path

#### Scenario: Nix executor starts after workspace materialization [r[dogfood-evidence.vmci.workspace-blob-progress.executor-started]]

- GIVEN workspace materialization completes in the guest
- WHEN the guest executor starts Nix or the configured CI command
- THEN diagnostics MUST record that executor start boundary separately from workspace/blob fetch
- AND later failures MUST preserve stderr/log snippets using the existing CI failure diagnostics contract

#### Scenario: Nix build command preparation is visible [r[dogfood-evidence.vmci.workspace-blob-progress.nix-command-preparation]]

- GIVEN workspace materialization completes for a `ci_nix_build` job
- WHEN the guest prepares to transform the Nix payload into an executable command request
- THEN diagnostics MUST preserve bounded phase markers for payload decode, workspace readiness, payload transformation, command request construction, and command execute entry
- AND a dogfood timeout before `command_started` MUST classify the failure as a pre-spawn `ci_nix_build` execution stall rather than generic CI wait timeout

#### Scenario: Long-running command progress remains visible [r[dogfood-evidence.vmci.workspace-blob-progress.command-progress]]

- GIVEN workspace materialization completes in the guest
- AND the guest executor starts a long-running CI command
- WHEN the command continues running until a CI or dogfood timeout
- THEN diagnostics MUST preserve bounded command progress markers for command start, command-running heartbeat, and timeout where available
- AND those markers MUST avoid exposing command arguments or environment values that can contain credentials

#### Scenario: Missing command progress is explicit [r[dogfood-evidence.vmci.workspace-blob-progress.missing-command-progress]]

- GIVEN a VM-CI dogfood run reaches `executor_started` for a `ci_nix_build` job
- AND the CI job remains `running` until dogfood wait timeout
- WHEN no command progress marker is present in the retained CI logs or VM diagnostics
- THEN the dogfood failure detail MUST explicitly report `no_command_progress_marker` with the job id, job type, worker id when known, and the last observed VMCI boundary
- AND the diagnostic summary MUST preserve handles to the relevant host node log, guest serial log, receipt, and CI run id

### Requirement: VMCI Layered Harness Rails [r[dogfood-evidence.vmci.layered-harness]]

The VMCI layered harness MUST provide named rails that exercise progressively larger parts of the product path while preserving Forge push, CI trigger, source archive, VM worker, workspace materialization, and job-result evidence appropriate to each rail.

#### Scenario: Shell/source materialization rail passes independently [r[dogfood-evidence.vmci.layered-harness.shell]]

- GIVEN an operator runs the VMCI harness shell/source rail
- WHEN the cluster starts and a VM worker executes the smoke job
- THEN the rail MUST verify the source root was materialized in the guest
- AND it MUST complete without running guest Nix evaluation, Cargo workspace checks, or full CI jobs
- AND it MUST emit a structured receipt identifying the rail and CI run id

#### Scenario: Guest Nix rail passes independently [r[dogfood-evidence.vmci.layered-harness.nix]]

- GIVEN the shell/source rail boundary is expected to work
- WHEN an operator runs the VMCI harness Nix rail
- THEN the rail MUST verify guest Nix can evaluate a bounded expression from the materialized workspace context
- AND it MUST avoid building the Aspen workspace or full CI closure
- AND it MUST emit a structured receipt identifying the rail and CI run id

#### Scenario: Guest Cargo rail passes independently [r[dogfood-evidence.vmci.layered-harness.cargo]]

- GIVEN the shell/source and guest Nix boundaries are expected to work
- WHEN an operator runs the VMCI harness Cargo rail
- THEN the rail MUST verify guest Cargo/Rust execution using a bounded smoke check
- AND it MUST avoid accidentally evaluating the full Aspen workspace graph unless the selected rail is full CI
- AND it MUST emit a structured receipt identifying the rail and CI run id

#### Scenario: Medium build rail separates build acceptance from clippy [r[dogfood-evidence.vmci.layered-harness.medium]]

- GIVEN source/blob, guest Nix, and guest Cargo smoke rails pass
- WHEN an operator runs the VMCI medium acceptance rail
- THEN the rail MUST exercise the real CI trigger, workspace materialization, format check, and at least one Nix build job
- AND it MUST NOT run the expensive clippy, nextest, or deploy gates
- AND it MUST emit a structured receipt identifying the rail and CI run id

#### Scenario: Clippy rail runs independently [r[dogfood-evidence.vmci.layered-harness.clippy]]

- GIVEN the medium build rail is separated from full CI
- WHEN an operator runs the VMCI clippy rail
- THEN the rail MUST execute the clippy gate without also running build, nextest, or deploy stages
- AND timeout or runtime failures MUST be attributable to the clippy rail instead of VMCI transport/source materialization

#### Scenario: Full VMCI remains final acceptance [r[dogfood-evidence.vmci.layered-harness.full]]

- GIVEN the smoke rails pass
- WHEN an operator runs the full VMCI acceptance rail
- THEN the rail MUST execute the configured full CI pipeline
- AND failures MUST be classified against the same boundary taxonomy used by smoke rails

#### Scenario: Nix-build timeout finalization rail [r[dogfood-evidence.vmci.layered-harness.nix-timeout-finalization]]

- GIVEN an operator needs to verify `ci_nix_build` timeout and job-result publication without running the full Aspen workspace build
- WHEN the operator runs the dedicated VMCI Nix-build timeout/finalization rail
- THEN the rail MUST submit a real `ci_nix_build` job through the VMCI product path with a deterministic short timeout
- AND the receipt MUST prove either a failed job result was published after timeout or identify the exact last phase before publication
- AND the rail MUST complete substantially faster than `vmci-medium` under normal timeout-regression conditions

### Requirement: VMCI Phase Receipt [r[dogfood-evidence.vmci.phase-receipts]]

Dogfood MUST emit bounded structured VMCI phase evidence that identifies the last completed or currently active boundary for each harness run.

#### Scenario: Source push or archive stall is classified [r[dogfood-evidence.vmci.phase-receipts.source-push-stall]]

- GIVEN a VMCI harness run creates the Forge repository
- AND the run does not reach CI trigger discovery before the configured phase timeout
- WHEN dogfood writes diagnostics or a receipt
- THEN the receipt MUST classify the failure as a Forge source push/archive/trigger stall rather than VM registration, workspace materialization, executor, or full CI failure
- AND it MUST include redacted artifact paths or log handles sufficient to inspect the push/archive phase

#### Scenario: VM registration stall is classified [r[dogfood-evidence.vmci.phase-receipts.registration-stall]]

- GIVEN a CI run is created and VMCI workers are expected
- WHEN no guest worker registers before the configured phase timeout
- THEN the receipt MUST classify the failure as VM registration/bootstrap
- AND it MUST preserve the relevant node log and VM serial log handles when available

#### Scenario: Workspace materialization boundary is classified [r[dogfood-evidence.vmci.phase-receipts.workspace-boundary]]

- GIVEN a VM worker receives a job with source blob inputs
- WHEN source hash, blob fetch, or extraction fails or stalls
- THEN the receipt MUST classify the failure as workspace/source materialization
- AND it MUST include bounded redacted evidence for source hash presence, blob fetch, and extraction progress where available

#### Scenario: Executor command boundary is classified [r[dogfood-evidence.vmci.phase-receipts.executor-boundary]]

- GIVEN workspace materialization completes
- WHEN the configured command starts, continues, times out, or exits non-zero
- THEN the receipt MUST classify the failure as executor/CI command execution
- AND it MUST preserve bounded command progress markers without raw arguments or environment values

### Requirement: VMCI Harness Redaction [r[dogfood-evidence.vmci.harness-redaction]]

VMCI harness receipts and summaries MUST redact secrets and credential-like values before writing durable evidence.

#### Scenario: Ticket and environment values are not persisted [r[dogfood-evidence.vmci.harness-redaction.no-secrets]]

- GIVEN a VMCI harness run uses cluster tickets, direct route metadata, and command environments
- WHEN it writes receipts, summaries, or progress markers
- THEN those artifacts MUST NOT include raw tickets, credential values, raw environment values, or unbounded command arguments
- AND tests MUST cover representative ticket-like and env-like strings

### Requirement: VM-CI Evidence Preservation Before Cleanup [r[dogfood-evidence.vmci.preserve-before-cleanup]]

VM-CI dogfood tooling MUST preserve redacted host and guest evidence before stopping or deleting `/tmp/aspen-dogfood` when a run reaches VM job assignment but lacks a final success receipt.

#### Scenario: Failed VM-CI run preserves artifacts [r[dogfood-evidence.vmci.preserve-before-cleanup.failed-run]]

- GIVEN a `dogfood-local-vmci -- full` run reaches VM worker registration or job assignment and then fails or times out
- WHEN cleanup runs
- THEN the top-level dogfood log, `/tmp/aspen-dogfood/node1.log`, relevant VM serial logs, and any receipt JSON MUST be copied to `target/runtime-proof/` or an equivalent configured evidence directory before removal
- AND shared summaries MUST redact secrets, tickets, and long opaque credential-like values

#### Scenario: Operator can archive classified evidence [r[dogfood-evidence.vmci.preserve-before-cleanup.archive-ready]]

- GIVEN a classified VM-CI failure evidence bundle exists
- WHEN the OpenSpec task is marked complete
- THEN the evidence bundle MUST include enough stable artifact paths and command outputs to support archive review without requiring the live VM processes to still be running
