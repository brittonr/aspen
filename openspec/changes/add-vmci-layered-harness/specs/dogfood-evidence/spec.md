## ADDED Requirements

### Requirement: Layered VMCI Harness Rails [r[dogfood-evidence.vmci.layered-harness]]

Dogfood MUST provide named VMCI harness rails that exercise progressively broader boundaries without requiring the full CI pipeline for each debugging iteration.

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

## MODIFIED Requirements

### Requirement: VM-CI Workspace and Blob Progress Evidence [r[dogfood-evidence.vmci.workspace-blob-progress]]

VM-CI dogfood diagnostics MUST expose enough bounded progress evidence to identify whether a stall occurs while creating or pushing the Forge source snapshot, creating or resolving the workspace source archive, resolving the workspace ticket, fetching workspace blobs, starting the guest executor, invoking Nix/Cargo/full CI commands, streaming logs, or publishing the job result.

#### Scenario: Pre-registration source phase stalls [r[dogfood-evidence.vmci.workspace-blob-progress.source-phase-stall]]

- GIVEN a VMCI dogfood run has started the cluster and created the Forge repository
- WHEN source push, source archive creation, or CI trigger discovery does not complete before the configured timeout
- THEN diagnostics MUST identify the source push/archive/trigger phase as the last observed phase
- AND the failure MUST NOT be reported as guest VM registration, workspace blob materialization, or full CI timeout
