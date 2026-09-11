# Tasks: Record turn causality in traces

## Vocabulary

- [ ] [serial] Record the current trace record census and the transition sites that can produce each cause. r[molten.runtime_spine.turn_causality]
- [ ] [serial] Add the closed cause vocabulary to the committed turn record and include it in the canonical record identity. r[molten.runtime_spine.turn_causality]
- [ ] [serial] Add the `spawn`, `link`, `facet-start`, and `facet-stop` action kinds, using the facet path when facet identity is unavailable. r[molten.runtime_spine.turn_causality]

## Validation

- [ ] [parallel] Add positive tests: a dependency-driven turn names the turn that released it; a cleanup turn names `cleanup`; a delayed turn names its causing turn and amount. r[molten.runtime_spine.turn_causality]
- [ ] [parallel] Add negative tests: a turn record without a cause fails validation, an unknown cause is rejected, a truncated trace fails closed, and a replay whose cause differs compares as divergent. r[molten.runtime_spine.turn_causality]
- [ ] [serial] Re-record any recorded trace fixtures that lack a cause and list the moved fixtures. r[molten.runtime_spine.turn_causality]
- [ ] [serial] Run `cargo test -p molten` and focused Clippy before and after the change, then the workspace checks the repository requires for tracing edits. r[molten.runtime_spine.turn_causality]
- [ ] [serial] Document the cause vocabulary, the validation rule, and the trace-completeness non-claim. r[molten.runtime_spine.turn_causality]
