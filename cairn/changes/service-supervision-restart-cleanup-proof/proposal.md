## Why

Service supervision decides when a service starts, waits, fails, restarts, or cleans up. The proof needs to show that dependency waits do not start actors, restart budgets cannot loop forever, monitor notifications are deterministic, and cleanup is idempotent and ownership-bound.

## What Changes

- Add proof requirements for service demand/start/wait/failure/restart/cleanup transitions.
- Require bounded restart policy evidence and denial on exhausted budgets.
- Require cleanup idempotence and ownership-bound assertion/resource retraction evidence.

## Impact

- **Files**: service runtime, service supervision, lifecycle/service records tests.
- **Testing**: dependency-wait negative tests, bounded restart traces, cleanup idempotence, monitor ordering, and replay divergence checks.
