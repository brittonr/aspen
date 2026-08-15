# Design: replay-identity-freshness

## Scope

This change covers replay identity and freshness binding for replay receipts, rollups, indexes, and release/dogfood readback. It does not change how source gates, provenance, authority, policy, or release promotion are decided.

## Proof checklist

- **Proof claim**: replay evidence is accepted for a release or subsystem readiness claim only when its deterministic run identity matches the subject identity it claims to cover.
- **Out of scope**: proving source correctness, build reproducibility, current authority, policy admission, release eligibility, or transport correctness.
- **Trusted assumptions**: run identity fields are canonical content refs and deterministic identity construction is stable.
- **Positive evidence**: replay receipts and indexes with matching identity refs pass freshness validation.
- **Negative evidence**: changed artifact, dependency closure, initial state, schema, policy, capability, handler profile, seed/effect-log, runtime, tool, or replay profile refs deny freshness validation.
- **Canonical refs**: run identity ref, subject identity ref, replay receipt refs, replay index refs, release gate refs, freshness validation receipt refs, and stale-component diagnostics.

## Identity model

The replay identity should be explicit enough to prevent stale evidence from drifting across release boundaries:

```text
ReplayRunIdentity {
  artifact_ref
  dependency_closure_ref
  initial_state_ref
  schema_refs
  policy_refs
  capability_refs
  revocation_refs
  handler_profile_ref
  seed_ref or effect_log_ref
  runtime_refs
  tool_refs
  replay_profile
}
```

A pure freshness validator compares the replay identity embedded in supplied evidence against the expected subject identity for a release, subsystem, or coverage row. It returns a pass summary or the first stale component diagnostic.

## Release readback

Release/dogfood readback should validate that replay indexes and raw replay verify receipts bind the same identity as the dogfood report or release gate member they support. Missing identity fields, malformed refs, or mismatched components deny readback with canonical diagnostics.

## Catalog readback

Catalog classifications should make replay identity discoverable without leaking sensitive payloads. Search can include identity ref, replay profile, artifact ref, handler profile ref, policy ref, and stale-component diagnostics.

## Non-goals

- No promotion of replay freshness into source-gate, policy, provenance, or release authority.
- No live replay execution.
- No acceptance of identity text rendering instead of canonical refs.
