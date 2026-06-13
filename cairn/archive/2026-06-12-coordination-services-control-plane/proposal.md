## Why

Aspen-style coordination primitives are one of the practical reasons for a strongly consistent Molten control plane. Once Raft-backed registry state exists, Molten needs receipt-backed locks, leases/fencing tokens, queues, semaphores, rate limits, elections, barriers, and service registry entries exposed as dataspace facts without using Raft for ordinary actor traffic.

## What Changes

- Add canonical coordination service manifests, requests, receipts, state snapshots, and dataspace status assertions.
- Implement first Raft/control-registry-backed primitives: fencing lock, FIFO queue, semaphore, rate limit, election, barrier, and service registry pointer.
- Gate operations through authority, resource, policy, idempotency, and read-index/commit evidence.
- Publish coordination outcomes as local dataspace assertions after committed control-plane apply.

## Impact

This gives Molten a practical replicated control-plane API and replaces ad hoc coordination with explicit evidence and fencing semantics.
