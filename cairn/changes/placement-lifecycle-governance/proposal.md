# Change: placement-lifecycle-governance

## Why

Kubernetes scheduling and lifecycle controls are useful because placement, capacity, readiness, restart, and cleanup decisions are first-class. Molten needs equivalent governance for actors, services, Wasm components, jobs, plugins, and node-control workflows, but expressed through resource budgets, capabilities, deterministic receipts, and replayable lifecycle state.

## What

- Add placement inputs for resource requests, limits, quotas, priorities, placement constraints, taints, tolerations, and capacity evidence.
- Add lifecycle health/readiness/startup probe records with restart/backoff decisions and status-condition updates.
- Add safe GC and cleanup gates that respect owner refs, finalizers, pins, retention policy, and authority evidence.
- Add negative coverage for over-quota placement, unsatisfied constraints, unauthorized node claims, flapping probes, restart loops, and GC without cleanup receipts.

## Impact

Molten can make operator-visible placement and lifecycle decisions without importing pod semantics. Runtime effects remain policy-gated and replayable, and capacity failures become structured denials instead of logs.
