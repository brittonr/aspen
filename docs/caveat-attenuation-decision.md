# Caveat attenuation boundary decision

Decision: **add no new caveat, rewrite, or payload-filter authority format**.
Date: 2026-10-15. Owner: Molten runtime-spine and authority maintainers.

r[impl molten.runtime_spine.caveat_authority_boundary] The reviewed consumers need scoped
allow/deny admission, not a new language that transforms delivered Preserves values.
Keep UCAN responsible for token/delegation/caveat verification, Basalt responsible for
policy enforcement, and Molten responsible for binding those decisions to its concrete
holder, session, operation, resources, and current state. This decision changes no runtime
behavior and does not certify the existing adapters or their supplied observations.

## Current need inventory

The inventory covers the attenuation implementations and labels in `src/` and the active
workspace core/host crates, including the five surfaces named by the change design.
Test fixtures and descriptive labels are distinguished from enforcement.

| Surface and consumer | Required narrowing and source | Classification |
|---|---|---|
| Plugin hostcalls | Grant resource refs and delegated scope, delegation-depth limit, evaluation-turn window, and budgets. [Grant matching](../src/plugin/parts/host/p002/body.rs) checks the supplied grant against the operation; [grant records](../src/plugin/parts/host/p012/body.rs) validate its declared attenuation. | Scope restriction; no payload rewrite. |
| Runtime authority contexts and capability requests | Bind resource/ability, holder, session, context, expiry, revocation, and admitted policy/resource refs. [Authority matching](../src/authority/parts/mod/p004/body.rs) recognizes scoped labels; [capability admission](../src/capability/parts/tokens/p001/body.rs) compares caveat strings with supplied context. | Scope restriction. Matching strings is not a general UCAN caveat interpreter or proof that external facts are true. |
| Effect handles | Retain run/session and any parent actor/turn; child operations must be a subset, expiry cannot widen, and transfer cannot become more permissive. [Handle derivation](../src/effects/parts/mod/p003/body.rs) uses the existing scope, operation, and transfer checks. | Scope restriction; payload unchanged. |
| Remote sessions, control tickets, and bootstrap offers | Limit endpoint/topic and separately require an authority grant. [Control ticket encoding](../src/node/parts/daemon/p005/body.rs) explicitly calls a ticket bootstrap, not authority. [Bootstrap selection](../src/peer/parts/bootstrap/p001/body.rs) compares capability/scope and a deny label. | Scope/connection selection, not a payload filter. An attenuation label or ticket is not cryptographic authorization. |
| Vat proxies | The [vat authority-graph fixture](../src/runtime/vat/parts/mod/p003/body.rs) records root/helper/proxy edges, including an `attenuates` label. It supplies no rule interpreter or transformed-value delivery path. | Descriptive scope/authority graph. The proposed rewriting proxy is a future payload-filter need, not an implemented consumer requiring one today. |
| Store views | Derive narrower namespaces/store roots from an acquired node-state filesystem authority; inner I/O retains that authority rather than reopening a host path. See [filesystem authority](node-state-filesystem-authority.md). | Filesystem scope restriction, not a Preserves payload transform. |
| Project-scoped artifact views | [Scoped name-view keys](../src/artifacts/parts/mod/p015/body.rs) separate scope/name identities; use inputs carry policy/capability refs. A project namespace alone grants no access. | Namespace/scope restriction; not an authority-bearing rewrite. |
| Peer subscription projections | [Projection admission](../src/peer/subscriber.rs) checks holder, projection kind, read-only operation, bounds, supplied authority/policy/resource/scope membership, and the declared redaction profile. | Scope/egress admission. This validator does not implement the named redaction transform or establish that redaction occurred. |
| World-branch authority | [Branch-authority mapping](../crates/molten-core/src/world_branch_authority/mapping/conversion.rs) retains Basalt's scope-widening and attenuation-not-narrower denials and the narrower-derivation obligation. | Scope derivation under existing stack policy, not a message filter. |

None of these current consumers justifies introducing a second authority format. In
particular, a proxy graph label, projection/redaction declaration, or scoped name is not
execution evidence for a filter. Existing admission and cleanup obligations remain with
their owning mechanisms; this review does not close their separate implementation work.

## Exact UCAN and Basalt cohort

