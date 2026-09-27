# Reviewing a policy change with negative cases

Mode: Review checklist

Use this checklist to review a policy change as a change in permitted behavior, not merely a successful normalization or a new receipt hash. A review packet should identify the changed policy, affected caller, proposed effect, source revision, positive case, negative cases, and observed execution evidence. This checklist is source-checked only: the cited tests and branches were inspected, not executed for this document.

Consult the [preflight companion](../../technical/capabilities/policy-preflight-composition.md) for theory and the [Nickel toolchain contract](../../nickel-toolchain.md) for cohort ownership. Neither an evaluator success nor a fixture report establishes release readiness.

## Scope and caller acceptance

- [ ] **Is the exact effect boundary named?** Require the caller path and the function that releases the effect after admission. A list of available admission helpers is not evidence that the affected caller uses them.
- [ ] **Are policy and capability obligations separated?** Show a matching grant with no matching deny rule for the positive case. Show missing capability authority independently of explicit policy refusal. Inspect `decide_with_capabilities`, not only the policy-only `decide` method.
- [ ] **Are identity and target dimensions retained?** Record actor or principal, action or operation, target/scope, and relevant value. Avoid a positive case made permissive by accidentally replacing a constrained field with an unconstrained optional field.
- [ ] **Is the claim limited to the integration exercised?** A local harness grant is fixture evidence. It must not be described as externally verified UCAN authority or proof that a live adapter dispatched successfully.

Acceptance evidence should connect input, decision, and observable boundary. An isolated pass string is insufficient.

## Policy material and evidence binding

- [ ] **Does the packet bind the actual changed policy?** Require the canonical policy snapshot and the generated Nickel source/export relationships. [Policy parsing](../../../src/harness/parts/schema/p019/body.rs) recomputes hashes and normalization; retaining old gate material after changing policy must not be accepted.
- [ ] **Are backend and contract identities reviewed?** Check contract version, input/output schemas, receipt schema, and normalized-source reference. Do not accept arbitrary Nickel evaluation as equivalent to the selected Molten contract.
- [ ] **Does changed capability material invalidate its old evidence?** [Capability validation](../../../src/harness/parts/schema/p020/body.rs) compares snapshot and grant refs and reconstructs expected gate evidence. Require a case that changes material without updating its evidence graph.
- [ ] **Are negative artifacts preserved as negative?** Missing Basalt authority preflight, tampered grant bindings, and unchecked UCAN proof material have dedicated [test cases](../../../src/harness/parts/mod/tests/m000/p003/body.rs). A diagnostic failure artifact must not be submitted as a passing report.

Request evidence of semantic rejection and boundary behavior, not just assertions that source text contains particular labels.

## Currentness and token negative matrix

For each applicable row, record the precise input change, expected refusal boundary, and actual observation. “Not applicable” needs a callsite explanation.

| Case | Review question | Required evidence |
|---|---|---|
| Wrong principal/session/context | Does the request remain bound to its legitimate caller? | Original and changed binding plus denial before effect release |
| Wrong operation or scope | Can a narrow permission leak into another action? | One changed action dimension, unchanged remaining inputs |
| Expiry boundary | Which helper defines the endpoint? | Just-before, equal, and just-after cases using its actual time domain |
| Epoch drift | Are both stale and future grants rejected? | Minimum/current epoch provenance and boundary decisions |
| Key rotation | Does a disjoint current-key set deny? | Independently supplied current-key facts, not copied context keys |
| Revocation | Is the target effective and applicable? | Target, effective time, matcher path, and refusal |
| Missing caveat/policy/resource input | Are required inputs actually checked? | Exact omitted requirement and corresponding decision |

Do not merge endpoint conventions silently: [currentness](../../../src/authority/parts/mod/p001/body.rs) rejects at context expiry equality, while [token diagnostics](../../../src/capability/parts/tokens/p001/body.rs) reject only above token expiry. Record this source-review discrepancy wherever a change crosses those APIs. Likewise, the convenience authority wrapper supplies several facts from the context itself; independent identity/key claims require caller evidence.

## Effect suppression and resource acceptance

- [ ] **Does a denial prevent the effect, rather than annotate it?** Identify the early return or equivalent suppression in the real shell. The harness [denied branch](../../../src/harness/parts/runner/p001/body.rs) rolls back before step application; another integration needs its own evidence.
- [ ] **Does the manifest/profile comparison use the changed inputs?** Require artifact/effect/operation identity, schema compatibility, resource class, capability requirements, and legitimate current policy/context refs.
- [ ] **Are resources enforced outside the receipt?** The [profile helper](../../../src/effects/parts/mod/p009/body.rs) checks nonempty resource refs and support matching. Require the effect owner's actual resource-enforcement evidence rather than upgrading that list check into a capacity guarantee.
- [ ] **Are uncertain prior outcomes handled safely?** If the review involves a failed live invocation, establish its outcome before proposing another. No unconditional retry, deletion, or bypass is acceptable evidence of a safe policy transition.

## Worked review decision

Use `report_validation_rejects_effect_response_after_denial` as a model negative case. It supplies a clock grant for `producer` and a policy refusal, builds a report, inserts forbidden effect evidence after rollback, and expects validation to reject the tampering. A policy change that merely retains a passing preflight result does not satisfy this case: the relevant evidence is that denied execution stays suppressed and altered reports cannot legitimize it.

Close the review only when the positive behavior, meaningful negative boundaries, currentness provenance, and effect-release observation agree. Record unexecuted cases as unexecuted. If current facts or the live dispatch path are unavailable, limit the conclusion to source/fixture review rather than approving live authority or release readiness.

## Sources

- [Handbook](../README.md)
- [Policy preflight companion](../../technical/capabilities/policy-preflight-composition.md)
- [Nickel toolchain contract](../../nickel-toolchain.md)
- [Nominal reference contract](../../nominal-authority-references.md)
- [Policy and capability composition](../../../src/runtime/admission/mod.rs)
- [Preflight and denial negative cases](../../../src/harness/parts/mod/tests/m000/p003/body.rs)
- [Currentness negative cases](../../../src/authority/parts/mod/tests/m000/p001/body.rs)
- [Effect manifest/profile contract](../../effect-manifest-profiles.md)
