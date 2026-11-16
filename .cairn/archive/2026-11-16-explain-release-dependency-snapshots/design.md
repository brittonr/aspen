## Context

`validate_release_dependencies` indexes observations by key (the last occurrence wins), compares fields, sorts and deduplicates diagnostics, and builds canonical material from code, subject, and message. Its `supporting_facts` strings name values but not the row occurrence or snapshot, and the validator cannot tell a completely collected relation from a partial one. Issue #11 and its adoption review require a corpus, explicit refusal outcomes, snapshot-bound bounded references, and a measured comparison with a relational alternative before any engine is adopted.

## Decisions

### Decision: A separate pure explanation view, not a validator change

**Choice:** Add `release_dependency::explanation` beside the validator. It re-derives only the two selected families with the validator's key and last-occurrence semantics and calls the validator for the same snapshot only to mirror its decision. Validator diagnostics, canonical material, and the release-policy shell output are untouched.

**Rationale:** Adding occurrence references to the validator's diagnostics would change their ordering, deduplication, and the shell's deny output. A view keeps the validator as the sole authority and lets the comparison test that both agree.

### Decision: Refusals carry no report

**Choice:** `Unavailable`, `Incomplete`, `Unsupported`, and `BudgetExhausted` are distinct variants without a report; `admits_release` is true only for `Completed` or `Truncated` with a same-snapshot validator pass. Checks run in a fixed order: limits, row bounds, collections, fact budget, step budget, depth, then output truncation.

**Rationale:** A caller cannot read a pass from an outcome that has no report. Truncation only limits display, so it keeps the report and counts omitted rows.

### Decision: Occurrence-exact identities and support

**Choice:** Snapshot identity is domain-separated BLAKE3 over length-framed fields in input order with label-encoded enums, following the core's existing framed-identity pattern (molten-core has no Preserves dependency). Support references carry relation, occurrence index, field, and value, or an absent key over a completed relation. Verification recomputes the identity and resolves every reference.

**Rationale:** Sorted, deduplicated canonical material would erase the duplicate rows the issue requires the projection to keep. Values in the references let verification reject a reference rebound to a corrected snapshot.

### Decision: Measure against a test-only relational pilot

**Choice:** Implement the minimal alternative as a test-only evaluator: occurrence-keyed set relations, stratified negation for superseded rows and absence, naive nested-loop joins to a fixpoint. Count steps, facts, and materialized bytes deterministically; measure wall time with the test runner's per-test durations of two cost tests that run the same corpus 2,000 times with identity, validation, and rendering on both paths.

**Rationale:** The issue forbids assuming an engine or a performance benefit. A hand-written pilot adds no dependency and isolates the relational formulation; molten-core reads no clock, so timing stays in the runner.

### Decision: Keep structured Rust; stop engine integration

**Choice:** Keep the structured Rust path and do not integrate a rule engine for these families unless the captured cost result shows the pilot cheaper.

**Rationale:** The tests show identical explanations; the pilot needs the same occurrence, ordering, completeness, and budget machinery plus 456 lines of evaluator against 164 lines of Rust derivation, and its absence rule silently derives nothing without a completed fact.

## Measurement capture

Run from the change worktree after integration, with no other build sharing the target directory:

```sh
cargo nextest run -p molten-core --no-run
cargo nextest run -p molten-core --no-capture --test-threads 1 \
  -E 'test(/release_dependency::explanation::comparison::/)'
```

Retain the comparison table and the two `explanation_cost_*` durations under `evidence/` with the commit or diff identity, toolchain, and runner versions, with host paths replaced by the publication placeholders, and check it with `scripts/evidence-publication-guard.py`.

## Risks / Trade-offs

- Debug-profile wall time over a 12-case corpus is indicative only; the deterministic step counts are the comparable cost measure.
- The view re-runs the validator per evaluation to mirror its decision, which costs one extra validation for callers that already hold a report.
- Rule identities are review text, not cryptographic identities.
