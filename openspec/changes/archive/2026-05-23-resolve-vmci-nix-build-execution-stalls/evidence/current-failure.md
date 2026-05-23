# Current failure evidence

Latest medium live proof before the timeout/finalization fixes:

- Command: `nix run .#dogfood-local-vmci-medium`
- Receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260518T232741Z.json`
- Diagnostics: `/home/brittonr/git/aspen/target/runtime-proof/vmci-diagnostics/dogfood-20260518T232741Z/`
- Rail: `vmci-medium`
- Final status: `build=failed`, `stop=succeeded`
- Failure: `build:ci_wait` / `ci_wait_timeout` after `3900s`
- CI status at timeout:
  - `format-check=success`
  - `build-cli=running`
- Diagnostic summary:
  - `vm_ci_boundary=executor_started`
  - `vm_ci_failure_class=post_registration_ci_execution`
  - `post_registration=true`
  - evidence includes `workspace_materialized,executor_started`

Interpretation at that point: lower VMCI layers were proven. The unresolved boundary was inside guest `ci_nix_build` execution after job assignment/workspace materialization/executor entry, specifically command preparation/spawn/progress/timeout/finalization/job-result publication.

## Cheap VMCI Nix timeout rail evidence

Initial live proof after adding `vm-ci-nix-timeout`:

- Command: `nix run .#dogfood-local-vmci-nix-timeout`
- Receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260519T024656Z.json`
- Diagnostics: `/home/brittonr/git/aspen/target/runtime-proof/vmci-diagnostics/dogfood-20260519T024656Z/`
- Rail: `vmci-nix-timeout`
- Failure: `build:ci_wait` / `ci_wait_timeout` after `300s`
- CI status at timeout:
  - `nix-timeout=running`
  - `guest-nix-timeout-finalization=running`

Dogfood's central running-job log tail had no `ASPEN_CI_COMMAND_PROGRESS` rows and reported `missing_phase=nix_payload_transformed`; preserved guest serial logs were more precise:

- `ci-n1-vm0-serial.log` showed the job was received as `ci_nix_build`.
- It emitted `ASPEN_CI_COMMAND_PROGRESS phase=nix_payload_transformed` with `timeout_secs=5`.
- It emitted `ASPEN_CI_COMMAND_PROGRESS phase=local_executor_payload_validated`.
- It created `/tmp/workspaces/<job-id>` and started source blob seeding.
- It retrieved the source blob successfully.
- It did **not** emit `workspace_ready`, `command_request_built`, `command_execute_enter`, `command_started`, `command_timeout`, `command_execute_returned`, `result_publish_enter`, or `result_published` before dogfood timeout.

Interpretation: this rail moved the live boundary earlier than medium `build-cli`: the guest could receive and transform the `ci_nix_build` job and fetch the source blob, but could stall after blob retrieval and before workspace-ready/command construction. Workspace archive unpack/materialization needed bounded markers and a timeout.

## VMCI Nix timeout rail after workspace-materialization instrumentation

Live proof after adding workspace materialization markers/timeouts:

- Command: `nix run .#dogfood-local-vmci-nix-timeout`
- Receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260519T040004Z.json`
- Diagnostics: `/home/brittonr/git/aspen/target/runtime-proof/vmci-diagnostics/dogfood-20260519T040004Z/`
- Rail: `vmci-nix-timeout`
- Failure: `build:ci_wait` / `ci_wait_timeout` after `300s`

Preserved guest serial showed the guest worker received a `ci_nix_build` job from the cluster but did **not** emit the next expected VM worker marker (`nix_payload_transformed`) or any local executor/workspace markers. That narrowed the live fault domain to the guest worker loop between job receipt and payload parse/transform/executor handoff.

Patch direction: add bounded/redacted pre-executor markers for job-spec parse, Nix payload transform, working-dir rewrite, local job construction, active-log-job setup, visibility extender spawn, and executor entry; classify these as a `pre_executor` diagnostic boundary.

## VMCI Nix timeout rail after pre-executor instrumentation

Live proof after adding pre-executor markers:

- Command: `nix run .#dogfood-local-vmci-nix-timeout`
- Receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260519T043253Z.json`
- Diagnostics: `/home/brittonr/git/aspen/target/runtime-proof/vmci-diagnostics/dogfood-20260519T043253Z/`
- Rail: `vmci-nix-timeout`
- Failure: `build:ci_wait` / `ci_wait_timeout` after `300s`

Preserved guest serial moved the boundary forward:

- `job_spec_parse_enter` / parse completion markers were present.
- `nix_payload_transformed` was present.
- working-dir rewrite, local job construction, active-log-job setup, visibility extender spawn, and `executor_enter` were present.
- `local_executor_payload_validated` was present.
- workspace source fetch started with `source_blob_fetch_enter`.
- The guest opened an Iroh client RPC connection to the host-side endpoint for the source archive blob.
- No `source_blob_fetch_done`, archive decode, workspace-ready, command, timeout, or result publication marker appeared before dogfood timed out.

Interpretation: VMCI startup, source push/archive creation, guest registration, job assignment, worker pre-executor transform, and local executor entry were proven. The remaining blocker was deeper in source fetch/materialization or executor finalization. The worker gained a VM executor watchdog: payload `timeout_secs` + 120s materialization budget + 30s grace, publishing a failed job result on expiry.

## VMCI Nix timeout rail after durable progress/result publication

Live proof after adding nonblocking progress marker sends, an independent process timeout guard, durable progress-marker preservation, and delaying active-log-job clearing until after completion publication:

- Command: `nix run .#dogfood-local-vmci-nix-timeout`
- Receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260519T150149Z.json`
- Diagnostics: `/home/brittonr/git/aspen/target/runtime-proof/vmci-diagnostics/dogfood-20260519T150149Z/`
- Progress marker bundle: `/home/brittonr/git/aspen/target/runtime-proof/vmci-diagnostics/dogfood-20260519T150149Z/progress-markers.txt`
- Rail: `vmci-nix-timeout`
- Final build failure: `build:executor_command` / `executor_timeout`
- Build stage elapsed: about `25s`, not a `ci_wait_timeout`
- Diagnostic summary:
  - `vm_ci_boundary=job_result_published`
  - `vm_ci_failure_class=post_registration_ci_execution`
  - `post_registration=true`
  - evidence includes `pre_executor_progress,workspace_materialized,executor_started,job_result_published`

Preserved progress markers now prove the cheap rail reaches and publishes the intended failure:

- `nix_payload_transform_done`
- `executor_enter`
- `source_blob_fetch_done`
- `archive_decode_enter/done`
- `workspace_unpack_enter/done`
- `workspace_preflight_done`
- `workspace_ready`
- `command_request_built`
- `command_execute_enter`
- `command_started timeout_secs=5`
- `command_timeout timeout_secs=5`
- `command_execute_returned`
- `result_publish_enter`
- `result_published`

Current interpretation: the original VMCI issue was not VMCI transport and is no longer a pre-transform/pre-executor stall. The root cause class was missing/fragile executor timeout and result-finalization observability: command timeout/failure could leave CI appearing `running` because progress/result publication markers were truncated or cleared before the central log bridge retained them. The cheap VMCI timeout rail now proves timeout finalization and failed job publication instead of silent running. Next live proof is the medium rail to confirm `build-cli` also publishes success/failure or a specific phase-classified failure.

## Medium cache-warm hidden-timeout evidence

Latest medium proof after guest-local Nix store, cache-warm split, and wait-budget alignment:

- Command: `nix run .#dogfood-local-vmci-medium`
- Log: `target/runtime-proof/vmci-rerun/medium-after-cache-warm-20260521T142524Z.log`
- Receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260521T143659Z.json`
- Diagnostics: `target/runtime-proof/vmci-diagnostics/dogfood-20260521T143659Z/`
- Final status: exit code 1; `format-check` passed, `cache-warm/build-cli-deps` failed.
- Failure before classifier correction: `build:executor_command` / `executor_timeout` after about `765s` even though the job payload allowed `timeout_secs=2400`.
- Missing timeout-origin markers: no `command_timeout`, no `executor_job_timeout`, no `origin=select`, no `origin=guard`, no `origin=vmci_executor_wrapper`.
- Nix was still producing build progress near failure.

Interpretation: the receipt label `executor_timeout` was over-broad. It was produced without any executor timeout-origin marker, so it cannot be treated as proof that the command runner or VMCI executor watchdog fired. The patch direction is to require timeout-origin markers before classifying a build failure as `executor_timeout`, classify command progress without timeout-origin as `executor_command_failed`, preserve bounded failure-log tails, and add queue visibility extension acceptance/rejection markers so the next run can distinguish queue lease expiry/status mapping from true executor timeout.
## VMCI smoke rail after startup readiness and host-client route fixes

Live proof after adding VM-CI local startup readiness, relaxing the startup health RPC to best-effort after local readiness, stopping the local-readiness gate from waiting for worker registration before cluster init, and keeping Forge push RPCs on the relay-disabled direct client path:

- Command: `/nix/store/5sdw4k96kyh6x2b0m88lb9pfz67lhw4p-dogfood-local-vmci-smoke`
- Log: `/home/brittonr/git/aspen/target/runtime-proof/vmci-rerun/smoke-after-forge-direct-client-20260519T233927Z.log`
- Receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260519T233927Z.json`
- Rail: `vmci-smoke`
- Final status: `start=succeeded`, `push=succeeded`, `build=succeeded`, `stop=succeeded`
- Stage timing: `start` about 39.6s, `push` about 23.5s, `build` about 7.3s, `stop` about 2.2s
- Build result: `smoke` / `workspace-materialization-smoke` succeeded.

