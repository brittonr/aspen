## Why

Individual world changes plan fail-closed local mutations and uncertain outcomes. The roadmap lacks one conformance rail that injects interruption around every world mutation boundary and exercises competing operations across subsystems.

A happy-path workflow can pass while restart exposes an unpublished commit, lost outbox reservation, stale head, unsafe deletion plan, or duplicated external request.

## What Changes

- Add a versioned inventory of every world capture, head, promotion, outbox, replication, import, retention, and garbage-collection mutation boundary.
- Bind each operation to one stable identity, expected pre-state, linearization point, durable records, and reconciliation contract.
- Add deterministic fault profiles for before-submit, after-submit, after-durable-write, before-response, lost-response, restart, and recovery observations.
- Exercise concurrent head claims, promotions, imports, replication updates, retention changes, and garbage-collection plans.
- Require conservative recovery from partial, uncertain, stale, conflicting, missing, or corrupt durable state.
- Reuse Transactional Reconciliation Core, deterministic simulation, and existing subsystem receipts.
- Emit one bounded conformance receipt over the exact fault matrix, revision, profile, and results.

## Dependencies

- World commit core, branch heads, effect release, replication and retention, and replay capsules.
- Transactional Reconciliation Core and existing Molten uncertain-outcome mechanisms.
- Molten deterministic whole-system simulation and reviewed fault-injection adapters.
- Independent witness observations for strong whole-store rollback cases.

## Non-Goals

- Proof against every filesystem, device, kernel, power controller, or Byzantine failure.
- A new transaction coordinator, consensus system, effect broker, or storage engine.
- Automatic recovery when authority, evidence, or exact durable observations are unavailable.
- Converting fault-injection coverage into correctness or production-readiness claims.

## Impact

- **Core**: mutation inventory, operation contracts, fault scenarios, expected recovery classes, and conformance comparison.
- **Shell**: deterministic fault hooks, restart harnesses, concurrent drivers, observation collection, and receipt persistence.
- **Schemas**: typed Nickel fault profiles and Rust-owned conformance receipts.
- **Testing**: positive recovery plus negative torn state, lost response, double submission, stale plan, conflict, corrupt record, missing object, rollback, unsafe cleanup, and overclaim cases.
