# Tasks: Exercise a service recovery composition end to end

## Composition definition

- [x] [serial] Define the three-service composition (durable stateful service, dependent worker, independent sibling) once through the typed facades and recovery-group declaration, consumable by both harness tracks. r[molten.testing.service_recovery_composition]

## Deterministic matrix

- [x] [serial] Add the deterministic scenario matrix asserting the five properties (unaffected services continue; stale instances lose influence; durable work is not silently lost; uncertain effects are not blindly repeated; storms terminate within the window) across member failure, delayed events, readmission, saturation, overload, storm, and upgrade scenarios. r[molten.testing.service_recovery_composition]
      All eight scenarios are implemented. The CPU-saturation scenario consumes the admitted time-sliced profile from
      `admit-time-sliced-component-execution`, now archived, and drives modeled work and a real Wasmtime callback
      through the admitted FIFO scheduler. The independent sibling completes between callback slices; the callback
      remains within its execution budget. Ignoring yields makes the sibling-progress property fail.
- [x] [parallel] Add the deliberately broken variants (fencing removed, group budget reset) and prove the harness fails each while the correct composition passes. r[molten.testing.service_recovery_composition]
      The slicing-disabled variant additionally fails the unaffected-services property for modeled and real Wasmtime
      work while both variants complete their bounded callback.

## Live twin

- [ ] [serial] Add the real-process twin covering crash-restart fencing, durable readmission, storm termination, and the generation upgrade, with observations mapped to the deterministic assertion vocabulary and timing checks on admitted observations. r[molten.testing.service_recovery_composition]
      The separately authorized `host-node-recovery-groups` prerequisite now implements the actual
      capability-rooted node hosting shell, real native lifecycle/upgrade bridge, exact current-authority
      public operations and retained post-source-admission serving-node lifecycle. The composition
      remains a consumer; no runtime mechanism is implemented inside its harness.
      `tests/nativesystemextension/recovery.rs` and its `recovery/` modules launch the independently
      source-admitted actual node, sign genuine exact UCAN load/operation credentials, install actual
      callback artifacts, wait for that current child's canonical hosted-ready observation, SIGKILL
      the same serving process and reopen its real Redb/native journals. They cover incarnation/claim
      fencing, retained durable work and explicit readmission, restart history/storm termination,
      materialized drain/checkpoint/recover/activation, a second activated-generation process loss,
      explicit migrated work and independent sibling callbacks using the shared property evaluator.
      Concrete fault-build mutations are in `tests/fixtures/service-recovery-faults/`; their live rail
      requires actual independently admitted good/fault source outputs and artifact cohorts, not a
      test-owned supervisor or fake gate observations. Those authoritative cohorts are not supplied:
      all documented source/cohort environment variables were absent in this worker. The parent owns
      the real Nix producer/signing rail. Both external tests are explicitly ignored in ordinary runs
      and fail when selected without the actual artifacts. No live scenario has been run or claimed;
      this task and lifecycle closure remain open until the real positive and broken rails execute.

## Validation and closeout

- [ ] [serial] Run the composition suite in both tracks, formatting, Clippy, Octet, Cairn validation, and the proposal, design, and tasks gates. r[molten.testing.service_recovery_composition]
      Earlier deterministic-track formatting, Clippy, Octet, workspace tests and strict Cairn evidence
      remain historical evidence. The new drained/checkpointed/materialized-upgrade assertions and
      `MigrationSkipped` negative variant have not been revalidated by this worker. The live track and
      current-source final checks remain open; no build, test, lint, formatter or gate was run here.
- [x] [serial] Retain the no-whole-system-correctness and no-mechanism-patching non-claims before sync or archive. r[molten.testing.service_recovery_composition]
