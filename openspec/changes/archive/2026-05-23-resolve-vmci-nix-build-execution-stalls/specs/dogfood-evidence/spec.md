## MODIFIED Requirements

### Requirement: VM-CI Workspace and Blob Progress Evidence [r[dogfood-evidence.vmci.workspace-blob-progress]]

VM-CI dogfood diagnostics MUST expose enough bounded progress evidence to identify whether a post-registration stall occurs while resolving the workspace ticket, fetching workspace blobs, starting the guest executor, preparing the Nix build command, invoking Nix, streaming logs, enforcing command timeout, draining output, publishing the job result, or waiting for CI run completion.

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

#### Scenario: Nix-build timeout finalization rail [r[dogfood-evidence.vmci.layered-harness.nix-timeout-finalization]]

- GIVEN an operator needs to verify `ci_nix_build` timeout and job-result publication without running the full Aspen workspace build
- WHEN the operator runs the dedicated VMCI Nix-build timeout/finalization rail
- THEN the rail MUST submit a real `ci_nix_build` job through the VMCI product path with a deterministic short timeout
- AND the receipt MUST prove either a failed job result was published after timeout or identify the exact last phase before publication
- AND the rail MUST complete substantially faster than `vmci-medium` under normal timeout-regression conditions
