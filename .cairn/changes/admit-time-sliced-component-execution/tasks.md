# Tasks: Admit a time-sliced component execution profile

## Profile admission

- [x] [serial] Add the `time-sliced` execution-profile variant binding quantum, total budget, and the deterministic scheduling contract, admitted through the existing component profile vocabulary without changing existing profile identities. r[molten.wasm_component.execution_quantum]
- [x] [parallel] Add negative admission fixtures: time-sliced request under a synchronous profile is denied; missing quantum or budget binding is denied. r[molten.wasm_component.execution_quantum]

## Sliced execution

- [ ] [serial] Implement quantum-based yielding under the new profile using the pinned cohort's async entrypoints, with fuel consumed per slice and the continuation re-enqueued through the admitted scheduler. r[molten.wasm_component.execution_quantum] r[molten.wasm_component.yield_continuation]
      The delivered contract covers the admitted profile, the admission boundary, the slice ledger, the pending turn, the
      continuation identity, and the replay comparison. The guest-side yielding stays open: the component's `invoke`
      export is a synchronous WIT export, and the pinned cohort's fuel-based yielding needs the async entrypoints that an
      async component ABI would introduce.
- [ ] [parallel] Add positive fixtures: a CPU-heavy component yields at least once while an unrelated runnable completes between its slices, and total execution stops at the budget with the existing fuel-exhaustion classification. r[molten.wasm_component.execution_quantum]
      These fixtures need the guest-side yielding above.
- [ ] [parallel] Add yield-continuation fixtures: a callback that yields resumes from its continuation, commits its full turn exactly once through existing admission boundaries, and never publishes partial state or reruns from the start. r[molten.wasm_component.yield_continuation]
      The pending turn and the continuation identity are delivered and tested; the resumed-execution fixture needs the
      guest-side yielding above.

## Replay and closeout

- [x] [serial] Record quantum boundaries and scheduler choices in the deterministic trace and prove replay consumes them, failing with a boundary-named mismatch when the recorded and recomputed paths diverge. r[molten.wasm_component.yield_continuation]
- [ ] [serial] Run focused component-runtime and scheduler tests, formatting, Clippy, Octet, Cairn validation, and the proposal, design, and tasks gates. r[molten.wasm_component.execution_quantum] r[molten.wasm_component.yield_continuation]
      The focused component-runtime gates are recorded in the change's evidence. The scheduler-track and live-execution
      gates stay open with the guest-side yielding.
- [x] [serial] Retain the no-elapsed-time, no-default-adoption, and no-added-isolation non-claims before sync or archive. r[molten.wasm_component.execution_quantum]