`Cargo.toml` and locked Cargo metadata select Basalt revision
`89675cd4f585f837323c049e4a25f7b94c903038`. The resolved `ucan` and `ucan-core` packages
come from that same Git source's `vendor/ucan/`, not an ambient installation or the current
sibling checkout. The following paths refer to that
[immutable source](https://github.com/OnixResearch/basalt/tree/89675cd4f585f837323c049e4a25f7b94c903038).

### Compact UCAN profile

- `vendor/ucan/crates/ucan-core/src/token.rs::CaveatView` validates caveat domain, type,
  key, and nonempty opaque payload bytes. It does not give those bytes Preserves rewrite
  semantics or itself supply a bounded payload-filter language.
- `vendor/ucan/src/token.rs::VerifiedTypedCaveatPolicySet` recognizes `time/before/unix`
  and `rate/limit/window`. Other identifiers have no policy in that set.
  `CaveatPolicy::evaluate` returns `Satisfied` or `Rejected`, not a rewritten value.
  `AuthorizationRequest` carries a concrete resource/ability capability, not a Preserves
  message to transform. A consumer-defined policy could interpret custom bytes, but that
  would require its own admitted semantics and bounds; opaque bytes are not an adopted
  filter format.
- `VerifiedToken::authorize_request` uses `DenyUnknownCaveats`.
  `evaluate_capability_caveats` visits the matched capability's caveats in stored order
  and denies at the first missing or rejected policy. This boolean authorization process
  is not Synit's reverse-order rewriting chain and must not be described as one.
- Verification needs the signed compact token, verification keys/resolver, all required
  proof material, explicit verification time and traversal/size limits, and a revocation
  checker. `verify_child_is_linked_to_parent_audience`, `verify_temporal_attenuation`,
  `verify_capability_attenuation`, and `parent_caveats_are_preserved` retain proof linkage,
  narrower time/capability bounds, and inherited restrictions. Missing/stale proof material,
  invalid signatures/linkage, excessive traversal, and revoked references are denials,
  not authority inferred from a token identifier or successful transport.
- `src/ucan_semantics.rs` consumes supplied UCAN decision facts and admitted effective
  grants before Basalt enforcement. Its module explicitly does not parse tokens, traverse
  proofs, evaluate caveats, or own replay state. Its non-claims include revocation freshness,
  key-resolution truth, cryptographic correctness, replay-store durability, and complete
  application authorization. Molten cannot promote those supplied facts into stronger
  claims.

### Final UCAN 1.0 profile

The same pinned Basalt source also admits the final UCAN `1.0.0` cohort through
`src/ucan_profile.rs::{final_ucan_cohort,admit_final_profile}`. The compact APIs
above are not the complete pinned authority model.

- `vendor/ucan/crates/ucan-core/src/profile/v1_authorization.rs::Policy` parses a
  bounded Delegation policy and evaluates boolean predicates over IPLD Invocation
  arguments. Parsing and evaluation enforce statement, depth, and work limits;
  `Policy::satisfied` returns a boolean or typed error, not a rewritten payload.
- In that module, `compose_chain` requires the exact ordered, CID-bound proof
  sequence and checks principal/subject continuity, executor, command attenuation,
  time, supplied revocation facts, and each Delegation's argument policy through
  `check_policies`. These are existing bounded argument restrictions, not a
  newest-first Preserves rewrite chain.
- `src/ucan_v1_profile/shell.rs::verify_final_profile_enforcement` actually calls
  `authorize_invocation` with the Invocation envelope, key resolver, Delegation
  store, revocation observation, expected executor, explicit time/skew, and a
  replay gate. Basalt policy, mapping, and resource-owner preflight precede the
  final replay-port commit. Compact-token verification is not evidence that this
  separate route ran, nor proof of the supplied observations' truth or freshness.

The pinned stack therefore already has a bounded argument-policy language. That
does not establish an admitted mapping that rewrites delivered Preserves values.
The reviewed current consumers still need no new authority format; adopting a
final-profile mapping for a future consumer would require its own explicit
profile and schema decision. This review adds no such runtime integration.

## Decision and future guard rails

Keep the accepted [scoped allow/deny boundary](../.cairn/specs/runtime-spine/spec.md).
Do not add a Molten filter type, token construction, new authority codec, caveat evaluator,
or dependency for this package. No follow-up implementation change is needed for the
chosen **add nothing** outcome: the review deliverable is this explicit boundary and its
source-backed need inventory, not an abandoned filter implementation.

Revisit only when a current consumer requires a transformation that the existing scoped
admission cannot express. That consumer must name its input/output schemas, immutable
UCAN/Basalt cohort, bounds, and maintenance owner in a new reviewed change. Prefer an
admitted stack-owned form; this note does not authorize a Molten-owned form. Any proposal
for the latter must fix all of these semantics before implementation:

1. State chain storage direction and evaluation order. For a chain stored newest-first,
   traverse in reverse, oldest-to-newest, so the newest attenuation is applied last.
   Do not change UCAN's existing stored-order boolean evaluation to mimic that contract.
2. Unknown caveats reject every input; there is no permissive fallback or protected effect.
3. An unbound rewrite-template reference fails validation before evaluation or delivery.
4. Each result must remain inside the parent's admitted scope; rewriting cannot add a
   resource, recipient, operation, or privilege. Empty/denied results publish no payload.
5. Bound depth, entries, matching work, binding/output bytes, and diagnostic disclosure;
   retain deterministic positive, negative, ordering, and non-widening evidence.

## Repeatability and claim boundary

The durable contribution is a consumer-first authority decision: later filter proposals
must be reviewed against the inventory, exact dependency cohort, and guard rails rather
than adding an incompatible token format by analogy. Runtime-spine/authority maintainers
own this note; UCAN and Basalt retain their upstream authority mechanisms.

Repeat the source review when the consumers or pinned authority cohort change. Package
verification uses document/link review and native Cairn validation/gates, under the
repository's prose-only exemption. No runtime tests are added or claimed: this package
adds no filter, verifies no token at runtime, and makes no security, live-integration,
whole-system correctness, or release-readiness claim.
