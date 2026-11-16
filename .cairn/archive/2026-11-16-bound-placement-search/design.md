## Context

`PlacementSearch::select` iterated every candidate at every replica ordinal, so it visited each set of nodes once per ordering. `eligible_candidates` does no feasibility pruning, and the search stops at `MAX_PLACEMENT_SEARCH_STEPS` (100,000). Issue #53's model shows modest unsatisfiable views (30 nodes in 3 zones for 4 replicas; 16 nodes in 4 zones for 5 replicas) exceed the bound and return `SearchLimitExceeded` instead of `Unsatisfied`.

## Decisions

### Decision: Search candidate sets, not orderings

**Choice:** `select` recurses only over candidates at or after the last selected index. Repeating the same index remains possible when `distinct_nodes` is false; `can_select` still enforces distinctness when it is true.

**Rationale:** Every selection rule is order-independent (residual capacity, anti-affinity values, degradation) and hereditary, so the lexicographically first valid ordering is sorted. The previous depth-first search therefore returned the same plan the set search returns. A property test compares both searches on small random views to keep replay stable.

### Decision: Pre-check distinct anti-affinity values

**Choice:** Before searching, count each anti-affinity key's distinct values among eligible candidates, excluding values held by active assignments of the same service and role. Fewer values than requested replicas returns `Unsatisfied` with the existing constraint explanation.

**Rationale:** `can_select` requires every anti-affinity label and rejects held values, so the count is a necessary condition; the check never rejects a satisfiable request.

## Risks / Trade-offs

- Jointly infeasible multi-key constraints are not decided by the pre-check; they still end at the unchanged search bound, which a test exercises.
- The ordering search remains only as a test reference.
