## Why

Molten borrows ideas from Synit/SAM, Goblins, BEAM/OTP, and Lunatic, but actor/service lifecycle and failure behavior need their own runtime contract. Without explicit lifecycle semantics, crashes, restarts, links, monitors, resource cleanup, assertion retraction, and supervision can diverge across adapters.

## What Changes

- Define actor, service, vat, session, handler, and job lifecycle states.
- Define links, monitors, supervisors, restart strategies, health assertions, and failure propagation.
- Require turn failure to roll back pending actions and trace the denial/crash.
- Retract assertions/subscriptions/live refs owned by failed or stopped scopes.
- Express service demand, readiness, failure, restart, and dependency status as dataspace assertions.
- Emit receipts and trace records for spawn, start, ready, fail, restart, stop, cleanup, and supervisor decisions.

## Impact

This makes failure behavior deterministic, inspectable, and policy-gated. The first milestone can add lifecycle states, local supervisors, assertion cleanup on actor stop, and restart receipts.
