# Proposal: Admit a time-sliced component execution profile

## Why

Static review of `src/wasm/component/runtime.rs` at `6c7158db6` found that component invocation uses a synchronous `call_invoke`, configures fuel consumption, and assigns one total fuel budget. Nothing in this path yields execution periodically. Fuel exhaustion terminates a runaway computation; it does not give other runnable work a chance while a CPU-heavy callback runs.

Aspen's pure scheduler (`fabric_time/scheduler`) answers which runnable is selected next. It does not answer how long a selected runnable may occupy execution before yielding. BEAM's reduction accounting distinguishes these two questions, and the distinction is the transferable lesson: selection fairness is not execution fairness.

## What Changes

- Add a separately admitted `time-sliced` component execution profile with two independent controls: a scheduling quantum (bounded guest execution before yielding) and a total execution budget (maximum work before the callback fails).
- Evaluate integration against the pinned Wasmtime cohort, using fuel-based asynchronous yielding, which requires asynchronous execution entrypoints and store configuration; do not silently substitute into the existing synchronous component profile.
- Require that yielded callbacks preserve their execution continuation in the shell, return to Aspen's admitted scheduler for re-selection, and commit pending state changes and outgoing effects only through existing turn and admission boundaries. A yielded callback must not publish half a semantic turn or rerun from the beginning.
- Keep epoch interruption out of the scheduling semantics; treat it, if adopted later, as a watchdog mechanism with explicit replay handling.
- Leave host-function blocking bounds to the existing bounded host-effect contracts; fuel and quanta do not solve an indefinitely blocking host call.

## Impact

- **Files**: component runtime profile admission, the component invoke path, scheduler handoff, and profile fixtures and tests.
- **Testing**: a CPU-heavy component yields under the new profile while unrelated work continues; total execution remains bounded; quantum exhaustion, budget exhaustion, and yield-then-resume continuations are exercised; replay follows the declared scheduling contract.
- **Non-goals**: no change to the existing synchronous profile's behavior for extensions admitted against it, no claim that the quantum approximates elapsed CPU time, and no new execution-isolation promises for trusted in-process native code.

## Dependencies

- Pinned Wasmtime cohort and `wasm-component-runtime` profile admission.
- `fabric_time` scheduler admission boundaries for re-selection of yielded continuations.
