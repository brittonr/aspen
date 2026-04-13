# Reduce Aspen Tigerstyle Safety-Debt Noise

## Why

Aspen's workspace-wide tigerstyle triage still has several high-noise lint
families that block later rollout phases:

- `ignored_result`: 299 matches
- `no_unwrap` / `expect`: 421 matches
- `no_panic`: 323 matches
- `unchecked_narrowing`: 510 matches
- `unbounded_loop`: 173 matches

Phase 1 deliberately avoids these families. Without a separate cleanup plan,
phase 2 and phase 3 would either drown maintainers in warnings or encourage
blanket suppression.

## What Changes

- pick an adoption order for the high-noise safety families
- build per-crate inventories for the first family in scope
- fix or explicitly justify the highest-risk findings instead of bulk-allowing
  them
- define the evidence required before each family graduates into Aspen's
  default tigerstyle path

## Initial Focus

Start with `ignored_result` and `no_unwrap`, because they hide failures or
panic directly in production paths. Hold `unchecked_narrowing` and
`unbounded_loop` until their remaining false-positive patterns are understood.
