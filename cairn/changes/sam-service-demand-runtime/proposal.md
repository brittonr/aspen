## Why

After service records exist, Molten needs the first deterministic SAM behavior: demand assertions should start services only when dependencies and policy evidence admit them. This must reuse the existing runtime kernel, authority/resource/effect gates, and replay/evidence rules rather than introducing an ambient process supervisor.

## What Changes

- Observe canonical service demand assertions and dependency readiness facts through the local dataspace kernel.
- Resolve service manifests and compute startup admission from explicit authority, policy, resource, effect-handle, and source-gate evidence.
- Commit readiness, degraded, failure, and stopped assertions as service-owned dataspace facts in deterministic turns.
- Emit lifecycle receipts and replay identity refs for every start, denial, readiness, and stop transition.
- Add a deterministic two-service fixture and CLI/test path that proves demand-driven startup and dependency readiness.

## Impact

This turns service records into executable local semantics while remaining deterministic and evidence-gated. It unlocks local-node dogfood steps that need a service to start because another actor demands it, but it does not yet implement full restart trees or long-lived plugin lifecycle integration.
