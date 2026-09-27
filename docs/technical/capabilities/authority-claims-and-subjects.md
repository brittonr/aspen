# Authority Claims and Subjects

A statement about an artifact is not the same thing as authority to make that statement, and authority to attest is not permission for a consumer to act on it. Molten represents these boundaries with subject selectors, authority claims, claim admission, and subsystem use decisions. This article follows the inspected claim helpers while keeping their limited validation distinct from the broader contract in the [architecture](../../architecture.md) and [nominal authority documentation](../../nominal-authority-references.md). Start from the [Technical companion](../README.md) for related evidence topics.

## Three identities to keep separate

The attesting party, the subject being described, and the consumer's intended action are different objects. `AuthorityClaim` therefore includes issuer, holder, session, and context references alongside a `subject_selector_ref`, exact subject references, claim kind, claim value reference, and evidence. It also carries policy/resource references, freshness, revocation references, and caveats. A subject cannot acquire authority merely by being named in that record ([claim structures](../../../src/capability/claim/parts/authority/p000/body.rs)).

`ClaimSubjectSelector` makes selection itself explicit. Supported selector constants include exact reference, reference prefix, artifact class, namespace, schema identity, release channel, cluster identity, and policy-defined selection. The broad kinds are enumerated separately from exact-reference selection. A selector is canonically encoded and hashed; its identity is then bound by the claim. This allows the authority question to concern a described subject domain rather than assuming that one content hash is the only possible selection mechanism.

Broad selection increases interpretive responsibility. The admission helper rejects a broad selector with no visible caveats. That check establishes the presence of attenuation material, not the semantic correctness of an arbitrary selector expression or the sufficiency of its caveats ([admission diagnostics](../../../src/capability/claim/parts/authority/p001/body.rs)).

## Attestation authority has a specific request shape

`claim_capability_request` requests ability `claim:attest`, token kind `external-claim-authority`, resource equal to the selector reference, and scope equal to the claim kind. Holder, session, context, tick, and required local policy/resource references are supplied explicitly. This is a useful separation: authority over a selector and claim kind is not a general-purpose permission to deploy, execute, retain, or promote every selected object.

`admit_authority_claim` computes the selector reference and denies a claim bound to another selector. It then computes the claim reference and gathers diagnostics. Admission requires a passing supplied capability admission with admitted token references, nonempty UCAN verification and Basalt enforcement reference lists, local policy/resource references, and freshness references. It checks issuer revocation against the supplied revocation list. Transport observations, registry discovery, and local fixture grant references have explicit diagnostics preventing their use as substitutes for missing claim authority ([claim admission](../../../src/capability/claim/parts/authority/p001/body.rs)).

These checks are bounded evidence assembly and validation. The inspected helper does not fetch the referenced UCAN or Basalt receipts and reexecute their verification. Its capability-request comparison constructs the request from the claim inputs; it does not independently parse the supplied capability admission's encoded request. Consequently, a passing helper result should not be described as a complete cryptographic proof binding every external receipt to the claim. The governing separation of UCAN/Basalt authority remains the contract; these observed checks are narrower.

## Claim admission and downstream use

`decide_claim_use` validates reference syntax, requires a passing admission, compares the required selector reference, checks that the admission text contains the required claim kind, and requires nonempty subsystem policy/resource references. It records subject, subsystem, freshness, and the admission reference in a canonical decision value. The emitted caveat explicitly says claim evidence remains evidence-only until the exact subsystem gate consumes a matching admitted claim and does not grant unrelated trust ([use-decision implementation](../../../src/capability/claim/parts/authority/p001/body.rs)).

An important limit follows directly from that code. The helper does not evaluate subject membership in a selector, and validating a freshness reference's syntax is not checking that it is temporally current. Those obligations cannot be inferred from the presence of `subject_ref` and `freshness_ref` in the output. A downstream subsystem must not treat the generic use record as a universal membership or freshness oracle.

## Worked confused-subject scenario

Consider an illustrative auditor permitted to attest a particular claim kind for selector S describing one release channel. The auditor emits a claim bound to S. A consumer requires selector T for a different channel. Even if the issuer is familiar and the claim is discoverable in a registry, exact selector mismatch is a reason for denial. A peer connection and successful transport only explain how the candidate evidence arrived.

Now hold S constant but present a subject outside S's intended domain. The inspected generic use helper does not establish domain membership. A review that stops at its passing decision would miss this distinction. The correct conclusion is not that the subject was proven eligible; it is that the helper's particular admission, selector-reference, claim-kind, and local-evidence checks passed. Semantic membership remains a separate boundary to inspect.

## Verification and review

Suggested review tracks issuer authority, selector identity, claim identity, and consumer requirements independently. Check changed-selector denial, missing local policy/resource evidence, revoked issuer, and discovery-only inputs. For actual subsystem integration, inspect how selector membership and freshness are established rather than merely checking that fields are populated. Verify the provenance of the supplied capability admission and referenced verification receipts. No runtime checks or cryptographic verification are reported as executed by this documentation.

## Limits and non-claims

Names, network peers, and registry entries grant no claim authority. Canonical hashing establishes identity of encoded claims, not truth of the claim value. The inspected helpers' syntactic and evidence-presence checks are not a complete proof of downstream eligibility. Filesystem capabilities likewise concern operational local access, not attestation truth ([filesystem boundary](../../local-filesystem-authority.md)). This distinction preserves the authority boundary without overstating the present helper implementations.

## Sources

- [Architecture](../../architecture.md)
- [Nominal authority references](../../nominal-authority-references.md)
- [Local filesystem authority](../../local-filesystem-authority.md)
- [Claim structures and request construction](../../../src/capability/claim/parts/authority/p000/body.rs)
- [Claim admission, use, and peer diagnostics](../../../src/capability/claim/parts/authority/p001/body.rs)
- [Underlying capability token admission](../../../src/capability/parts/tokens/p000/body.rs)
