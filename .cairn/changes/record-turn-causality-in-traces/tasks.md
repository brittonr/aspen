# Tasks: Record turn causality in traces

## Vocabulary

- [x] [serial] Record the current trace record census and the transition sites that can produce each cause. r[molten.runtime_spine.turn_causality]
- [x] [serial] Add the closed cause vocabulary to the committed turn record and include it in the canonical record identity. r[molten.runtime_spine.turn_causality]
- [ ] [serial] Add the `spawn`, `link`, `facet-start`, and `facet-stop` action kinds, using the facet path when facet identity is unavailable. r[molten.runtime_spine.turn_causality]
      The facet action kinds already exist in the turn action vocabulary, and the cause vocabulary covers the causes
      the current runtime can produce. The `spawn` and `link` action kinds stay open: the runtime has no spawn or
      link behavior to record, and adding the kinds without the behavior would be a vocabulary that no turn can
      produce.

## Validation

- [x] [parallel] Add positive tests: a dependency-driven turn names the turn that released it; a cleanup turn names `cleanup`; a delayed turn names its causing turn and amount. r[molten.runtime_spine.turn_causality]
- [x] [parallel] Add negative tests: a turn record without a cause fails validation, an unknown cause is rejected, a truncated trace fails closed, and a replay whose cause differs compares as divergent. r[molten.runtime_spine.turn_causality]
- [x] [serial] Re-record any recorded trace fixtures that lack a cause and list the moved fixtures. r[molten.runtime_spine.turn_causality]
- [x] [serial] Run `cargo test -p molten` and focused Clippy before and after the change, then the workspace checks the repository requires for tracing edits. r[molten.runtime_spine.turn_causality]
- [x] [serial] Document the cause vocabulary, the validation rule, and the trace-completeness non-claim. r[molten.runtime_spine.turn_causality]
