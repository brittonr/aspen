# Policy, capability, and resource input reference

Mode: Reference

Use this reference when assembling an evidence inventory for admission review. It maps actual Rust input fields and canonical artifacts to their owning decision boundaries. It is not a new wire schema or a claim that all inputs are composed by one live service. Definitions are source-checked, not executed; no command invocation or output is implied.

Canonical Preserves values and BLAKE3 references define identity. Rust structure layout does not. The [nominal reference contract](../../nominal-authority-references.md) supplies category-separation rules, while the [preflight companion](../../technical/capabilities/policy-preflight-composition.md) explains why preflight and request permission remain different questions.

## Authority currentness inputs

Owner: `AuthorityGrantCurrentnessInput` in [authority declarations](../../../src/authority/parts/mod/p000/body.rs), evaluated by [currentness](../../../src/authority/parts/mod/p001/body.rs).

| Field or group | Meaning at this boundary | Evidence the caller must retain |
|---|---|---|
| `context` | Parsed authority context with subject, capabilities, validity, keys, and reference lists | Exact context value and its canonical reference |
| `requested_principal_ref` | Principal compared with context subject | Caller identity binding, not merely a copied subject |
| `requested_capability`, `requested_operation`, `requested_scope` | Requested action dimensions | The action actually dispatched and its target scope |
| `logical_time` | Explicit validity and revocation comparison input | Time domain and source |
| `grant_epoch`, `minimum_epoch`, `current_epoch` | Inclusive accepted grant-epoch interval | Integration's epoch facts |
| `current_key_refs` | Current key set intersected with nonempty context keys | Source of the current key view |
| `revocations` | Supplied records considered for effective target matches | Complete applicable facts for the reviewed boundary |

The result has `decision` and `diagnostics`; currentness uses `pass` or `fail`. A reference parsing error is a `Result` error rather than this ordinary diagnostic result. The wrapper `admit_authority` does not accept all these facts independently, so its signature must not be described as equivalent evidence of caller identity or key rotation.

## Token and proofset inputs

Owner: [capability token declarations](../../../src/capability/parts/tokens/p000/body.rs), with [matching rules](../../../src/capability/parts/tokens/p001/body.rs).

| Input | Actual fields useful during review | Checks and limits |
|---|---|---|
| `CapabilityRequest` identity | `holder_ref`, `session_ref`, `context_ref` | Compared with proofset and token bindings |
| `CapabilityRequest` action | `resource_ref`, `ability`, `scope`, `required_token_kind` | Resource/ability exact; scope exact or permitted token wildcard |
| `CapabilityRequest` context | `at_tick`, `required_policy_refs`, `required_resource_refs`, `caveat_context` | Explicit time, proofset membership requirements, supplied caveat strings |
| `CapabilityProofset` | Identity triple, `tokens`, `policy_refs`, `resource_refs`, `revocation_refs`, `evidence_refs` | Reference lists are inputs, not fetched facts |
| `CapabilityToken` | Issuer/holder/session/context/resource, ability/scope/attenuation, caveats, expiry, revocation/policy/resource/delegation/evidence lists | Presence of a field does not imply every helper evaluates it |
| `UcanVerificationInput` | Token/proofset/request refs, proof/key/fact/derived-grant refs, identity/action bindings, `checks` | Check booleans are supplied to receipt construction; this type alone is not a signature verifier |

`CapabilityAdmissionReceipt` carries `decision`, `diagnostics`, `admitted_token_refs`, canonical `value`, and `receipt_ref`. Its ordinary decisions are `pass` and `deny`. The aggregate denies when any collected diagnostic exists, even if another token was admitted; inspecting only a nonempty admitted-token list is unsafe.

## Effect and resource inputs

Owners: [effect declarations](../../../src/effects/parts/mod/p000/body.rs), [profile admission](../../../src/effects/parts/mod/p006/body.rs), and [support checks](../../../src/effects/parts/mod/p009/body.rs).

| Type | Fields identifying the obligation | What is not established by the type |
|---|---|---|
| `DeclaredEffect` | `effect_id`, `operation`, `input_schema_ref`, `output_schema_ref`, `resource_class`, `capability_refs`, `evidence_refs` | A working live adapter |
| `EffectManifestInput` | Artifact kind/ref, executor kind, declared effects, policy/evidence refs | Current permission for every declared effect |
| `HandlerProfileInput` | Profile name, binding refs, `policy_ref`, `capability_context_ref`, resource/evidence refs | Numeric resource availability merely from nonempty refs |
| `HandlerProfileAdmissionInput` | Manifest/profile values, supported effects, determinism/replay classes, current policy/context refs, evidence refs | Discovery of current policy or revocations |
| `EffectRequestInput` | Artifact, effect/operation, profile name, input ref, capability/evidence refs | Validity of capability authority from reference possession |

Profile support compares input/output schemas, resource class, and capability vectors for an effect/operation pair. Capability-vector equality is the inspected implementation, not an unordered-set promise. Request admission instead checks that each declared capability reference appears in the request. These distinctions matter when diagnosing apparently equivalent inputs.

## Preflight artifact inventory

| Artifact | Producer/validator owner | Review purpose |
|---|---|---|
| Policy snapshot and `nickel-source` | Harness schema policy material | Bind policy, generated source, normalized export, and their references |
| `nickel-contract`, `basalt-preflight` | Harness policy preflight | Bind envelope and normalization evidence |
| `capability-gate-v1` | Harness capability material | Bind capability snapshot, contract, preflight, proofset, and grants |
| `handler-profile-admission-receipt-v1` | Effects profile admission | Record supplied policy/context and supported effect comparison |
| Effect binding receipt | `admit_effect_request` | Record request-to-manifest/profile binding decision |

All are evidence artifacts, not portable permission tokens. A correctly hashed historical artifact may be irrelevant to a changed request.

## Worked boundary distinction

Suppose a context and token both record expiry `8`. The currentness helper denies context use at logical time `8`; token diagnostics expire only when `at_tick > 8`. Likewise, a profile resource list can satisfy the local nonempty-list check without proving a live executor enforces those resources. Record the actual helper and comparison, then request evidence from the effect owner. Do not normalize these differences into an invented common rule.

## Sources

- [Handbook](../README.md)
- [Nominal reference contract](../../nominal-authority-references.md)
- [Preflight companion](../../technical/capabilities/policy-preflight-composition.md)
- [Effect manifest contract](../../effect-manifest-profiles.md)
- [Authority input declarations](../../../src/authority/parts/mod/p000/body.rs)
- [Capability input declarations](../../../src/capability/parts/tokens/p000/body.rs)
- [Policy material producer and parser](../../../src/harness/parts/schema/p019/body.rs)
- [Capability gate producer and parser](../../../src/harness/parts/schema/p020/body.rs)
- [Effect request admission](../../../src/effects/parts/mod/p002/body.rs)
