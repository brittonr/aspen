# Fabric membership placement deltas

## ADDED Requirements

### Requirement: Pending intents and Quarantined role assignments have explicit recovery paths without bricking role slots
r[molten.fabric_membership.recoverable_pending_and_quarantined_slots] `AssignmentPersistence` MUST support inspecting, aborting definite pre-effect failures, and reconciling uncertain pending intents. Role assignment succession MUST permit a fenced, strictly higher-epoch successor (or explicit fenced release) after `AssignmentState::Quarantined` so role slots cannot be permanently bricked.

#### Scenario: Pre-effect failure rolls back pending intent cleanly
- GIVEN `record_intent` persisted a pending intent and `run_role_effect` failed with `effect_may_have_happened == false`
- WHEN pre-effect abort runs
- THEN the pending intent MUST clear and subsequent transitions on that role slot MUST proceed.

#### Scenario: Fenced successor replaces a Quarantined assignment
- GIVEN an assignment slot entered `AssignmentState::Quarantined`
- WHEN an authorized higher-epoch successor presents valid fencing/reconciliation evidence
- THEN initialization MUST admit the successor rather than failing permanently on `predecessor.state != Released`.

### Requirement: Placement ranks healthy nodes before degraded nodes and failure reduction is deterministic per time basis
r[molten.fabric_membership.healthy_first_placement_and_time_basis_reduction] Under `allow_degraded = true`, `PlacementSearch::select` MUST rank healthy nodes strictly ahead of `Suspected`/`Unavailable` nodes regardless of `preference_score`. Failure-observation reduction MUST be independent of input slice ordering and MUST NOT compare raw ticks across unrelated `time_basis_ref`s.

#### Scenario: Higher-preference Unavailable node does not displace healthy node
- GIVEN one `Unavailable` node with `preference_score = 100` and enough healthy nodes with `preference_score = 10`
- WHEN `PlacementSearch::select` runs with `allow_degraded = true`
- THEN it MUST select healthy nodes and produce a non-degraded plan.

#### Scenario: Reordered observations produce identical reduction and mismatched time bases do not mix
- GIVEN identical observations supplied in reversed order or observations with distinct `time_basis_ref`s
- WHEN failure reduction runs
- THEN reordered equal-basis observations MUST yield identical output and distinct time bases MUST NOT compare raw ticks.
