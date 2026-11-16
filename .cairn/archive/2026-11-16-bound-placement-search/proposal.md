## Why

Issue #53: `plan_placement` returned `Err(SearchLimitExceeded)` instead of
`PlacementOutcome::Unsatisfied` for unsatisfiable requests in modestly sized
views because the search explored every ordering of the same node set and had
no feasibility pre-check.

## What Changes

- Search combinations of candidate indices instead of permutations.
- Add a sound anti-affinity feasibility pre-check, including values already
  held by active current assignments for the same service/role.
- Keep satisfiable first-found plans byte-identical.
- Bound the search with the existing `MAX_PLACEMENT_SEARCH_STEPS` limit.

## Impact

- **Files**: fabric membership placement core/tests, placement documentation.
- **Testing**: modest unsatisfiable views, held-value pre-check, joint
  infeasibility limit, and combination/permutation replay property.
- **Owner / consumer**: Molten fabric membership placement core.
