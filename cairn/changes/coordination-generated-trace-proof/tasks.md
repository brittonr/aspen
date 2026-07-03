# Tasks: coordination-generated-trace-proof

## Phase 1: Trace generator

- [ ] [serial] r[molten.coordination_state_machine_proof.generated_traces] Add bounded generated coordination trace inputs for implemented lock, queue, semaphore, rate-limit, election, and barrier operations.
- [ ] [parallel] r[molten.coordination_state_machine_proof.generated_traces] Keep trace generation deterministic and bounded so failing cases can be replayed from the Hegel case seed.

## Phase 2: Invariant assertions

- [ ] [parallel] r[molten.coordination_state_machine_proof.generated_traces] Assert mutual exclusion, fencing-token monotonicity, FIFO queue behavior, semaphore capacity bounds, barrier release thresholds, and election winner consistency after generated steps.
- [ ] [parallel] r[molten.coordination_state_machine_proof.deny_no_mutation] Assert denied generated requests leave the coordination state ref unchanged and emit deny receipts.
- [ ] [parallel] r[molten.coordination_state_machine_proof.duplicate_no_advance] Assert duplicate generated operation ids return the prior receipt ref and do not advance state a second time.

## Phase 3: Evidence

- [ ] [serial] r[molten.coordination_state_machine_proof.generated_traces] r[molten.coordination_state_machine_proof.deny_no_mutation] r[molten.coordination_state_machine_proof.duplicate_no_advance] Add traceability evidence and run `cargo test coordination`.
