## Phase 1: Lifecycle model

- [ ] [serial] r[molten.lifecycle.state_model] Define lifecycle states and canonical transition records for actors, services, vats, sessions, handlers, and jobs.
- [ ] [serial] r[molten.lifecycle.transition_receipts] Emit receipts for spawn, start, ready, degraded, fail, restart, stop, cleanup, and supervisor decisions.
- [ ] [parallel] r[molten.lifecycle.no_otp_compat] Document BEAM/OTP and Lunatic as prior art only, not compatibility targets.
- [ ] [parallel] r[molten.lifecycle.trace_events] Emit tracing events for lifecycle transitions with cause and policy refs.

## Phase 2: Failure and cleanup

- [ ] [serial] r[molten.lifecycle.turn_failure] Roll back pending turn actions and vat deltas on panic, denial, or validation failure.
- [ ] [serial] r[molten.lifecycle.scope_cleanup] Retract assertions/subscriptions/live refs and release resources on stop, crash, authority loss, or disconnect.
- [ ] [parallel] r[molten.lifecycle.cleanup_idempotent] Make cleanup idempotent and receipt-backed.
- [ ] [parallel] r[molten.lifecycle.one_shot_effects] Report irreversible one-shot effects explicitly in failure traces.

## Phase 3: Supervision

- [ ] [serial] r[molten.lifecycle.links_monitors] Add policy-controlled links and monitors for failure propagation and observation.
- [ ] [serial] r[molten.lifecycle.supervisors] Add local supervisors with never, one-for-one, and bounded restart strategies.
- [ ] [parallel] r[molten.lifecycle.restart_windows] Use logical-time restart windows and resource budgets for restart throttling.
- [ ] [parallel] r[molten.lifecycle.service_assertions] Represent service demand, readiness, failure, dependency, exposed refs, restart, and stop as dataspace assertions.

## Phase 4: Tests

- [ ] [serial] r[molten.lifecycle.failure_tests] Add tests that failed turns discard pending actions and emit failure receipts.
- [ ] [serial] r[molten.lifecycle.cleanup_tests] Add tests that actor stop/crash retracts owned assertions and subscriptions.
- [ ] [parallel] r[molten.lifecycle.restart_tests] Add deterministic supervisor restart tests with bounded restart windows.
- [ ] [parallel] r[molten.lifecycle.property_tests] Add Hegel property tests for cleanup idempotence, no leaked assertions, and restart bounds.
