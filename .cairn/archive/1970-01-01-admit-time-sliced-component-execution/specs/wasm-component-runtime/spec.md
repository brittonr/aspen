# Wasm Component Runtime Delta

## ADDED Requirements

### Requirement: Time-sliced execution profile bounds execution before yielding
r[molten.wasm_component.execution_quantum] Molten MUST admit a time-sliced component execution profile that independently binds a scheduling quantum and a total execution budget. Under that profile, guest execution MUST yield control back to the admitted scheduler after at most one quantum of work, and MUST terminate with the fuel-exhaustion classification at the total budget. The existing synchronous profiles MUST remain unchanged, and a time-sliced request outside an admitting profile MUST be denied at admission.

#### Scenario: CPU-heavy component yields to unrelated work
- GIVEN a component callback admitted under the time-sliced profile runs longer than one quantum and an unrelated runnable is waiting
- WHEN the quantum is exhausted
- THEN the callback yields, the unrelated runnable is selected before the callback resumes, and the callback's total execution still stops at the budget.

#### Scenario: Budget exhaustion terminates
- GIVEN a component callback consumes its total execution budget across multiple slices
- WHEN the budget reaches zero
- THEN the callback fails with the existing fuel-exhaustion denial class and no further slices run.

#### Scenario: Admission denies mismatched profiles
- GIVEN a manifest requesting time-sliced execution under a synchronous-only profile
- WHEN admission evaluates the manifest
- THEN admission denies with a typed profile mismatch and no execution starts.

### Requirement: Yielded continuations preserve turn atomicity and replay
r[molten.wasm_component.yield_continuation] A yielded callback MUST preserve its execution continuation in the shell and MUST NOT publish partial semantic turns or rerun from its beginning on resume. Pending state changes and outgoing effects MUST commit only through existing turn and admission boundaries after the callback completes. Recorded quantum boundaries and scheduler choices MUST participate in the deterministic replay contract.

#### Scenario: Resume continues without republishing
- GIVEN a callback yielded after publishing no state changes
- WHEN the scheduler re-selects its continuation and it completes
- THEN its state deltas, messages, and effect intents commit exactly once as a full turn, and no partial publication occurred before completion.

#### Scenario: Replay matches recorded slicing
- GIVEN a recorded execution trace with quantum boundaries and scheduler choices
- WHEN the trace replays
- THEN the replay reproduces the recorded slicing, and any divergence fails at the first mismatching boundary.
