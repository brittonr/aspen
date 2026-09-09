# Design: Admit a time-sliced component execution profile

## Context

`execute` in `src/wasm/component/runtime.rs` calls `session.bindings.call_invoke(&mut session.store, &guest_input)` synchronously, observes fuel only after the call returns or traps, and configures `consume_fuel(true)` with one `store.set_fuel(profile.resources.fuel)` budget. Wasmtime supports fuel-based asynchronous yielding, but reaching it requires asynchronous entrypoints (`call_invoke_async` or equivalent), an async store configuration, and yield handlers; the pinned cohort and profile identity must admit that combination explicitly.

## Approach

1. **Profile admission.** Extend the component execution-profile vocabulary with a `time-sliced` variant that binds `quantum` (fuel units per slice), `total_budget` (fuel units for the whole callback), and the deterministic scheduling contract used for replay. Admitting the variant is a profile-identity change; existing profiles are untouched.
2. **Async entrypoints only under the new profile.** The time-sliced profile configures async store/entrypoint paths and installs a fuel-exhaustion yield handler. The synchronous path stays as-is for the existing profile; a manifest requesting time-sliced execution under a synchronous profile is an admission denial.
3. **Continuation in the shell.** On quantum exhaustion, the guest future yields. The shell records the fuel consumed, re-enqueues the callback continuation with the admitted scheduler (which selects it like any runnable, subject to priorities and fairness bounds), and never commits partial turn state. On resume, execution continues from the yield point with the remaining budget. Budget exhaustion at any yield point classifies as the existing fuel-exhaustion denial.
4. **Replay contract.** The recorded scheduling choices and quantum boundaries are part of the deterministic trace. Replay consumes recorded choices; a replay mismatch names the boundary where the recorded and recomputed paths diverge.
5. **Isolation honesty.** The profile documentation and non-claims keep the three execution domains distinct: trusted in-process native code does not acquire BEAM-like isolation by being scheduled fairly.

## Alternatives considered

- Epoch interruption as the slicing mechanism. Rejected for scheduling semantics: epoch advancement timing depends on the host clock thread, so slices are not deterministic. Acceptable later as a watchdog with explicit replay handling; not part of this change.
- Lowering the total fuel budget only. Rejected: a budget bounds total work but never yields; long-budget callbacks still starve unrelated work.
- Making time slicing the default for all components. Rejected: it changes the execution strategy bound in existing admitted profiles and forces async entrypoints on every consumer; adoption stays per-profile.

## Non-claims

- A quantum is a work-accounting bound, not elapsed CPU time or a latency guarantee.
- Yielded continuations re-entering the scheduler do not prove fairness of the whole system; fairness remains scoped to the admitted scheduler policy.
- Time slicing provides no additional memory or authority isolation.

## Risks

- Async entrypoints change the store configuration surface; the pinned cohort may need a new profile identity. The change admits a new identity rather than mutating pinned ones.
- Yield handling interacts with host calls made from within the guest; host effects keep their own bounded, cancellable contracts, and a yield during a host call resolves the host call first.
