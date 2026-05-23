## Why

VM-CI medium now reliably proves host startup, Forge push, source archive propagation, guest worker registration, job assignment, workspace blob materialization, and executor entry. The remaining failure is narrower and more dangerous: a guest-owned `ci_nix_build` job can remain `running` until the dogfood wait timeout while retained diagnostics contain no `ASPEN_CI_COMMAND_PROGRESS` marker, no command timeout marker, and no job-result publication.

The latest live evidence is receipt `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260518T232741Z.json`: `vmci-medium`, `start=succeeded`, `push=succeeded`, `format-check=success`, `build-cli=running`, final `build:ci_wait` / `ci_wait_timeout` after `3900s`, diagnostics summary `vm_ci_boundary=executor_started`. This means another VMCI networking/blob retry will not resolve the issue; Aspen needs a durable contract for `ci_nix_build` execution progress, timeout enforcement, and final job publication.

## What Changes

- **Nix-build execution state machine**: Make the guest `ci_nix_build` path expose stable phase markers from payload decode through command spawn, heartbeat, timeout, cancellation, and final result publication.
- **Shared bounded command runner**: Ensure `ci_nix_build` uses the same timeout/watchdog/output-drain semantics as shell commands, or prove an equivalent path with focused tests.
- **Timeout finalization contract**: A Nix build command that exceeds its configured timeout must publish a failed CI job result and must not remain `running` until dogfood timeout.
- **Running-job evidence**: Dogfood timeout summaries and diagnostics must preserve bounded/redacted job metadata, latest phase marker, and log tails for still-running `ci_nix_build` jobs.
- **Focused VMCI proof rail**: Add a small `ci_nix_build` timeout/finalization rail or fixture so this behavior can be proven without waiting for the full Aspen `build-cli` graph.

## Capabilities

### Modified Capabilities

- `dogfood-evidence.vmci.workspace-blob-progress`: Post-materialization diagnostics must distinguish executor entry, pre-spawn stalls, command spawn, command heartbeat, timeout, finalization, and result publication.
- `ci-failure-diagnostics`: CI Nix build jobs must retain bounded logs/markers and fail deterministically on timeout.
- `dogfood-evidence.vmci.layered-harness`: The layered harness must include a cheap rail or fixture that proves Nix-build timeout/finalization independently of full workspace scale.

## Impact

- **Files**: likely `crates/aspen-ci-executor-shell/src/local_executor/*`, `crates/aspen-ci-executor-shell/src/agent/executor.rs`, `src/bin/aspen_node/worker_only.rs`, `crates/aspen-dogfood/src/ci.rs`, `crates/aspen-dogfood/src/vmci_diagnostics.rs`, `crates/aspen-dogfood/src/forge.rs`, `crates/aspen-dogfood/src/main.rs`, `flake.nix`, and tests.
- **APIs**: internal CI/job log and receipt evidence only; no HTTP endpoints and no non-Iroh communication.
- **Dependencies**: no new external runtime dependency expected.
- **Testing**: focused unit tests for `ci_nix_build` request construction, timeout/cancellation/finalization, redaction, and diagnostics classification; Nix app eval for any new rail; live VMCI timeout-finalization receipt before reattempting clippy/full.

## Out of Scope

- Making `build-cli` or clippy faster as a performance optimization.
- Replacing the Aspen CI scheduler or job manager.
- Treating VMCI bridge/firewall/ticket/blob materialization as the current blocker unless new evidence regresses those boundaries.
- Printing raw tickets, secret keys, environment variables, command secrets, or unbounded command arguments in logs or receipts.