Interpretation: the latest blocker after the pre-executor/materialization/timeout fixes was host-side orchestration, not guest pre-transform. VM-CI startup had been gated on a fragile ticket health RPC and then on worker registration before cluster initialization; after local node readiness, Forge still needed a relay-disabled/direct client to avoid n0 relay/DNS discovery in the offline same-host VM-CI path. The smoke rail now proves startup, Forge repo creation/push, CI watch/auto-trigger, source snapshot/archive path, guest worker registration/job execution, workspace materialization, and job result publication for the minimal workspace-materialization rail. The remaining full proof is the medium rail to determine whether `build-cli` now completes or yields a specific phase-classified failure under the heavier Nix build workload.

## Medium rail after selective input path rewrite

Latest medium proof after Octet lock-original preservation and selective path rewrite:

- Command: `nix run .#dogfood-local-vmci-medium`
- Log: `target/runtime-proof/vmci-rerun/medium-after-selective-input-path-rewrite-20260520T160845Z.log`
- Receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260520T162316Z.json`
- Final status: exit code 1; `format-check` passed, `build-cli` failed.
- Proven boundaries: VM-CI local startup/readiness, source push, tracked WIP overlay (`files=45`), CI trigger, workspace materialization, executor command start.
- Failure signature:

```text
copying path '/nix/store/i2gsp87gqp16whm9mw0ybk9n84zir01x-source' from 'https://cache.nixos.org'...
error: chmod ".../adoptopenjdk-icedtea-web/patches": Too many open files in system
unpacking 'github:NixOS/nixpkgs/b86751bc4085f48661017fa226dee99fab6c651b?...' into the Git cache...
```

Interpretation: the `resolve-vmci-nix-build-execution-stalls` timeout/finalization work succeeded in making the job publish a final failure instead of silently staying `running`. The remaining root cause has moved to a new active change, `avoid-vmci-nix-store-fd-pressure`: VMCI guest Nix still materializes/traverses large public source inputs such as `nixpkgs` through a source/store boundary that exhausts system file handles after `command_started`. Phase 1 of the new change added dogfood diagnostic class `nix_source_store_fd_pressure` with exact stderr-shape and redaction tests.
