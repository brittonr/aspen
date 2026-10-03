# Inspecting current authority before an effect

Mode: How-to

## Goal and prerequisites

Use this procedure to determine whether an integration has enough explicit evidence to make a current authority decision before releasing an effect. It is a source-review and evidence-inspection procedure, not a universal admission command. The APIs below are source-checked; no runtime invocation was executed for this guide.

Have the exact proposed request, its caller identity binding, the selected context or proofset, the policy and resource inputs, and the caller's source location. For a currentness decision, also obtain the logical-time domain, epoch bounds, current-key source, and applicable revocation facts. If those facts cannot be supplied by the integration's legitimate owner, stop: an old passing receipt cannot fill the gap.

The [nominal reference contract](../../nominal-authority-references.md) distinguishes checked reference syntax from current authority. Use the [stale-authority companion](../../technical/capabilities/revocation-and-stale-authority.md) for the underlying model rather than treating this procedure as a replacement specification.

## 1. Select the decision boundary actually used

Find the caller, not just a convenient function with “admit” in its name. These are different paths:

- `authority_grant_currentness` accepts explicit principal, operation, time, epochs, keys, and revocations.
- `admit_authority` wraps currentness with facts partly taken from the context itself.
- `admit_capability` evaluates a token proofset against a request using separate matching rules.
- `AdmissionPolicy::decide_with_capabilities` combines local harness grants and policy deny rules.
- Effect manifest/profile admission checks effect declarations and supplied context references; it is not itself an external currentness service.

Record which path will run immediately before the proposed effect. A review that only demonstrates another helper's stricter checks leaves the real caller unreviewed.

## 2. Establish identity and requested action separately

For explicit currentness, compare `requested_principal_ref` with the context's subject, and retain capability, operation, and scope independently. The matching helper accepts several capability-name forms, including the combined capability/operation form; scope matching is exact or wildcard, not a hierarchical path-prefix interpretation.

For token admission, compare proofset and token holder, session, and context with the request. Token resource and ability also have exact comparisons. Preserve each mismatching dimension in the inspection record rather than summarizing everything as “bad credentials.”

Do not “fix” a mismatch by copying the context's subject into the request. The caller must establish the requested identity independently where the integration requires it.

## 3. Check currentness facts and their provenance

The [currentness implementation](../../../src/authority/parts/mod/p001/body.rs) denies grant epochs below the minimum or above the current epoch. Context validity begins at `not_before` and ends before `expires_at`. Nonempty context keys require an intersection with supplied current keys. Effective supplied revocations are checked against the context, subject, delegations, keys, and subject-bound capability references.

Ask who supplied each fact and whether it is appropriate for this invocation. The helper does not discover new revocations, read a wall clock, or contact a key service. Its deterministic answer is only as current as the supplied facts.

Inspect wrapper use carefully: `admit_authority` supplies the context subject as principal, uses context keys as current keys, sets minimum epoch to zero, and uses logical time as current epoch. This is a source-observed limitation of that entry point, not evidence that every caller independently authenticates identity or observes rotation.

## 4. Check policy and resources without substituting receipts

For token admission, inspect required policy and resource references against the proofset lists. These are membership checks, not evaluation of every referenced policy or measurement of resource availability. Token caveats are matched against the supplied caveat context; the helper does not independently establish the truth of those strings.

At the effect boundary, [profile admission](../../../src/effects/parts/mod/p006/body.rs) takes current policy/context references, supported effect declarations, determinism/replay classes, and evidence. Its [diagnostics](../../../src/effects/parts/mod/p009/body.rs) check reference equality, nonempty binding/resource/evidence lists, and matching schemas, resource class, and capability lists. The caller still owns how “current” references are obtained and how resource bounds are actually enforced.

Then inspect request admission: declared effect/operation, artifact identity, handler profile, and required capability references must match. Neither a profile receipt nor a request binding receipt should be promoted to an operating-system capability.

## 5. Work one negative case before approving release

The checked-in [currentness case](../../../src/authority/parts/mod/tests/m000/p001/body.rs) permits `node-control:status` in scope `node:control`, valid over logical times `[2, 8)`. At time `5`, grant epoch `4` lies between minimum `3` and current `5`, and the supplied current key matches. The test then changes scope, time to `8`, epoch to `2`, key, and delegation revocation independently, expecting distinct denials.

Apply that method to the real request: vary one relevant input while retaining the others. Require evidence that a denial stops dispatch, not merely that a receipt contains a failure string. For the harness, the denied-turn branch returns before step application; for another shell, inspect its actual release boundary.

## 6. Record a bounded conclusion

Keep request/context identities, input provenance, decision diagnostics, and the dispatch boundary together. If the effect may already have occurred, investigate the recorded outcome before any retry. Do not delete state, weaken admission, or replace current facts with historical evidence.

Approval should name the exact integration and input snapshot reviewed. Missing currentness provenance, uninspected dispatch, or absent resource enforcement is a stop condition, not a reason to claim partial checks authorize the effect.

## Sources

- [Handbook](../README.md)
- [Nominal reference contract](../../nominal-authority-references.md)
- [Stale-authority companion](../../technical/capabilities/revocation-and-stale-authority.md)
- [Currentness and wrapper](../../../src/authority/parts/mod/p001/body.rs)
- [Capability and revocation matching](../../../src/authority/parts/mod/p004/body.rs)
- [Token matching rules](../../../src/capability/parts/tokens/p001/body.rs)
- [Currentness negative cases](../../../src/authority/parts/mod/tests/m000/p001/body.rs)
- [Effect profile contract](../../effect-manifest-profiles.md)
