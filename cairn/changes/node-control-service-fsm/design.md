## Context

The node daemon already emits startup, service-lock, heartbeat, supervisor, loop, service-run, live-listener, health, and shutdown receipts. Those receipts are canonical evidence, but the semantic state progression is implicit across shell functions. This change introduces a pure service lifecycle model and leaves IO in shells.

## Design

### Service state model

Define closed service states such as uninitialized, initialized, startup-locked, service-lock-held, serving, draining, stopped, duplicate-denied, stale-lock-recovery-pending, stale-lock-recovered, restart-denied, and failed. Final names can be adjusted during implementation, but the relation must be explicit and documented.

Events include init, startup, acquire-service-lock, heartbeat, tick-drain, live-listener-drain, supervisor-restart-request, stale-lock-detected, stale-lock-recover, duplicate-runner-observed, shutdown-requested, drain-complete, stop, and failure.

### Pure transition core

The pure core consumes:

- prior service state;
- event;
- startup receipt ref;
- service lock ref or absence;
- supervisor policy facts;
- heartbeat/tick counters supplied explicitly by the shell;
- inbox and drain facts;
- shutdown evidence;
- duplicate-runner and stale-lock observations;
- authority/policy/resource refs where relevant.

It returns decision, diagnostics, next state or preserved state, receipt input facts, and any shell intents such as acquire lock, release lock, scan ingress, drain inbox, send live listener receipt, or write shutdown receipt. The core does not read/write the filesystem, open sockets, spawn tasks, inspect clocks, publish Iroh messages, or persist receipts.

### Receipts

Existing receipts should either include or be accompanied by service-FSM evidence that binds prior state, event, next/preserved state, lock/startup/supervisor refs, counters, decision, diagnostics, and no-authority-by-lock caveats. Denials must bind the preserved state ref.

### Tests

Use pure tests for transition relation and shell-adapter tests for receipt wiring. Negative tests should include duplicate runner, stale lock without recovery policy, stale lock with wrong startup ref, heartbeat timeout beyond policy, restart beyond bounds, shutdown drain over limit, and live listener drain denial.