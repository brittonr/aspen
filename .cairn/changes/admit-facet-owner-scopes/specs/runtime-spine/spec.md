# Runtime spine: facet owner scope delta

## ADDED Requirements

### Requirement: Facets own nested assertion lifetimes
r[molten.runtime_spine.facet_owner_scopes] Molten MUST represent facet owner scopes as a nested lifetime owner over assertions, observers, and child facets, MUST stop children before their parent and retract the stopping facet's owned state after its children stop, MUST make a stopped facet permanent, and MUST skip facet stop handlers on the crash path.

#### Scenario: Child stops before parent
- GIVEN a facet tree with a parent and a child, and both own assertions
- WHEN the parent stops
- THEN the child stops first, the child's assertions retract, then the parent's assertions retract, then the parent's stop handler runs.

#### Scenario: Stopped facet never restarts
- GIVEN a facet that stopped in an earlier committed turn
- WHEN a later turn asserts an observer or a child into that facet
- THEN the action denies before commit and the facet remains stopped.

#### Scenario: Crash retracts without running handlers
- GIVEN a running facet with owned assertions and a registered stop handler
- WHEN the owning actor crashes and owner-scope cleanup runs
- THEN facet-owned assertions and observers retract and the stop handler does not run.

#### Scenario: Flat actor scopes are unchanged
- GIVEN an actor that never creates a facet
- WHEN its owner scope is cleaned up
- THEN cleanup removes its assertions, observers, and messages exactly as before.
