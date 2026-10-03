# Capability Context Admission

Capability admission is not one universal boolean attached to an identity. Molten has several deliberately different representations: local runtime grants, canonical authority contexts, token proofsets, and nominally typed reference sets. This article explains how to reason about those representations without silently transferring guarantees between them. Readers should first understand the [architecture](../../architecture.md) and [nominal reference boundary](../../nominal-authority-references.md). The [Technical companion](../README.md) places this discussion alongside other subsystem articles.

## Separate the representations

The local runtime `CapabilityContext` contains `CapabilityGrant` values. A grant constrains four request dimensions: `actor`, `action`, `target`, and `value`. Each is optional. In `CapabilityGrant::matches`, an absent constraint matches any request value in that dimension; a present constraint requires equality. Authorization succeeds on the first matching grant and returns that grant as part of `CapabilityAuthorization`. This is a finite request-matching mechanism, not a delegation-chain verifier ([runtime admission implementation](../../../src/runtime/admission/mod.rs)).

A canonical authority `Context` is richer and different. It carries a subject, scoped capabilities, delegation references, optional validity bounds, revocation references, keys, policy references, evidence references, and its Preserves value. `parse_context` computes the context reference from the canonical value. The `Capability` entries in this representation use name, scope, and attenuation strings rather than the runtime grant's four optional fields ([context structures and encoding](../../../src/authority/parts/mod/p000/body.rs)). A reader should therefore avoid describing a local grant as if it automatically includes the context's temporal or key constraints.

Nominal admission is another boundary. `admit_context_refs` constructs `AuthorityContextRef`, `PrincipalRef`, and the appropriate canonical reference types from selected strings. Checked construction establishes category syntax and prevents accidental alias substitution inside typed APIs. It does not establish that referenced evidence exists, that the subject holds a valid delegation, or that current policy permits an operation ([wire admission](../../../src/authority/nominal.rs)).

## Admission is a conjunction, not possession

`AdmissionPolicy::decide_with_capabilities` first invokes context authorization. Missing authority produces a denial before deny-rule policy evaluation. With a matching grant, policy rules can still deny the request. Thus “a matching grant exists” is necessary on this path but insufficient for admission. Conversely, calling `AdmissionPolicy::decide` alone evaluates policy without performing that capability check. Review must follow the entry point actually used, not just the types present nearby.

There is an important implementation qualification to the phrase “deny by default.” An empty `CapabilityContext::from_grants` denies every request, but `CapabilityContext::default()` calls `allow_all()`. The evidence-bearing harness compensates at its own boundary by rejecting suites without an explicit capability fixture, alongside explicit actors and budgets, before collecting runtime traces ([runner preparation](../../../src/harness/parts/runner/p000/body.rs)). The deny-by-default architectural intent is not evidence that every constructor in every layer defaults to denial.

For canonical contexts, `authority_grant_currentness` evaluates supplied facts: requested principal, capability and operation, scope, logical time, epoch interval, current keys, and revocations. A nonempty diagnostic set yields `fail`. Parsing errors instead return an error. Consumers should preserve that distinction between a validly formed denied request and a request that never entered the decision domain ([currentness implementation](../../../src/authority/parts/mod/p001/body.rs)).

## Worked reasoning example

Consider an illustrative actor `publisher` with a local grant limited to one action, target `catalog-a`, and an exact Preserves payload. The actor sends the same action to `catalog-b`. Actor identity and action match, but target equality fails; possessing the grant does not authorize this request. If the target is corrected but the payload changes, exact-value matching can still fail. If all four dimensions match, a policy deny rule may nevertheless reject publication.

Now suppose the successful local request is used as evidence for a separate authority-context request. That new request cannot inherit currentness merely from the local result. Its principal must match, its capability and scope must be covered, and its supplied temporal, epoch, key, and revocation facts must pass the other checker. These are distinct predicates over distinct inputs, not repeated spellings of the same authorization.

Finally, neither decision opens a filesystem directory. Local storage containment requires an acquired typed root and operations through its `cap_std::fs::Dir`; nominal strings and policy permission are not operational filesystem authority ([filesystem boundary](../../local-filesystem-authority.md)).

## Verification and review

Suggested review should trace a request from fixture or wire decoding to the exact authorization entry point, then to the effect boundary. Exercise absent versus explicit-empty fixtures separately; test each constrained grant dimension; and retain a case where capability matching succeeds but policy denies. For canonical contexts, distinguish malformed references from currentness diagnostics and compare canonical receipt bindings, not debug output. These are suggested checks, not executions reported by this article.

## Limits and non-claims

A local matching law is not cryptographic authentication. Nominal typing is not freshness. Canonical receipt identity is not renewed permission, and a passing harness fixture is not production authority. No filesystem, network, clock, or other ambient effect is implied by these in-memory decisions. The constructor/default qualification above remains visible rather than being generalized away.

## Sources

- [Architecture](../../architecture.md)
- [Nominal authority references](../../nominal-authority-references.md)
- [Local filesystem authority](../../local-filesystem-authority.md)
- [Runtime grant and policy implementation](../../../src/runtime/admission/mod.rs)
- [Authority context representation](../../../src/authority/parts/mod/p000/body.rs)
- [Currentness and admission](../../../src/authority/parts/mod/p001/body.rs)
- [Nominal wire admission](../../../src/authority/nominal.rs)
- [Harness preparation](../../../src/harness/parts/runner/p000/body.rs)
