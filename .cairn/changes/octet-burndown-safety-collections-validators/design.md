# Design: Octet burn-down, collection growth in validators

## Context

The pinned lint (`octet` `fc38f593`, `src/safety/unbounded_collection_growth.rs`) fires on a same-block local
collection that starts with `new()`/`default()` and grows with `push`/`insert`/`extend` inside a loop, unless growth is
guarded by `len()` or preceded by `with_capacity`/`reserve`. Every site in this slice is a validator or receipt helper
whose growth is already bounded by its inputs.

## Decisions

### Decision: Structural bounds, not new limits

**Choice:** A loop that only filters and maps becomes an iterator chain. A loop that adds at most `k` items per element
of an input reserves `input.len() * k` before the loop. `k` is a named constant when it is not 1.

**Rationale:** Growth is already bounded by the input, and those inputs are bounded upstream. For example, hardening
inputs are checked with `ensure_bound` against `MAX_ITEMS`, and parsed receipts are bounded by their record limits. A
second denial limit on the output could never trigger independently of the input limit. It would add dead denial paths
and no protection.

### Decision: Options for single-shot diagnostics

**Choice:** `evaluate_admission_chain` keeps an `Option` for the denial diagnostic, because the loop breaks at the first
denial. `dogfood::archive::read` keeps a flag, because `materialization::verify_archive` rejects duplicate normalized
members.

**Rationale:** Both produce the same zero-or-one diagnostic as before.

## No-spec classification

Accepted requirement text does not change. Semantic review inputs: the diff of the 24 files and the Octet, clippy, and
test evidence.

## Failure behavior

First-error behavior is preserved: `collect::<Result<_>>()` stops at the first failing element, in the same order the
loop visited them.

## Risks / Trade-offs

- A reservation is sized to the loop's own maximum pushes. Helper functions that push into the same vector still grow it
  as before, bounded by the same inputs.
