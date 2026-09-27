# Connectivity Versus Membership

This article explains why an established connection, an authenticated identity, and an admitted member are different facts in Molten. It assumes familiarity with the [membership runtime](../../fabric-membership-placement.md) and the [cryptographic identity boundary](../../fabric-cryptographic-identity.md). It is explanatory material for the [Technical companion](../README.md), not an additional admission policy.

## Three questions, three evidence domains

Connectivity answers whether a transport interaction is possible under the conditions of an observation. Cryptographic verification answers whether a supplied public key verifies a particular canonical payload in its declared domain and currentness context. Membership answers which nodes occur in a particular source-scoped view. None is a substitute for the others.

The identity documentation explicitly excludes membership and capability authority from signature-verification claims. It also separates transport keys from federation, delegation, evidence, and authority purposes. Consequently, even a correctly authenticated transport peer cannot acquire membership merely by maintaining a connection. Conversely, a temporarily unreachable member does not disappear from an immutable membership snapshot. These are authority boundaries, not recommendations to distrust all transport information: transport observations may inform a separately admitted failure observation without becoming the membership source.

The [pure membership types](../../../crates/molten-core/src/fabric_membership/mod.rs) make this separation inspectable. `MembershipMember` contains `node_id`, `descriptor_ref`, and `eligibility_ref`; it is not a socket or endpoint handle. `MembershipView` carries a source-profile reference, source evidence, authority and eligibility-policy references, an epoch, and freshness bounds. The profile records provider kind, authority strength, and authority scope. These fields identify the context in which a statement about membership is meaningful.

## Admission is a bounded consistency check

`validate_membership_view` accepts the profile, view, descriptors, caller-supplied current ticks, and required compatibility reference. The pure function neither contacts a node nor consults a policy service. It checks schemas and reference shapes, requires a nonzero epoch, verifies the source-profile match, rejects stale or future-dated observations, and requires strictly ordered member identifiers.

Descriptors are validated separately, including compatibility, label ordering, runtime-feature ordering, and evidence-reference shape. Every member must have a matching descriptor reference, and the descriptor and member sets must correspond. Supplying an extra descriptor for a recently connected node therefore does not silently add it to the view. The set relationship is part of admission, not inferred from network reachability.

These checks establish consistency of supplied data under the contract. A well-shaped `eligibility_ref` is not the execution of the referenced policy, and a well-shaped `authority_ref` is not cryptographic verification of an authority service. The governing document preserves that distinction by explicitly denying capability authority and global membership truth.

The shell's [`observe_provider`](../../../src/fabric_membership/shell.rs) adds an integration boundary. It obtains a provider snapshot, checks that the provider's reported kind matches its profile, and routes membership and failure data through canonical admission. The resulting admitted snapshot is still source-scoped. Canonical projection provides stable evidence about admitted inputs; it does not strengthen the provider's authority.

## Illustrative failure: the reachable stranger

Suppose a service is planning replicas from view 12, which contains `node-a` and `node-b`. A transport connection from `node-c` succeeds, and the peer presents a valid transport identity. An operator also receives a descriptor advertising available memory and the expected runtime feature.

There are three tempting but invalid shortcuts:

1. Appending the descriptor while keeping view 12 unchanged breaks the member–descriptor set relationship.
2. Treating the signature as an eligibility record confuses purpose-scoped identity with membership policy.
3. Selecting `node-c` directly bypasses the planner's iteration over admitted members.

A separate source decision is needed before a later view can include `node-c`. For the inspected policy-managed adapter, replacing a snapshot preserves provider kind and profile reference and advances the view epoch. That adapter mechanism is not itself a quorum protocol. After admission, placement can consider the new member, but a successful advisory plan still does not start its role.

Now reverse the situation: `node-b` loses connectivity. View 12 still contains it. A fresh `Suspected` observation can influence a placement policy, but does not remove membership, revoke authority, or transfer its current assignment. Keeping these distinctions visible prevents “disconnected” from becoming an undocumented ownership transition.

## Verification and review guidance

For review, trace each fact to its producer: transport for connection observations, identity adapters for signature outcomes, membership providers for snapshots, and assignment authority for role effects. Inspect the existing membership tests for stale-view, ordering, compatibility, and unknown-observation-subject rejection. Suggested focused checks include an extra descriptor without a member and a valid member whose transport is absent; these should be reasoned about as different cases, not merged into one health boolean.

No runtime checks are reported here. This documentation work inspected source and existing test assertions; executing the suggested scenarios remains separate verification evidence.

## Limits and non-claims

The model does not provide global discovery, establish process death, or prove a live authority backend. The source profile's declared strength remains an input whose real enforcement belongs to its integration. Neither successful connectivity nor canonical membership evidence proves service correctness or production readiness.

## Sources

- [Fabric membership and placement runtime](../../fabric-membership-placement.md)
- [Fabric cryptographic identity adapters](../../fabric-cryptographic-identity.md)
- [Membership types and admission](../../../crates/molten-core/src/fabric_membership/mod.rs)
- [Provider observation shell](../../../src/fabric_membership/shell.rs)
- [Provider adapter mechanisms](../../../src/fabric_membership/adapters.rs)
- [Membership regression cases](../../../crates/molten-core/src/fabric_membership/tests.rs)
