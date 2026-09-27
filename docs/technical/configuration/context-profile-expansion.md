# Context Profile Expansion

Operator context profiles package explicit references for a named operation. Expansion reduces repetitive input without converting a convenient bundle into authority. This article assumes familiarity with content references and subsystem gates. It accompanies the [Technical companion](../README.md); the [proof workflow](../../proof-workflow.md) governs the evidence-only role of context expansion.

## Profiles as structured inputs

The [context core](../../../src/operator/context/parts/profile/p000/body.rs) defines `ContextRefSet` with policy, capability, authority, resource, evidence, redaction, and retention reference vectors. `ContextProfileInput` adds a profile identifier, tier, allowed operations, and caveats. Supported tiers are `local`, `pilot`, and `release`; recognition of a tier is not a release eligibility decision.

`OperationRequirements` supplies the requested operation and booleans indicating whether policy, authority, resource, evidence, and retention references are required. There are no corresponding capability or redaction requirement booleans in this structure. Callers should not infer those additional checks from the fact that the profile can carry those categories.

Building the profile artifact validates text, collection bounds, reference syntax, tier, and duplicate operation scopes. It then emits a canonical `context-profile-v1` value and computes its content reference. Some invalid structures produce errors, while diagnostic-bearing inputs can produce a denied artifact. A content-addressed denial is still useful evidence: it records what was evaluated without accepting the proposed context.

## Expansion mechanics

`expand_context_profile` first constructs the profile artifact and carries its diagnostics forward. It validates the operation and overrides, checks exact membership in `allowed_operations`, merges references, and checks required categories for emptiness. Diagnostics are sorted and deduplicated; their absence selects `pass`. The [serialized expansion](../../../src/operator/context/parts/profile/p001/body.rs) binds the profile reference, operation, requirement booleans, supplied overrides, expanded references, diagnostics, and evidence-only caveat.

The override behavior is category-specific rather than a generic last-writer-wins merge:

- Policy, authority, and resource overrides are checked against the profile using set equality. A nonempty different set adds a conflict diagnostic.
- Evidence overrides are additive. An ordered set combines profile and override evidence, deduplicating references.
- Capability and redaction references are copied from the profile; the override input has no fields for replacing them.
- Retention overrides use the same selection helper as the non-additive categories, but the current function does not add a corresponding conflict diagnostic.

For a conflicting policy, authority, or resource override, the helper can still place the override vector in `expanded_refs`; the overall decision remains denied. A consumer that reads only the vector and ignores the decision would cross the intended boundary. Denial does not mean the artifact contains no proposed values.

## Scoped discrepancy: retention overrides

The [proof workflow](../../proof-workflow.md) describes expansion denials for conflicting overrides in broad terms. The inspected [`merge_refs` implementation](../../../src/operator/context/parts/profile/p000/body.rs) explicitly diagnoses policy, authority, and resource conflicts, but not retention conflicts. A nonempty different retention vector is selected, subject to reference validation and required-presence checks. This article does not interpret that omission as an approved retention-policy exception or amend the governing description. It records the narrower observed behavior; intended treatment of conflicting retention overrides remains a source discrepancy for review.

A similar distinction applies to diagnostic terminology. Reference validation can report `stale-ref`, but the inspected helper delegates to content-reference validation without retrieving external objects or checking their lifecycle. That diagnostic name is not proof that a remote revocation or freshness check occurred.

## Worked reasoning: installation context

Consider an illustrative profile allowing `node.status` and `node.install`, carrying reviewed policy P, authority A, resource R, and evidence E. An installation request requires those four categories. Adding evidence F yields the union of E and F; it does not replace P, A, or R. This is useful when an operator attaches an additional observation without silently switching the decision context.

Now replace authority A with a different authority reference B. The authority override is a different set, so expansion records `conflicting-authority-override` and denies. Even if B is syntactically valid and the expanded vector contains B, the expansion is not permission to install. Finally, requesting `retention.delete` from this profile is denied as unsupported scope, independently of whether retention references happen to be present. Category presence and operation scope solve different problems.

These cases correspond to [existing context tests](../../../src/operator/context/parts/profile/p002/body.rs), which cover additive evidence, conflicting authority, malformed references, unsupported operations, and attempts to use the profile itself as mutation authority.

## Verification and non-claims

Suggested verification, not executed here, is the existing `context_profile` library and CLI coverage named in the proof workflow. Review both decisions and expanded values, and inspect set equality separately from artifact identity: comparison ignores order for conflict purposes, but serialized vectors are not universally normalized.

`evaluate_context_profile_authorization_use` always denies profile-as-authority use, even when expanded authority references are provided. Downstream gates still interpret those references. Expansion does not fetch proofs, establish freshness, enforce runtime hard caps, grant retention clearance, perform an installation, or establish production readiness. [Runtime-limit admission](../../runtime-limit-profiles.md), for example, remains a separate pure decision over limits and hard-cap descriptors.

## Sources

- [Proof workflow and context-profile boundary](../../proof-workflow.md)
- [Runtime limit profiles](../../runtime-limit-profiles.md)
- [Context profile construction and override merge](../../../src/operator/context/parts/profile/p000/body.rs)
- [Required references and canonical expansion representation](../../../src/operator/context/parts/profile/p001/body.rs)
- [Context expansion regression cases](../../../src/operator/context/parts/profile/p002/body.rs)
