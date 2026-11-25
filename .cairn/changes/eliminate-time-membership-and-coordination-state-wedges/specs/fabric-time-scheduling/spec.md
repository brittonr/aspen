# Fabric time scheduling deltas

## ADDED Requirements

### Requirement: Expired reservations support delayed Stale-to-Released transitions and restore worker capacity
r[molten.fabric_time.reservation_stale_release_and_worker_conservation] `expire_reservation` MUST allow `Stale` reservations to transition to `Released` upon later admitted enforcement (`ObservedStop` or `FencingTakeover`) and MUST restore both pool capacity and the assigned worker's available units and bytes on release.

#### Scenario: Delayed ObservedStop releases a Stale reservation and restores worker capacity
- GIVEN a reservation expired without enforcement (entering `Stale`) and the assigned worker has zero remaining units
- WHEN `expire_reservation` is called later with an admitted `ObservedStop` fact
- THEN the reservation MUST transition to `Released` and both pool and worker capacity MUST return to pre-reservation levels.

#### Scenario: Already-released reservation denies duplicate release
- GIVEN a reservation already in `Released`
- WHEN `expire_reservation` is invoked again
- THEN it MUST fail closed without double-crediting pool or worker capacity.

### Requirement: Batch reservations preserve per-item atomicity and exclusive lease renewals enforce fencing tokens
r[molten.fabric_time.batch_item_atomicity_and_fenced_renew] `Owner::reserve_batch` MUST retain committed state for every item already externally allocated when a later item fails and MUST preserve `FabricPortError::Uncertain` class. `evaluate_lease` MUST enforce fencing-token and consistency checks on `LeaseAction::Renew` for `FencedExclusive` leases.

#### Scenario: Second item failure does not revert first allocated item
- GIVEN item 1 allocates externally and item 2 returns an adapter error in `Owner::reserve_batch`
- WHEN `reserve_batch` returns
- THEN item 1 MUST remain committed in `self.state` so its capacity cannot be double-allocated.

#### Scenario: Stale fencing token denies exclusive lease renewal
- GIVEN a `FencedExclusive` `LeaseAction::Renew` request with `fencing_token: None` or a token below `previous_fencing_token`
- WHEN `evaluate_lease` runs
- THEN it MUST deny renewal rather than returning `RenewalAllowed`.
