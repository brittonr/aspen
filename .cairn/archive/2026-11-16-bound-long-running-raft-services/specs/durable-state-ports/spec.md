# Durable-State Ports Delta

## ADDED Requirements

### Requirement: Snapshot retirement and covered truncation require retention authority
r[molten.fabric_durability.snapshot_compaction] Aspen MUST provide snapshot retirement that removes only snapshots whose log coverage does not exceed a kept, valid, durable, uncorrupted snapshot, and covered truncation that removes only records before a retained record no later than one past that snapshot's coverage. Both MUST require the namespace's declared retention authority; a namespace without one MUST deny them. Retirement MUST commit metadata before removing snapshot payloads.

#### Scenario: Superseded snapshots retire
- GIVEN two valid snapshots and the namespace's retention authority
- WHEN retirement keeps the newer one
- THEN only the newer snapshot remains.

#### Scenario: Truncation without authority is denied
- GIVEN a valid covering snapshot
- WHEN covered truncation is requested without the retention authority
- THEN it is denied before mutation.

#### Scenario: Uncovered records stay
- GIVEN a snapshot covering records through sequence c
- WHEN truncation asks to retain from a sequence after c + 1
- THEN it is denied.
