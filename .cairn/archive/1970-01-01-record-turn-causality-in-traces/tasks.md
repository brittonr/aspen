# Tasks: Record turn causality in traces

## Vocabulary

- [x] [serial] Record the current trace record census and the transition sites that can produce each cause. r[molten.runtime_spine.turn_causality]
- [x] [serial] Add the closed cause vocabulary to the committed turn record and include it in the canonical record identity. r[molten.runtime_spine.turn_causality]
- [x] [serial] Record the facet start and stop actions in the turn record, and settle the `spawn` and `link` action kinds. r[molten.runtime_spine.turn_causality]
      The facet start and stop actions are recorded as the `facet-created` and `facet-stopped` turn events, carrying the
      facet identity and its parent, so the facet path is the parent chain. The facet identity is always present: the
      facet API requires it, so the identity-unavailable case does not arise. The `spawn` and `link` kinds stay
      unimplemented: the runtime has no spawn or link behavior to record, and a kind no turn can produce would be a
      vocabulary entry without evidence. The accepted requirement covers the cause vocabulary only.

## Validation

- [x] [parallel] Add positive tests: a dependency-driven turn names the turn that released it; a cleanup turn names `cleanup`; a delayed turn names its causing turn and amount. r[molten.runtime_spine.turn_causality]
- [x] [parallel] Add negative tests: a turn record without a cause fails validation, an unknown cause is rejected, a truncated trace fails closed, and a replay whose cause differs compares as divergent. r[molten.runtime_spine.turn_causality]
- [x] [serial] Re-record any recorded trace fixtures that lack a cause and list the moved fixtures. r[molten.runtime_spine.turn_causality]
- [x] [serial] Run `cargo test -p molten` and focused Clippy before and after the change, then the workspace checks the repository requires for tracing edits. r[molten.runtime_spine.turn_causality]
- [x] [serial] Document the cause vocabulary, the validation rule, and the trace-completeness non-claim. r[molten.runtime_spine.turn_causality]
