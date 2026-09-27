# Diagnosing denied authority contexts

Mode: Troubleshooting

Start by identifying the helper that produced the result. A malformed reference error, currentness `fail`, capability `deny`, policy refusal, and report-validation error are not interchangeable failures. Preserve the request and exact supplied evidence before changing anything. The guidance below is source-checked, not an executed incident reproduction; diagnostics quoted here come from source, not captured terminal output.

The [stale-authority companion](../../technical/capabilities/revocation-and-stale-authority.md) supplies background. The operational objective here is to find the smallest discriminating fact without broadening authority, deleting state, or replaying an uncertain effect.

## Symptom: a well-formed context still fails

**Discriminating evidence:** inspect the diagnostic vector from [currentness](../../../src/authority/parts/mod/p001/body.rs), not merely whether the context parsed.

| Diagnostic | Compare | Safe next action |
|---|---|---|
| `principal-mismatch` | Requested principal versus context subject | Recheck caller identity binding; do not copy the subject into the request |
| `capability-denied` | Capability, operation, exact scope, attenuation | Locate the intended narrow grant and actual dispatched operation |
| `stale-epoch` | Grant epoch below minimum | Obtain an appropriately current grant through its owner |
| `not-yet-current-epoch` | Grant epoch above current epoch | Inspect epoch provenance and ordering; do not advance counters manually |
| `not-yet-valid` | Logical time below context start | Verify the time domain and intended validity interval |
| `expired` | Logical time at or beyond context expiry | Treat the old context as historical evidence, not renewable permission |
| `key-not-current` | Nonempty context keys disjoint from supplied current keys | Inspect rotation facts and their source |
| `revoked` | Effective supplied target matching | Identify the matching revocation and legitimate replacement process |

Several diagnostics may apply simultaneously. Correcting one does not prove the next evaluation will pass. Stop if the integration cannot establish current facts; a syntactically valid replacement string does not solve missing provenance.

## Symptom: a token looks usable but admission denies

**Discriminating evidence:** inspect both proofset-boundary and per-token diagnostics in [token matching](../../../src/capability/parts/tokens/p001/body.rs). Proofset holder/session/context mismatches differ from token mismatches. Required policy/resource references are checked against proofset lists; token resource and ability are compared directly with the request.

An asterisk token scope additionally requires attenuation exactly `attenuated`. Caveats require matching supplied caveat strings. Revocation checks here concern issuer and delegation references in the supplied proofset revocation list; do not assume the helper consumes every serialized revocation-related field.

**Safe next action:** retain the full diagnostic set and the token references associated with failures. In [aggregate admission](../../../src/capability/parts/tokens/p000/body.rs), one valid token does not cancel another token's diagnostics. A receipt can contain admitted-token references and still deny.

**Stop condition:** do not strip inconvenient proofset members or caveats simply to obtain a passing result. Determine the intended proofset composition with its authority owner, then evaluate that legitimate input.

## Symptom: preflight passes but the action is refused

**Discriminating evidence:** distinguish the policy/capability gate from the per-step result. [Runtime admission](../../../src/runtime/admission/mod.rs) first checks capability grants and then policy deny rules. `missing capability grant` means that the policy's apparent permissiveness is irrelevant. A matching grant plus a matching deny rule is also denied.

**Worked case:** the checked-in `deny-effect-validation` fixture gives `producer` a clock grant and then explicitly denies that clock action. The [test](../../../src/harness/parts/mod/tests/m000/p003/body.rs) expects validation to reject a forged effect request/response inserted after rollback. The right investigation is whether dispatch was suppressed, not how to make the policy gate fail earlier.

**Safe next action:** locate the request's actor/action/target/value and the matching rule. Preserve the original report; inspect the [denied-turn dispatch](../../../src/harness/parts/runner/p001/body.rs) before considering a later invocation. A passing harness report can accurately describe a denied action.

## Symptom: effect profile says “stale or revoked”

**Discriminating evidence:** [profile diagnostics](../../../src/effects/parts/mod/p009/body.rs) use that wording when profile and supplied current capability-context references differ. This comparison does not itself query a revocation service or prove which reason caused the difference. Policy-reference mismatch is checked separately.

**Safe next action:** inspect who supplied `current_policy_ref` and `current_capability_context_ref`, and whether they belong to this request. For resource denial, distinguish an empty resource-reference list from absent enforcement of the referenced bounds. For effect support mismatch, compare schema refs, resource class, and capability vectors, including order.

**Stop condition:** do not relabel an old profile with current references or treat nonempty resource refs as proof of live availability. Obtain a profile admitted against legitimate current inputs and evidence of the actual shell boundary.

## Symptom: equivalent expiry or revocation inputs disagree

Context currentness expires at equality; token expiry uses a strict greater-than comparison. The [revocation matcher](../../../src/authority/parts/mod/p004/body.rs) respects effective time, while the cleanup helper in the currentness implementation filters matching supplied assertions without that timing check. These are scoped source-review observations, not reproduced defects.

Identify the exact helper and time domain before interpreting the disagreement. Cleanup is not an admission check and is not a safe diagnostic command. Preserve state and escalate the contract question with the concrete callsite and boundary values. Never “repair” an incident by deleting authority-bound state or unconditionally retrying an effect whose outcome is unknown.

## Sources

- [Handbook](../README.md)
- [Stale-authority companion](../../technical/capabilities/revocation-and-stale-authority.md)
- [Nominal reference non-claims](../../nominal-authority-references.md)
- [Currentness diagnostics and cleanup](../../../src/authority/parts/mod/p001/body.rs)
- [Token diagnostic rules](../../../src/capability/parts/tokens/p001/body.rs)
- [Effect profile diagnostic rules](../../../src/effects/parts/mod/p009/body.rs)
- [Denied-effect and tampered-evidence cases](../../../src/harness/parts/mod/tests/m000/p003/body.rs)
