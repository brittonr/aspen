## Why

Molten now has a fail-closed policy/admission evidence rail: policy preflight is recorded before side effects, every step records an admission decision, and report validation recomputes decisions from the embedded static policy fixture. The next missing boundary is authority. Admission requests currently identify actor, action, target, value, and effect metadata, but they do not carry or bind a capability/authority context.

Without capability context, a policy gate can deny specific deny-list cases, but cannot prove that an allowed actor actually held authority to send, assert, observe, retract, or request an effect. This leaves the Basalt/UCAN boundary as a marker rather than an evidence-bearing input to admission. Molten needs a deterministic local capability fixture now so later Basalt/UCAN integration can replace the fixture without weakening replay or report validation.

## What Changes

- Add a canonical static capability fixture for deterministic harness suites.
- Track omitted capability fixtures explicitly so later mandatory-fixture hardening can reject implicit authority rather than silently granting or inferring it.
- Bind every admission request to a capability context ref and the grant evidence used to authorize or deny the request.
- Make authorization deny by default when no matching grant exists.
- Recompute capability authorization during report validation instead of trusting recorded decisions.
- Extend policy-gate/admission/gate receipt evidence with capability context checks and refs.
- Add negative scenarios for missing grants, tampered grants, stale capability refs, and denied effects without authority.
- Keep the first capability fixture Preserves-shaped and deterministic while preserving a migration path to Basalt/UCAN proofs and attenuation chains.

## Impact

This change turns Basalt/UCAN from a future marker into a concrete admission input: even before full UCAN is implemented, reports and gates can prove that allowed actions were backed by explicit authority and denied actions failed because authority was absent or revoked. It also provides the next security boundary needed before adding richer actor kinds, adapters, Wasm hostcalls, or remote proxies.
