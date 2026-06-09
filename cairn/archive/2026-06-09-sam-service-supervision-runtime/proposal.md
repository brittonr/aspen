## Why

Molten's dataspace slice can route local and remote assertions/messages, but it does not yet provide the Synit/SAM service layer needed for Aspen 2.0: demand-driven startup, readiness/failure assertions, lifecycle state, supervision, restart policy, authority cleanup, and deterministic replay of service transitions.

## What Changes

- Add canonical service, demand, readiness, failure, lifecycle, monitor, link, supervisor, restart, and cleanup records.
- Implement a local service runtime over the existing dataspace/turn kernel.
- Start services only when demand assertions and authority/resource gates admit them.
- Emit service lifecycle receipts and actor-scoped turn-journal context refs.
- Auto-retract service-owned assertions and references on stop, failure, revocation, or supervisor cleanup.
- Emit replay-bound service supervision gate receipts for operator review without treating those receipts as authority.

## Impact

This converts basic dataspace facts into a usable local service runtime. It remains local/deterministic first; remote service exposure uses the existing remote dataspace envelope and later choreography/control-plane slices.
