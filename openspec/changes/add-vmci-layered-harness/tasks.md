## Phase 1: Harness Contract

- [x] [serial] Define VMCI rail enum/profile mapping for shell, Nix, Cargo, source/blob, medium build, clippy, and full acceptance rails.
- [x] [serial] Add or normalize CLI/Nix app entrypoints so each rail can be invoked with one stable command.
- [x] [parallel] Add focused tests that rail selection never maps smoke rails to the full CI workspace graph unless the full rail is selected.

## Phase 2: Phase Receipts and Classification

- [x] [serial] Add a bounded VMCI phase receipt model with schema/version, rail name, phase timestamps, status, last boundary, CI run id, and diagnostics handles.
  - [x] Record each stage as `running` before invoking it so interrupted/killed full VMCI runs retain the last active boundary in memory/diagnostics summaries.
- [ ] [serial] Instrument dogfood cluster, Forge repo creation, source push/archive, CI trigger, VM registration, job assignment, workspace materialization, executor command, and job result boundaries.
- [x] [parallel] Add phase-specific timeout classification for source push/archive/trigger stalls distinct from VM registration, workspace materialization, executor, and full CI failures.
  - [x] Classify interrupted/running `push` stages as `forge_source_push_or_archive` instead of a generic unknown/full-CI failure.
  - [x] Classify source snapshot/archive preparation failures under the push source boundary.
  - [x] Classify build failures into VM registration, workspace materialization, executor command, and CI wait timeout categories from bounded CI status/detail evidence.
- [x] [parallel] Add redaction tests proving receipts/progress markers exclude raw tickets, credential-like values, command environments, and unbounded raw command arguments.

## Phase 3: Validation

- [x] [depends:phase-receipts] Run focused unit tests for rail mapping, phase receipt serialization, redaction, and failure classification.
  - Verified with `nix run .#rustfmt`, `cargo test -p aspen-dogfood vmci -- --nocapture`, `cargo test -p aspen-dogfood redacts -- --nocapture`, `cargo test -p aspen-dogfood build_receipts_classify_vmci_failure_boundaries -- --nocapture`, `openspec validate add-vmci-layered-harness --strict --json`, and `git diff --check`.
- [x] [depends:phase-receipts] Run live VMCI shell, Nix, and Cargo smoke rails and capture receipt paths as evidence.
  - Shell receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260518T051758Z.json` (`vmci-smoke`, CI run `b19a5147-e135-400a-9898-fd1bf2a1b1dc`).
  - Source/blob receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260518T053437Z.json` (`vmci-source-blob`, CI run `4dcb4187-ddb1-4fe7-b2a7-e09c3e41429c`).
  - Nix receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260518T053533Z.json` (`vmci-nix-smoke`, CI run `583f4c6e-4c67-42e7-b6f3-52d76d50c6d1`).
  - Cargo receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260518T053703Z.json` (`vmci-cargo-smoke`, CI run `0655579a-dead-4f8e-b79b-755ca0c87322`).
- [x] [depends:smoke-rails] Run or resume full VMCI acceptance and verify the receipt classifies any remaining full-run stall at the correct boundary.
  - Full receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260518T064205Z.json` (`vmci-full`) classified the remaining stall as `build:ci_wait` / `ci_wait_timeout`; VMCI startup, Forge push, source blob materialization, and guest worker execution were proven before full CI timed out on the clippy/check stage.
- [x] [depends:validation] Run the new medium build rail and/or dedicated clippy rail to prove the split full-runtime boundary without rerunning all full CI gates.
  - Medium receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260518T135733Z.json` (`vmci-medium`) proved startup/push/worker execution and failed at `build:ci_wait` because the wrapper wait timeout (`1200s`) was shorter than the rail command timeout (`3600s`).
  - Medium rerun receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260518T143256Z.json` (`vmci-medium`) used the corrected `3900s` wrapper wait timeout; `format-check` succeeded and `build-cli` remained running, narrowing the blocker to post-registration CI execution rather than VMCI plumbing.
  - Diagnostic evidence: `/home/brittonr/git/aspen/target/runtime-proof/vmci-diagnostics/dogfood-20260518T143256Z/` reported `vm_ci_boundary=worker_registered` and `vm_ci_failure_class=post_registration_ci_execution`; node log also showed a stale queue redelivery for already-running job `833c1d99-1d68-4662-9d0b-031c7748f68d`.
- [ ] [depends:validation] Update operator documentation or AGENTS notes with the preferred VMCI harness debug order and receipt locations.
