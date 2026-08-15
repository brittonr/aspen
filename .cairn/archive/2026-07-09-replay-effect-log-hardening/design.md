# Design: replay-effect-log-hardening

## Scope

This change covers validation of recorded deterministic effect logs used by replay. It does not introduce live effect execution during replay, new production effect capabilities, or authority/provenance/policy semantics.

## Proof checklist

- **Proof claim**: replay admits an effect log only when every recorded effect entry is ordered, unique, profile-bound, request-bound, response-bound, and consumed by the deterministic replay trace.
- **Out of scope**: external adapter correctness, live transport, current authority, production effect admission, and release promotion.
- **Trusted assumptions**: effect request/response values are canonical Preserves values and BLAKE3 refs identify them.
- **Positive evidence**: a well-formed multi-entry effect log validates and replays with every entry consumed exactly once.
- **Negative evidence**: missing, extra, duplicated, reordered, stale, wrong-profile, wrong-effect-kind, and wrong-request response entries deny before replay pass evidence.
- **Canonical refs**: effect log ref, handler profile ref, run identity ref, effect entry refs, request refs, response refs, turn refs, and validation receipt refs.

## Functional core

A pure validator accepts parsed effect-log entries and replay-consumed effect observations:

```text
EffectLogEntry {
  sequence
  effect_kind
  run_identity_ref
  handler_profile_ref
  turn_ref
  boundary_ref
  request_ref
  response_ref
}

ConsumedEffect {
  sequence
  effect_kind
  request_ref
  response_ref
  boundary_ref
}
```

It returns either a passing validation summary or the first denial reason. It does not read files, call adapters, emit logs, or mutate state.

## Imperative shell

The harness parser and CLI load Preserves reports, parse effect logs, call the pure validator, write canonical validation diagnostics or replay failure artifacts, and render summaries.

## Denial ordering

Denials should identify the earliest semantic reason in this order:

```text
schema/profile/run identity
sequence monotonicity and gaps
duplicate request or sequence
request/response binding mismatch
unconsumed extra effect
missing consumed effect
live effect fallback
```

This ordering keeps diagnostics stable and avoids downstream state drift masking the root cause.

## Non-goals

- No hidden fallback to live effects.
- No authority or policy admission based on old effect logs.
- No acceptance of rendered logs as primary evidence.
