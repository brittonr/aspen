## ADDED Requirements

### Requirement: Effect log sequences are complete and monotonic
r[molten.determinism.effect_log.sequence] Replay effect logs MUST have deterministic sequence metadata with no gaps, duplicates, or reordering relative to the consumed replay effect boundaries.

#### Scenario: Ordered complete log passes
- GIVEN a replay report whose effect log entries are monotonic and match every consumed effect boundary
- WHEN effect-log validation runs
- THEN validation passes
- AND replay may continue to compare downstream trace and state refs.

#### Scenario: Missing sequence denies
- GIVEN a replay report whose consumed effect boundaries require a sequence missing from the effect log
- WHEN effect-log validation runs
- THEN validation denies before replay pass evidence
- AND diagnostics identify the missing sequence or boundary ref.

#### Scenario: Duplicate sequence denies
- GIVEN a replay effect log with two entries for the same sequence or request ref
- WHEN effect-log validation runs
- THEN validation denies before any duplicate response is consumed
- AND diagnostics identify the duplicate entry refs.

### Requirement: Effect requests and responses are directly bound
r[molten.determinism.effect_log.request_response_binding] Replay effect logs MUST bind each recorded response to the exact effect request ref, effect kind, turn or boundary ref, and consumed replay observation that used it.

#### Scenario: Response for another request denies
- GIVEN a replay effect log entry whose response ref was recorded for a different request ref
- WHEN replay consumes that entry
- THEN validation denies with a request/response binding diagnostic
- AND no live effect is issued to repair the mismatch.

#### Scenario: Extra unused response denies
- GIVEN a replay effect log with an entry that is not consumed by the replay trace
- WHEN effect-log validation completes
- THEN validation denies with an unused recorded response diagnostic
- AND the extra entry cannot satisfy pass evidence.

### Requirement: Effect logs are handler-profile and run-identity bound
r[molten.determinism.effect_log.handler_profile_binding] Replay effect logs MUST bind the run identity ref and handler profile ref used to record the effects, and replay MUST deny stale logs from different identities or profiles.

#### Scenario: Handler profile mismatch denies
- GIVEN a replay report whose effect log was recorded under a different handler profile ref
- WHEN replay validation evaluates the log
- THEN validation denies before effect consumption
- AND diagnostics include the expected and actual handler profile refs.

#### Scenario: Run identity mismatch denies
- GIVEN a replay report whose effect log belongs to a different artifact, dependency closure, initial state, policy, schema, capability, or seed identity
- WHEN replay validation evaluates the log
- THEN validation denies before treating the log as deterministic evidence
- AND diagnostics bind the stale run identity ref.

### Requirement: Replay denies live effect fallback
r[molten.determinism.effect_log.live_effect_denial] Replay MUST deny any attempt to satisfy a missing or invalid recorded effect by issuing a live external effect, and the denial MUST be represented as canonical failure or replay verification evidence.

#### Scenario: Missing response cannot call live adapter
- GIVEN a replay run reaches an effect boundary with no valid recorded response
- WHEN replay evaluates the boundary
- THEN replay denies as recorded-effects-only
- AND no external adapter request is issued.

### Requirement: Effect-log hardening is tested
r[molten.determinism.effect_log.tests] Molten SHOULD test valid logs plus missing, extra, duplicated, reordered, request/response-mismatched, profile-mismatched, run-identity-mismatched, wrong-effect-kind, and live-effect fallback denial cases.

#### Scenario: Negative matrix denies before final state drift
- GIVEN malformed effect-log fixtures covering every supported denial kind
- WHEN replay tests evaluate them
- THEN each case denies with the expected effect-log diagnostic
- AND final-state drift is not reported as the first divergence when the effect-log error is earlier.
