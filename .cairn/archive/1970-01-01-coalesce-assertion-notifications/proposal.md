# Proposal: Coalesce equal-assertion notifications

## Why

`RuntimeState` keys assertions by `(actor, value)` in an ordered set
(`src/runtime/dataspace/state.rs`; `RuntimeAssertion` at `src/runtime/turn/mod.rs:127`), and `stage_step` emits
`Event::AssertionObserved` once per matching owner. Two actors that assert an equal value therefore deliver two
notifications for one visible fact.

The manual requires the opposite: "assertion of a value is idempotent: multiple assertions of the same value appear
to observers indistinguishable from a single assertion" (`07-syndicated-actor-model.md → Dataspaces`). The reference
harness follows that rule because it counts assertions in `syndicate::bag::BTreeBag`
(`src/runtime/dataspace/syndicate.rs`), so the product runtime and its own parity harness disagree. No test covers
duplicate assertions (`src/runtime/dataspace/tests.rs`).

The tracey baseline lists `molten.runtime_spine.assertion_lifetimes` and `molten.runtime_spine.observe_patterns` as
accepted and implementation-unestablished, so this change closes accepted scope instead of opening new scope.

## What Changes

- Deliver one observer notification per visible assertion value, regardless of how many owners hold an equal value.
  r[molten.runtime_spine.assertion_notification_coalescing]
- Notify a retraction only when the last owner withdraws. A single owner's retraction leaves the visible value and the
  notification count unchanged.
- Apply the same rule when `Observe` registration delivers existing matches to a new observer.
- Keep per-owner bookkeeping internal, so per-owner retraction, duplicate owner entries, and owner-scope cleanup keep
  their current removal behavior.

## Impact

- **Files**: `src/runtime/dataspace/state.rs`, `src/runtime/dataspace/tests.rs`, `docs/architecture.md` dataspace
  section.
- **Testing**: positive coalescing and last-owner-retraction cases; negative cases for distinct values, duplicate
  owner entries, and cleanup of one of two owners; `cargo test -p molten` before and after; focused Clippy.
- **Non-goals**: no change to ownership, retention, authority, message delivery, or the reference harness, and no new
  wire or receipt schema.
