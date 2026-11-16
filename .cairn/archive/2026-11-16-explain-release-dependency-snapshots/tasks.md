## Phase 1: Implementation

- [x] [serial] Add the pure explanation view with declared collections, limits, and the `Completed`, `Truncated`, `Unavailable`, `Incomplete`, `Unsupported`, and `BudgetExhausted` outcomes; refusals carry no report or pass. r[molten.project.reproducible_dependencies.explanation_outcomes]
- [x] [serial] Bind explanations to the occurrence-exact snapshot identity, rule revision, collections, and limits, and verify every support reference independently. r[molten.project.reproducible_dependencies.explanation_references]
- [x] [serial] Add the fixture corpus, the test-only relational pilot, the agreement test, and the two cost tests. r[molten.project.reproducible_dependencies.explanation_comparison]
- [x] [serial] Document the model, outcomes, comparison, and keep/stop decision in `docs/release-dependency-explanations.md`. r[molten.project.reproducible_dependencies.explanation_comparison]

## Phase 2: Validation

- [x] [serial] Run the focused `molten-core` explanation tests, Clippy with `-D warnings`, and `cargo fmt --check`. r[molten.project.reproducible_dependencies.explanation_outcomes]
- [x] [serial] Capture the comparison table and cost-test durations under `evidence/` with commands, toolchain, and source identity, and confirm or revise the keep/stop decision from them. r[molten.project.reproducible_dependencies.explanation_comparison]
- [x] [serial] Run strict Cairn validation before review, sync, and archive. r[molten.project.reproducible_dependencies.explanation_references]

The 2026-11-16 [ordered integrated execution](../../../evidence/integrated-cutover-2026-11-16/checks.json) passed `cargo test --locked --workspace --lib` (including the explanation selection), workspace/all-target Clippy with denied warnings, formatting and strict provider-policy Cairn validation. The comparison capture remains separately source-bound under this change's `evidence/`; neither record grants release authority.
