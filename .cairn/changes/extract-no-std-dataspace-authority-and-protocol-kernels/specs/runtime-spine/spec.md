# Runtime spine deltas

## ADDED Requirements

### Requirement: Dataspace, vat, turn, predicate, envelope, and service-lifecycle FSMs live in no_std kernels
r[molten.runtime_spine.no_std_dataspace_and_service_fsm_kernels] Runtime dataspace/vat/turn/predicate/envelope transitions (`molten-dataspace-core`) and service-supervision/lifecycle/node-profile FSMs (`molten-service-fsm-core`) MUST compile and execute as `#![no_std]` + `alloc` crates atop `molten-preserves-core` without `std`, Redb, Tokio, or `parts/**/body.rs` `include!()` sharding.

#### Scenario: Dataspace turn commit and rollback execute under no_std
- GIVEN a dataspace snapshot, actor capability context, and pending turn actions
- WHEN `molten-dataspace-core` evaluates commit or rollback under `#![no_std]`
- THEN it MUST produce canonical packed Preserves events and state hashes identical to the `std` shell bridge without OS I/O.

#### Scenario: Service supervision FSM executes under no_std
- GIVEN lifecycle observations and a shutdown or restart event
- WHEN `molten-service-fsm-core` computes the transition under `#![no_std]`
- THEN it MUST emit deterministic state transitions and typed shell intents without filesystem or process access.
