## MODIFIED Requirements

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

VM-CI dogfood diagnostics MUST expose enough bounded progress evidence to identify whether a post-registration stall occurs while resolving the workspace ticket, fetching workspace blobs, starting the guest executor, invoking Nix, streaming logs, or publishing the job result.

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
