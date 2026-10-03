# Delegation and Attenuation

Delegation explains how authority is carried from an issuing context toward a particular use; attenuation explains which uses remain available. Neither concept is equivalent to copying a reference into a record. This article examines the inspected token and authority-context mechanisms, including the limits of their delegation evidence. It assumes the [nominal reference model](../../nominal-authority-references.md) and the policy/effect separation in the [architecture](../../architecture.md). See the [Technical companion](../README.md) for adjacent topics.

## Authority as a constrained set of requests

A useful reasoning model treats a grant as a set of requests that it permits. Attenuation is monotonic only when a derived grant's permitted set is contained in its parent's. Narrowing the holder, session, resource, operation, scope, or lifetime can reduce that set; merely adding an “attenuated” label cannot establish containment. This is a reasoning model, not an additional Molten API or a claim that every helper computes set inclusion.

The concrete `CapabilityToken` binds issuer, holder, session, context, resource, ability, scope, attenuation, caveats, expiry tick, and several reference lists. `CapabilityProofset` supplies its own holder/session/context boundary and policy, resource, revocation, and evidence references. `CapabilityRequest` supplies the corresponding expected bindings and required policy/resource references ([token data and admission](../../../src/capability/parts/tokens/p000/body.rs)). These independent bindings prevent a token's mere presence from being the whole authorization question.

`token_diagnostics` requires exact holder, session, context, resource, and ability equality. Scope is either equal or the literal wildcard. Wildcard scope is rejected unless the attenuation string is exactly `attenuated`. Each token caveat must occur in the request's `caveat_context`. Required policy and resource references are checked against the proofset separately ([diagnostics](../../../src/capability/parts/tokens/p001/body.rs)). This caveat check is exact string membership; it is not an interpreter for arbitrary predicates.

## Do not conflate attenuation vocabularies

Canonical authority contexts use another representation and another matcher. Their scope check also accepts equality or wildcard, but `attenuation_allows` accepts `scoped`, `unattenuated`, or `*`. Capability names can match the requested capability, requested operation, their colon-combined form, or wildcard ([context matching](../../../src/authority/parts/mod/p004/body.rs)). The token checker instead requires exact ability equality and uses the `attenuated` marker for wildcard scope.

These are observed helper semantics, not interchangeable policy languages. In particular, the context encoder's `attenuation-monotonic` check label does not make the simple matcher a parent-child containment verifier. The [governing nominal documentation](../../nominal-authority-references.md) assigns capability, caveat, delegation, revocation, key, policy, and resource decisions to UCAN and Basalt. That ownership statement should not be read as proof that every lower-level record constructor independently verifies all those properties.

## Delegation references versus verified derivation

In token admission, a delegation is rejected when one of the token's `delegation_refs` appears in the proofset's `revocation_refs`. The inspected helper does not traverse parent grants, validate signatures, or prove monotonic authority reduction. It hashes the token's canonical value and evaluates the supplied facts.

The separate UCAN verification receipt input records proof references, verification keys, caveat decisions, revocation facts, replay facts, derived grants, and request bindings. Its `UcanVerificationChecks` includes signature, audience, holder, session, context, time, proof, revocation, caveat, and replay booleans. Receipt construction diagnoses false checks and missing proof/key/derived-grant lists. This is an evidence-construction boundary over supplied verification results, not itself evidence that cryptographic verification was executed by the constructor ([receipt construction](../../../src/capability/parts/tokens/p000/body.rs), [receipt diagnostics](../../../src/capability/parts/tokens/p001/body.rs)).

## Worked delegation failure

An illustrative parent authority permits reading a dataset; a derived token is intended for worker A in session S. Assume its resource, ability, and exact scope match a request, and all required proofset references are present. Moving that token to session T fails session matching even if worker A remains the holder. Reusing it after its delegation reference is included in the supplied revocation set fails delegation currentness as well.

Now add a second token with the wrong session to an otherwise passing proofset. `admit_capability` accumulates diagnostics from all token outcomes. Its final pass requires no diagnostics and at least one admitted token. Therefore a good token does not neutralize the bad token; this helper is not a “choose any successful token and ignore the rest” evaluator. Proofset construction and review should account for that observable behavior rather than treating extra candidates as harmless.

## Verification and review

Suggested verification separates three questions: are boundary fields equal, are attenuation conditions satisfied by this checker, and does external evidence actually establish valid delegation? Include changed-session, revoked-delegation, wildcard-without-marker, missing-caveat, and mixed-good/bad-proofset cases. Review the origin of every supplied verification boolean and reference list. A receipt parser recognizing a passing record is not a substitute for that upstream provenance analysis. These checks are guidance; no commands are reported as executed here.

## Limits and non-claims

The inspected helpers do not prove an arbitrary delegation chain sound. Strings called caveats are not automatically executable policy. Nominal `DelegationRef` establishes checked reference syntax and category separation, not chain validity. Filesystem capability roots enforce a different operational boundary and cannot be derived from token attenuation alone ([filesystem authority](../../local-filesystem-authority.md)). None of these observations establishes distributed revocation propagation, production readiness, or permission to perform unrelated effects.

## Sources

- [Architecture](../../architecture.md)
- [Nominal authority references](../../nominal-authority-references.md)
- [Local filesystem authority](../../local-filesystem-authority.md)
- [Token and verification receipt structures](../../../src/capability/parts/tokens/p000/body.rs)
- [Token and verification diagnostics](../../../src/capability/parts/tokens/p001/body.rs)
- [Authority-context matching](../../../src/authority/parts/mod/p004/body.rs)
- [Context encoding and check labels](../../../src/authority/parts/mod/p000/body.rs)
