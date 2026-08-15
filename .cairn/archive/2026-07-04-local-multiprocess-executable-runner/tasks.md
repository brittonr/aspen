# Tasks: local-multiprocess-executable-runner

## Phase 1: Runner contracts

- [x] [parallel] r[molten.testing.multinode.local_multiprocess_executable_runner] Define the executable runner input and output contract around the existing pure local multiprocess plan and run receipt builders.
- [x] [parallel] r[molten.testing.multinode.local_multiprocess_runner_negatives] Extend pure validation for missing startup, workflow, shutdown, cleanup, stale ticket, timeout, orphaned process, and collision diagnostics.

## Phase 2: Thin shell implementation

- [x] [serial] r[molten.testing.multinode.local_multiprocess_executable_runner] Implement the runner shell that prepares isolated roots, spawns `molten node` processes, executes a bounded control workflow, collects receipts, and emits run evidence.
- [x] [serial] r[molten.testing.multinode.local_multiprocess_runner_negatives] Add cleanup handling that terminates children, detects orphans, records cleanup refs, and denies if cleanup fails.

## Phase 3: Positive and negative coverage

- [x] [parallel] r[molten.testing.multinode.local_multiprocess_executable_runner] Add a positive local runner test or check for a two-node cross-process control workflow.
- [x] [parallel] r[molten.testing.multinode.local_multiprocess_runner_negatives] Add negative tests for stale ticket, state-root collision, transport collision, missing workflow receipt, child timeout, orphaned process, and missing cleanup receipt.
- [x] [serial] r[molten.testing.multinode.local_multiprocess_executable_runner] Run focused local multiprocess runner tests and `cairn validate --root .`, or record the missing policy/host-support blocker and next best check.
