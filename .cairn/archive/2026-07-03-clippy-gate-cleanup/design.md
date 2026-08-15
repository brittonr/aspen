# Design: clippy gate cleanup

## Scope

This change restores the repo-wide clippy validation gate before any new replay or release evidence is promoted. It is intentionally a narrow validation-readiness slice.

## Proof checklist

- **Proof claim**: the candidate tree can pass the standard Rust lint gate with warnings denied, after preserving existing runtime behavior.
- **Out of scope**: changing public CLI behavior, receipt schemas, runtime semantics, source-gate policy, or production-readiness claims.
- **Trusted assumptions**: clippy diagnostics identify syntactic or style issues, not semantic requirements.
- **Positive evidence**: `cargo clippy --all-targets -- -D warnings` passes, with `cargo fmt --check` and `cargo test` still passing.
- **Negative evidence**: pre-change clippy denial is recorded in the change notes or task evidence, and no lint-only edit may introduce a failing test.
- **Canonical refs**: not applicable for lint-only source cleanup; release review uses command outputs and subsequent dogfood evidence.
- **Regeneration command**: `cargo fmt --check && cargo test && cargo clippy --all-targets -- -D warnings`.

## Functional core

Keep edits semantics-preserving. Do not move validation logic across functional-core / imperative-shell boundaries except where required to remove a lint without changing outputs.

## Non-goals

- No release evidence regeneration in this change.
- No replay receipt schema changes.
- No Octet remediation or source-gate policy changes.
