## Why

World commits are useful only while their immutable closure remains available. Branch claims also need exchange between peers without turning content transfer into branch authority.

Molten already plans generic DAG synchronization and content replication extensions. Artifact Binding Core and existing retention rails provide root inventory and conservative deletion mechanics. The missing work is a world-commit domain adapter and complete retention-root policy.

## What Changes

- Add a world-commit DAG adapter for the planned DAG-sync system extension.
- Transfer immutable commits and typed root objects through the content-replication extension after exact closure validation.
- Exchange detached signed head claims separately from immutable content.
- Preserve competing claims and apply local current authority and branch policy before selecting any head.
- Deny activation until the required world closure is complete, typed, current, and admitted.
- Add retention roots for current and competing heads, active executions, replay pins, merge conflicts, promotion outboxes, reconciliation state, rollback holds, evidence holds, and remote lease observations.
- Feed complete reachability reports into existing retention planning without granting deletion authority.
- Treat missing, stale, contradictory, or unavailable remote observations conservatively.

## Dependencies

- `introduce-world-commit-core` and `add-world-branch-head-protocol`.
- `dag-sync-system-extension` and `content-replication-system-extension`.
- `adopt-artifact-binding-and-semantic-effects` root inventories.
- Existing Molten retention, remote-clearance, GC, Iroh, and content-store contracts.

## Non-Goals

- Global convergence, permanent durability, trust, merge authority, or automatic winning-head selection.
- Making replication grant activation, execution, promotion, or deletion authority.
- Deleting content from an incomplete inventory or an uncertain remote state.

## Impact

- **Core**: world DAG projection, missing-closure plans, claim-set comparison, retention-root projection, reachability, and diagnostics.
- **Shell**: DAG-sync and replication adapters, claim transport, closure materialization, lease observation, and retention integration.
- **Schemas**: sync request, closure report, claim exchange, retention-root, and reachability receipts.
- **Testing**: complete transfer plus negative partial, corrupt, wrong-domain, competing-claim, stale-lease, missing-root, interrupted-sync, and unsafe-GC cases.
