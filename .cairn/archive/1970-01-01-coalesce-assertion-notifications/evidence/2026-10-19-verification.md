# Coalesced assertion notifications verification

## Baseline

At source HEAD `e8286ce9c`, `cargo test -p molten --lib dataspace`
reported 23 passing tests. The existing lifecycle test expected one
`AssertionObserved` event for each owner of an equal value and one
`AssertionRetractionObserved` event for every owner retraction.

## Implementation

- A committed-value visibility decision gates `Assert`, `Retract`, and
  `Observe` notifications in `src/runtime/dataspace/parts/state/p001/body.rs`
  and `p002/body.rs`.
- Per-owner assertion records remain in the ordered set. An `Assert` notifies
  observers only when the value was not already visible; a `Retract` notifies
  only when the withdrawing owner was the last owner; `Observe` emits one
  notification per distinct matching value.
- A retraction from an actor that does not hold the assertion stages a
  `TurnRolledBack` event and no mutation, so it fails closed before commit.
- The Syndicate reference harness retains `BTreeBag` per-owner counting while
  gating visible notifications the same way
  (`src/runtime/dataspace/parts/syndicate/p000/body.rs`,
  `p002/body.rs`). Re-asserting an already-held `(actor, value)` is idempotent.
- `docs/architecture.md` records the rule and its non-claims.

## Validation

- `cargo fmt --all --check`: pass.
- `cargo test -p molten --lib dataspace`: 28 passed, 0 failed.
- `cargo test -p molten`: all suites passed (1,569 lib tests plus 74, 62, 12,
  6, 3, 5, 2, 9, 4 integration tests and doctests).
- `cargo check --workspace --all-targets`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo octet check -p molten --artifact-dir target/octet-coalesce-lib-3 -- --lib`:
  clean, 0 findings, 0 warnings, 0 errors.
- `cargo octet check --artifact-dir target/octet-coalesce-workspace`:
  clean, 0 findings, 0 warnings, 0 errors.

## Non-claims

The change alters visible notification count only. It does not change
ownership, authority, retention, delivery, snapshot shape, canonical value
identity, or reference-harness bag accounting. The reference harness comparison
is exact over the coalesced visible event sequence. No whole-system
correctness, live-profile, or release claim is made.
