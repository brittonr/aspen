## Why

Aspen's current plugin host records installation, lifecycle, hostcall, and fixture receipts, but it does not execute an arbitrary long-running service implementation or expose request, message, stream, timer, checkpoint, recovery, drain, and shutdown callbacks. Databases, replicated logs, schedulers, object stores, and workflow engines cannot safely fit inside that receipt-first plugin surface.

A distinct system-extension runtime is needed so privileged distributed services can execute without modifying node core or gaining ambient authority.

## What Changes

- Add a canonical system-extension artifact and manifest distinct from ordinary plugins.
- Execute admitted extension code through explicit lifecycle, request, message, stream, timer, health, checkpoint, recovery, drain, and shutdown callbacks.
- Host long-lived, node-supervised service instances with bounded mailboxes, concurrency, cancellation, deadlines, resource accounting, and backpressure.
- Bind every callback to an admitted service instance, generation, capability set, and fabric-port set.
- Define activation, failure, restart, upgrade, rollback, drain, and cleanup behavior without granting extension code direct ambient filesystem, socket, clock, process, or environment access.
- Provide positive and negative conformance fixtures for executable callbacks and lifecycle failures.

## Impact

- **Files**: plugin/system-extension host modules, runtime supervisor integration, canonical models, CLI/operator readback, fixtures, and a new `system-extension-runtime` accepted spec.
- **Testing**: pure manifest and transition tests, executable callback fixtures, timeout/cancellation/backpressure tests, crash/restart/recovery tests, unauthorized-port denials, and lifecycle cleanup checks.
- **Safety**: system extensions are privileged but capability-scoped; installation is not activation, artifact identity is not authority, and callback success is not proof of distributed-service correctness.
