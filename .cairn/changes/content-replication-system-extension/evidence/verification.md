# Content-replication verification

Recorded on 2026-08-27.

All declared fabric dependencies are archived. The same-core simulation archive is `.cairn/archive/2026-08-01-fabric-whole-system-simulation`.

## Pure core

The pure core defines the manifest, policies, epochs, inventory, operation history, actions, plan, status, issues, and non-claims.

The planner computes deterministic transfers, repairs, handoffs, reuse, deferrals, retention pins, and cleanup candidates.

Resume and operation identity bind the service generation, membership epoch, placement epoch, content, source, receiver, action, and attempt.

Eight positive and negative core tests pass. The tests cover stable ordering, placement domains, stale epochs, corruption, idempotency, conflicts, repair exhaustion, retention, cleanup, protected content, resources, and malformed manifests.

The focused core Clippy command passes for all targets and features with warnings denied.

## Current non-claims

The shell, lifecycle, live adapter, multiprocess fixture, and final repository gates are not complete at this stage.
