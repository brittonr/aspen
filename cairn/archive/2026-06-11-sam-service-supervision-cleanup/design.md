## Context

The service demand runtime can start services and publish owned readiness/status assertions. The next local runtime gap is supervision and cleanup: service failures and revocations must not leave stale assertions, observers, live refs, pending effects, or restart loops behind. Supervision is a logical dataspace/evidence model, not an OS process tree.

This change depends on canonical service records and demand runtime lifecycle receipts.

## Goals

- Define logical service link and monitor records independent from OS parent/child process relationships.
- Emit deterministic failure, monitor notification, restart, backoff, stop, and cleanup lifecycle receipts.
- Enforce bounded restart policies with maximum attempts, logical backoff slots, restart windows, and resource refs.
- Retract service-owned assertions, observers, live refs, exposed refs, and pending effect intents on stop, failure, revocation, or cleanup.
- Bind cleanup receipts into authority revocation and retention/GC evidence.
- Add tests and Hegel properties for bounded restart, monitor ordering, no stale owned state, and no-authority-after-revocation.

## Non-Goals

- No OS process-tree semantics or signal handling contract.
- No unbounded restart loops or wall-clock backoff.
- No hidden cleanup of refs that are not service-owned or explicitly authority-bound.
- No remote service discovery protocol; remote surfaces continue to use canonical envelopes.
- No global service supervisor singleton.

## Logical Supervision Model

`service-link-v1` records represent owner/child relationships and failure propagation preferences. `service-monitor-v1` records represent observers that receive canonical failure/status facts. Both records are dataspace/evidence values with refs included in lifecycle receipts.

When a service fails:

1. Commit a failure status assertion owned by the service or supervisor.
2. Emit monitor notification refs in deterministic service-id/ref order.
3. Evaluate restart policy against recorded attempt count, logical window, resource budget, and authority state.
4. Emit a restart decision receipt with `pass`, `deny`, or `backoff`.
5. If restart passes, schedule a new demand/startup transition using the demand runtime path.
6. If restart denies or service stops, run cleanup.

## Cleanup Model

Cleanup takes service id, manifest ref, owner authority refs, prior status refs, and ownership indexes. It must produce a `service-cleanup-receipt-v1` binding:

- owned readiness/degraded/failure/stopped assertion refs;
- observer and monitor refs;
- live/exposed reference refs;
- pending effect intent refs;
- retraction/tombstone refs;
- authority/revocation refs;
- diagnostics and checks.

Cleanup is fail-closed: if ownership cannot be proven, cleanup emits diagnostics and does not silently delete unrelated state. Retention/GC may use cleanup receipts as eligibility evidence but still requires its own retention policy checks.

## Resource Bounds

Restart and cleanup evaluation must use logical resource receipts, not wall-clock or ambient process state. Supported bounds include restart attempts per window, mailbox entries inspected, assertions retracted, monitors notified, cleanup items processed, and trace bytes emitted.

## Replay

Replay identity includes prior lifecycle receipt refs, link/monitor refs, restart policy ref, failure ref, authority/revocation refs, resource receipt refs, cleanup input refs, and effect-log refs. Replay fails on changed monitor order, restart decision, cleanup retraction set, or resource denial.
