# Coordination deltas

## ADDED Requirements

### Requirement: Coordination primitives and control-registry state transitions live in a no_std kernel separated from Redb
r[molten.coordination.no_std_coordination_and_registry_kernel] Coordination locks, leases, barriers, queues, elections, fencing tokens, and control-registry transitions MUST live in a `#![no_std]` + `alloc` crate (`molten-coordination-core`), leaving Redb persistence and CLI orchestration strictly inside thin `std` adapters in `src/`.

#### Scenario: Coordination lock and election transitions run without Redb or std
- GIVEN an in-memory coordination registry state and lock/election requests
- WHEN `molten-coordination-core` applies the transitions under `#![no_std]`
- THEN it MUST produce deterministic state updates and fencing-token receipts without filesystem or Redb access.
