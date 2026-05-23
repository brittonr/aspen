## Context

The VMCI debug ladder has advanced past lower layers. Smoke/source/blob rails passed; medium rails repeatedly reached post-registration CI. Stale queue redelivery was fixed and live-proven. Diagnostics now classify post-registration CI correctly and preserve bounded log tails. A watchdog/output-drain patch fixed a separate shell timeout-finalization seam, but the latest medium proof reached `build-cli` and still ended as `build:ci_wait` with the CI job `running` and no command progress markers in retained diagnostics.

This change treats the absence of `ASPEN_CI_COMMAND_PROGRESS` after `executing local job` as the key product gap. We need to know whether `ci_nix_build` is stuck before command spawn, bypassing the instrumented command runner, writing logs to an uncollected store, or unable to publish completion.

## Goals / Non-Goals

**Goals:**

- Make every `ci_nix_build` job expose a bounded phase trail from guest receipt to final publication.
- Ensure Nix build commands obey configured timeout, cancel descendants, bound output drain, and return failed `ExecutionResult` / job result on timeout.
- Preserve enough redacted dogfood evidence to identify the last phase without live VM processes.
- Add a cheap timeout/finalization proof rail or fixture before rerunning full clippy/full CI.

**Non-Goals:**

- Optimizing Nix closure/cache performance.
- Changing VMCI networking, source blob streaming, or direct-route behavior without new evidence.
- Expanding receipt/log output to include secrets, raw environment, full argv with credentials, or raw tickets.

## Decisions

### 1. Phase markers around the Nix-build guest path

**Choice:** Emit structured `ASPEN_CI_COMMAND_PROGRESS`-compatible markers for `ci_nix_build` phases before and after every potentially blocking transition: `payload_decoded`, `workspace_ready`, `nix_payload_transformed`, `command_request_built`, `command_execute_enter`, `command_started`, `command_running`, `command_timeout`, `command_execute_returned`, `job_result_publish_enter`, and `job_result_published`.

**Rationale:** The current evidence stops at `executing local job`. Markers before command spawn distinguish pre-spawn deadlocks from slow Nix builds and from log-capture gaps.

**Alternative:** Add ad-hoc serial log messages only. Rejected because dogfood timeout summaries and CI log APIs need stable machine-readable strings.

### 2. Shared timeout/finalization semantics

**Choice:** Route `ci_nix_build` command execution through the same bounded command runner/watchdog/output-drain path used for shell commands, or encapsulate the common behavior behind a pure/testable helper that both paths call.

**Rationale:** Prior repairs to shell command timeout finalization do not help if Nix jobs bypass that code path. Timeout semantics must be uniform for any CI job that spawns a process.

**Alternative:** Increase dogfood or build timeout. Rejected because the observed mismatch is a finalization/observability failure, not proven legitimate build duration.

### 3. Dogfood timeout evidence includes running-job internals

**Choice:** On CI wait timeout, dogfood must include bounded per-running-job metadata: job id, job type, worker id, job timeout, started/updated timestamps when available, last phase marker, and bounded log tail. It must label missing markers explicitly as `no_command_progress_marker` rather than silently showing only `job build-cli: running`.

**Rationale:** A failed receipt must identify the next code boundary without requiring the VM to still be alive.

**Alternative:** Depend on retained serial logs only. Rejected because serial logs can omit CI job logs and may not contain structured job metadata.

### 4. Dedicated timeout/finalization proof rail

**Choice:** Add a tiny VMCI rail or CI fixture that submits a `ci_nix_build` job with a deterministic short-timeout command/flake and proves timeout/failure publication without the full Aspen workspace `build-cli` closure.

**Rationale:** Waiting 3900s for every timeout regression is too slow and confounds finalization with Nix cache/build scale.

**Alternative:** Continue using `vmci-medium` only. Rejected because medium is useful acceptance evidence but too expensive as the first timeout regression rail.

## Risks / Trade-offs

**Marker noise** → Keep markers bounded and stable, and avoid raw env/argv values. Use tests for redaction.

**False confidence from synthetic timeout rail** → Keep the rail on the real VMCI/Forge/source/blob/CI/job path, changing only the command/flake size and timeout. Follow with `vmci-medium` once the tiny rail passes.

**Killing the wrong process tree** → Preserve existing process group/child cancellation behavior where present; add tests around timeout result publication rather than relying only on live proof.

**Receipt schema churn** → Add optional evidence fields or diagnostic text that remains backward compatible with older receipts.

## Validation Plan

1. Focused unit tests for Nix payload-to-command request construction and timeout propagation.
2. Executor tests proving timeout emits markers, cancels/bounds output, and returns failed result.
3. Dogfood diagnostics tests for missing marker, pre-spawn marker, timeout marker, and result-published marker classifications.
4. Redaction tests for markers and timeout summaries.
5. Nix app eval for any new VMCI timeout rail.
6. Live proof order: tiny `ci_nix_build` timeout rail, then `dogfood-local-vmci-medium`, then clippy/full only after medium no longer leaves `build-cli` silently running.
