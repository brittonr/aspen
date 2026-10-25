# Time-sliced component execution: guest-side yielding

## Correction to the recorded blocker

The earlier evidence held that fuel-based yielding needs an async component
ABI. In the pinned Wasmtime 45 cohort it does not:

- `Config::async_support` is a deprecated no-op; async support comes with the
  default `async` feature.
- `Store::fuel_async_yield_interval` suspends a synchronous guest through the
  `*_async` entrypoints.

The guest's `invoke` export therefore stays a synchronous WIT export. The host
drives it through the asynchronous binding (`bindgen!` with
`exports: { default: async }`) and polls it one slice at a time with a no-op
waker.

## Delivered

- `src/wasm/component/runtime/sliced.rs`:
  - instantiation under the time-sliced profile, with the store budget set to
    the total budget and the yield interval set to the quantum;
  - a pending poll records a quantum boundary;
  - completion records the final partial slice, so the ledger's consumption
    equals the fuel the store spent;
  - a trap at a spent budget keeps the existing fuel-exhaustion
    classification.
- `src/wasm/component/runtime/slicedshell.rs`: `start_sliced_execution` and
  `SlicedExecution::run_slice`.
  - Admission, materialization, and the inspection, instantiation, and
    execution receipts are shared with synchronous execution through the
    extracted `admit_request`.
  - A yield returns the continuation's requeue command, and completion
    publishes through `PendingTurn` exactly once.
- **`ContinuationKey::requeue_command` fix.** It now returns the scheduler's
  `Yield` command. The previous `Wake` command is refused for a running
  runnable with `DuplicateRunnable`, so a yielded continuation could not have
  re-entered the admitted scheduler.
- **Execution-plan admission fix.** Admission accepted only the synchronous
  profile id, so the admitted time-sliced profile could not plan an
  execution. It now admits either admitted component profile id, provided it
  equals the executing profile.

## Fixtures

`cargo test -p molten --lib -- slicing component`: 45 passed, including:

- `a_cpu_heavy_callback_yields_while_an_unrelated_runnable_completes_between_its_slices`:
  - The heavy continuation and an unrelated runnable go through the admitted
    `fabric_time` scheduler: `apply_scheduler_command` and `choose_runnable`.
  - The heavy callback yields first, and the sibling completes before the
    heavy callback completes.
  - The output equals the input, and the ledger has at least two boundaries.
  - The ledger's consumption equals the budget minus the fuel remaining in
    the execution receipt.
- `a_sliced_execution_stops_at_the_budget_with_the_fuel_exhaustion_classification`:
  the spinning guest yields, then stops with a denial receipt whose trap class
  is `fuel-exhausted`.
- `a_resumed_continuation_commits_its_turn_once_and_never_reruns_from_the_start`:
  - The turn passes once, and another slice is refused.
  - The sliced run spends exactly the fuel of the same synchronous run.
  - A replayed sliced run's ledger compares equal.
- `a_synchronous_request_cannot_start_a_sliced_execution`.

## Non-claims

- A quantum is a work-accounting bound, not elapsed CPU time or a latency
  guarantee.
- Yielded continuations do not prove fairness of the whole system.
- Time slicing adds no memory or authority isolation.
- Time slicing is not the default for any existing profile.

## Gates run

- `cargo test --workspace --lib`: 1737 (`molten`) and 619 (`molten-core`)
  passed. `cargo test -p molten --lib -- slicing component`: 45 passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean. `cargo octet
  check`: clean.
- Cairn sync added the accepted `wasm-component-runtime` requirements. The
  tracey guard passed with 2955 requirements, 1021 referenced, and 0 dangling.
