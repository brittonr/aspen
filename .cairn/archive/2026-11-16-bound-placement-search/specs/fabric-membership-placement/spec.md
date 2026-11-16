# Fabric Membership Placement Delta

## MODIFIED Requirements

### Requirement: Placement planning is deterministic and explainable
r[molten.fabric_membership.placement] Aspen MUST implement placement as a pure deterministic function over a declared membership view, role requirements, current assignments, resource inventory, typed locality and anti-affinity constraints, policy decisions, failure observations, and deterministic tie-break input. It MUST return either a canonical plan with reasons and residual capacity or structured unsatisfied constraints without side effects. Because replicas share one requirement, the search MUST consider each candidate set once rather than every ordering of it, and a request that cannot have enough distinct anti-affinity values MUST be reported unsatisfied before searching.

#### Scenario: Identical inputs produce identical plan
- GIVEN identical canonical planner inputs and tie-break stream
- WHEN placement runs repeatedly
- THEN it returns the same assignments, scores, reasons, and residual-capacity model.

#### Scenario: Impossible placement is explicit
- GIVEN a role requires more distinct eligible fault domains than the view provides
- WHEN placement runs
- THEN it returns an unsatisfied hard constraint
- AND does not silently colocate roles or weaken anti-affinity.

#### Scenario: Too few anti-affinity values are reported before searching
- GIVEN a modest view whose candidates, after excluding values held by active assignments of the same service and role, have fewer distinct values for an anti-affinity key than requested replicas
- WHEN placement runs
- THEN it returns unsatisfied anti-affinity constraints rather than exhausting the search bound.

#### Scenario: Search over candidate sets preserves plans
- GIVEN any request that the previous ordering search answered within its step bound
- WHEN placement runs
- THEN it returns the identical plan or unsatisfied outcome.

#### Scenario: Jointly infeasible constraints still respect the search bound
- GIVEN anti-affinity keys that each have enough distinct values but cannot all be satisfied together across a large view
- WHEN placement runs
- THEN it reports the search limit rather than searching without bound.
