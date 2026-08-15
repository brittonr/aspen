## Why

Distributed extensions need to discover eligible nodes, observe membership views, detect suspected failures, express locality and fault-domain constraints, recruit service roles, place replicas or workers, fence stale assignments, drain nodes, and replace failed roles. Aspen has peer and coordination models, but not one generic membership and placement runtime that separates observation from authority and can run identically over live and simulated clusters.

## What Changes

- Add canonical node descriptors, membership views, view-source profiles, locality/fault-domain labels, and eligibility evidence.
- Add pluggable failure-detector observations with explicit suspicion, confidence, freshness, and non-authoritative semantics.
- Add extension-owned role requirements and a deterministic placement core with capacity, anti-affinity, locality, policy, and resource constraints.
- Add recruitment, assignment, acknowledgement, activation, drain, replacement, and release lifecycles.
- Bind assignments to epochs and fencing tokens supplied by an admitted authority profile; deny stale work.
- Provide live and deterministic-simulation providers with bounded evidence and operator readback.

## Impact

- **Files**: membership and node descriptor models, failure detector, placement core, assignment coordinator, system-extension lifecycle integration, transport/time/durability adapters, simulation provider, operator readback, fixtures, and a new `fabric-membership-placement` accepted spec.
- **Testing**: membership changes, stale views, suspicion, locality, capacity, deterministic placement, unsatisfiable constraints, split views, fencing, recruitment races, drain, replacement, and cleanup.
- **Safety**: connectivity and suspicion are not authority; a placement plan is not a committed assignment; fencing is only as strong as its authority and storage profile; membership does not imply consensus or service correctness.
