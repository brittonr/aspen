## Phase 1: Diagnostic contract

- [x] [serial] Add VM-CI post-registration diagnostics and workspace/blob progress requirements to the dogfood evidence contract.
- [x] [depends:contract] Inventory current host node, guest serial, dogfood receipt, and CI job log surfaces for markers that prove worker registration, job assignment, workspace materialization, executor start, and job result publication.

## Phase 2: Implementation

- [x] [depends:inventory] Implement a pure VM-CI run classifier that reports the highest reached boundary and separates connectivity regressions from post-registration CI execution stalls.
- [x] [depends:classifier] Add bounded workspace/blob/job progress evidence to VM executor or dogfood receipt/log output without exposing secrets.
- [x] [depends:evidence] Preserve redacted host/guest logs and receipt handles before VM-CI cleanup when a run reaches worker registration or job assignment but lacks final success.
- [x] [depends:evidence] Emit bounded command start/running/timeout progress markers for long-running VM CI commands without including command arguments or environment values.

## Phase 3: Verification and archive

- [x] [depends:implementation] Add positive and negative tests for connectivity regression, job-assigned timeout, workspace/blob materialization timeout, executor-started failure, redaction, and cleanup evidence preservation.
- [x] [depends:tests] Run focused `aspen-dogfood` and `aspen-ci-executor-vm` tests plus formatting/diff hygiene.
- [x] [depends:verification] Run strict OpenSpec validation for this change and all changes.
- [x] [depends:live-vmci] Run one clean live VM-CI dogfood retry and archive either a success receipt or a classified post-registration evidence bundle.
  - Full VMCI retry receipt `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260518T064205Z.json` reached source/blob materialization and VM executor command progress, then classified the remaining failure as `build:ci_wait` / `ci_wait_timeout` with `clippy` still running.
  - Medium VMCI retry receipt `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260518T143256Z.json` reached `format-check: success` and `build-cli: running` after `3900s`; diagnostics are archived at `/home/brittonr/git/aspen/target/runtime-proof/vmci-diagnostics/dogfood-20260518T143256Z/` with `vm_ci_boundary=worker_registered` / `post_registration_ci_execution`.
  - Follow-up fix: stale queue redelivery for already-running jobs is now consumed in the job handler so another worker cannot keep re-polling an in-flight `Running` job after `mark_started` rejects it.
