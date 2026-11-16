## Why

Issue #11 asks whether a small Datalog model would explain release-dependency failures better than the Rust validator. The earlier change added stable rule ids and textual support to `LockDrift` and `MissingArchiveReceipt`, but left the issue's acceptance open: there is no fixture corpus, no explicit incomplete, refused, or truncated outcome, explanation references name neither the snapshot nor the row occurrence and are not bounded by depth, facts, steps, or output, and the keep/stop decision was not measured against an alternative.

## What Changes

- A pure `release_dependency::explanation` module evaluates the two families from one snapshot with declared collection states and limits. Outcomes are `Completed`, `Truncated`, `Unavailable`, `Incomplete`, `Unsupported`, and `BudgetExhausted`; only the first two carry a report, and a release pass only mirrors the same-snapshot validator.
- Explanations name a refined rule and two occurrence-identified support references, bound to a length-framed BLAKE3 snapshot identity, the rule revision, collections, and limits. `verify_explanation_references` independently checks them against a snapshot.
- A test-only relational pilot (stratified, naive bottom-up evaluation over occurrence-keyed relations) and a fixture corpus compare both paths with the validator; cost tests let the runner measure wall time outside `molten-core`.
- `docs/release-dependency-explanations.md` records the model, outcomes, comparison, and the keep/stop decision.

## Impact

- **Files**: `crates/molten-core/src/release_dependency.rs` (module mount), `crates/molten-core/src/release_dependency/explanation.rs` and `explanation/{identity,verify,corpus,relational_pilot,comparison,tests}.rs`, `docs/release-dependency-explanations.md`, and the #11 row of `docs/open-issue-triage.md`.
- **Testing**: focused `molten-core` explanation tests (corpus agreement, distinctions, duplicates, crossed packages, replacement, refusals, budgets, truncation, identity), the comparison capture, Clippy, rustfmt, and strict Cairn validation.
- **Owner / consumer**: release-dependency maintainers and operators reading diagnostics. The validator, `molten-release-policy`, admission, canonical report material, and redaction are unchanged; no dependency is added.
