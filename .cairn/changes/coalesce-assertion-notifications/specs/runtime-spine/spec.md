# Runtime spine: assertion notification delta

## ADDED Requirements

### Requirement: Equal assertions coalesce for observers
r[molten.runtime_spine.assertion_notification_coalescing] Molten MUST deliver one observer notification per visible assertion value, regardless of how many owners hold an equal canonical value, and MUST notify a retraction only when the last owner withdraws.

#### Scenario: Duplicate assertions notify once
- GIVEN two actors assert an equal canonical value into the same dataspace
- WHEN an observer watches the matching pattern
- THEN the observer receives one notification for that value.

#### Scenario: Retraction of one owner keeps the value
- GIVEN two owners hold an equal canonical value
- WHEN one owner retracts or its scope is cleaned up
- THEN the value stays visible and no retraction notification is delivered.

#### Scenario: Last owner retracts
- GIVEN only one owner still holds a visible value
- WHEN that owner retracts
- THEN the observer receives one retraction notification.

#### Scenario: Distinct values stay separate
- GIVEN two owners assert different values
- WHEN an observer watches the matching pattern
- THEN each value produces its own notification.
