# Tasks: replay-effect-log-hardening

## Phase A: Effect-log model and pure validator

- [ ] [serial] r[molten.determinism.effect_log.sequence] Define parsed effect-log entry DTOs with sequence, request, response, effect kind, run identity, handler profile, and boundary refs.
- [ ] [serial] r[molten.determinism.effect_log.sequence] Implement pure monotonicity, gap, duplicate, and consumed-entry validation.
- [ ] [serial] r[molten.determinism.effect_log.request_response_binding] Validate request/response binding and unused-entry denial.
- [ ] [parallel] r[molten.determinism.effect_log.handler_profile_binding] Validate run identity and handler-profile bindings before effect consumption.

## Phase B: Replay integration

- [ ] [serial] r[molten.determinism.effect_log.live_effect_denial] Integrate effect-log validation before deterministic replay admits recorded responses.
- [ ] [parallel] r[molten.determinism.effect_log.request_response_binding] Add canonical diagnostics for malformed effect-log entries and stale response refs.
- [ ] [parallel] r[molten.determinism.effect_log.live_effect_denial] Ensure replay shells cannot issue external adapter calls when an effect entry is missing or invalid.

## Phase C: Tests and docs

- [ ] [serial] r[molten.determinism.effect_log.tests] Add positive multi-entry effect-log replay tests.
- [ ] [serial] r[molten.determinism.effect_log.tests] Add negative tests for missing, extra, duplicated, reordered, stale, wrong-profile, wrong-run-identity, wrong-kind, and wrong-request entries.
- [ ] [serial] r[molten.determinism.effect_log.tests] Add CLI malformed-effect-log tests and update replay documentation.
