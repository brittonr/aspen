## Why

Molten already contains actors, dataspaces, transport adapters, durable local stores, coordination models, policy gates, and deterministic evidence, but it does not yet state one reviewed product boundary for those pieces as a general distributed-systems fabric. Without that boundary, database-specific, actor-specific, or control-plane-specific semantics can leak into the node core, and the current sandboxed plugin model can be mistaken for the stronger system-extension tier needed by distributed services.

Molten needs an explicit fabric contract that keeps universal mechanisms in the core while allowing databases, replicated logs, schedulers, object stores, queues, workflow engines, and other systems to own their semantics as extensions.

## What Changes

- Define Molten as a workload-neutral distributed-systems fabric rather than a database framework or one mandatory global runtime model.
- Define sandboxed plugins, system extensions, and applications/workloads as separate execution and authority tiers.
- Add a canonical registry model for capability ports such as transport, durable state, time, scheduling, membership, placement, consistency, supervision, policy, resources, and simulation.
- Require extension-owned semantics to remain outside the node core unless promoted through a separate reviewed fabric change.
- Define reference-system exit criteria using a transactional key-value service, replicated log, and distributed scheduler without making any one reference service normative.
- Reject OpenRaft as a dependency or adaptation target and keep consistency engines behind explicit extension ports.
- Preserve a clean-room boundary around AGPL-licensed Aspen `main` implementation code, comments, and fixtures.
- Record explicit non-claims for compatibility, production readiness, global ordering, global consensus, and database correctness.

## Impact

- **Files**: `docs/architecture.md`, a new distributed-system fabric document, core capability/port descriptors, extension manifests, operator readback, `README.md`, and `cairn/specs/project/spec.md`.
- **Testing**: canonical descriptor tests, duplicate/unknown port denial tests, extension-tier validation, mechanism/semantics boundary tests, and reference-system architecture fixtures.
- **Safety**: fabric descriptors and receipts identify mechanisms and admitted bindings only; they do not prove extension correctness, production readiness, transport delivery, durability, consensus, or application semantics.
