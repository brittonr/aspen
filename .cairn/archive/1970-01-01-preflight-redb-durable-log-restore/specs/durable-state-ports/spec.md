# Existing current-profile bounds on persisted Redb log restore

## ADDED Requirements

### Requirement: Persisted durable log is admitted before value ownership

r[molten.fabric_durability.redb_restore_profile_admission] `RedbDurableStateAdapter::open` MUST apply its supplied, validated current canonical profile's `max_log_records` before iterating or cloning Redb durable-log record values. It MUST apply that profile's existing `max_operation_bytes` and `max_namespace_bytes` together with the namespace descriptor's existing `quota_bytes` before owning each decoded durable-log value. Denial MUST use existing durability count/byte diagnostic authority, MUST NOT modify persisted legacy records, and MUST NOT substitute a default budget or affect admitted logs or unrelated ordered/snapshot/effect semantics. The owner selects current profile budgets; no universal new numeric cap is implied.

#### Scenario: Exactly the current persisted log count restores

- GIVEN two real Redb durable-log records persisted under an earlier broader profile, and a validated current profile that admits exactly two
- WHEN `RedbDurableStateAdapter::open` restores the log
- THEN both records and their values are available at their existing sequence positions.

#### Scenario: One extra persisted log record denies before loading values

- GIVEN three real Redb durable-log records and a validated current profile that admits exactly two
- WHEN `RedbDurableStateAdapter::open` attempts restore
- THEN the existing count admission denies before decoding or cloning any log record value, and the persistent store remains unchanged for another authorized profile.

#### Scenario: Current profile bytes deny oversized legacy payloads before cloning

- GIVEN legacy persisted values at and one byte above the supplied profile's per-operation limit or current namespace quota
- WHEN `RedbDurableStateAdapter::open` attempts restore
- THEN values exactly within both selected budgets remain eligible, while excess values deny before owning the overbound payload.

#### Scenario: Existing native host journal still survives restart

- GIVEN a valid native host instance appended durably under the current profile
- WHEN a new Redb adapter and durable native host journal reopen it
- THEN the journal retrieves the unchanged canonical instance record and its history remains bounded by admitted persisted log entries.
