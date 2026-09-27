# ALPN Routing and Protocol Identity

ALPN identifies a transport protocol route, not the authority of a caller or the compatibility of an application transaction. This article examines Molten's reviewed Iroh registry and fabric protocol descriptors as related but distinct identity surfaces. Read the [ALPN registry contract](../../iroh-alpn-routing-registry.md) and [fabric transport contract](../../fabric-transport-session-runtime.md) first. The distinction matters when evaluating router replacement, handler ownership, or endpoint handoff evidence.

## A route has more identity than its wire label

`IrohAlpnRegistryEntry` records a symbolic name, ALPN string, owner namespace, handler profile, lifecycle state, supported schema/profile names, limit references, required evidence references, and receipt schema references. Its `entry_ref` is computed from the canonical Preserves value. The [registry construction](../../../src/node/parts/iroh/p006/body.rs) therefore binds a structured statement about a route, not merely the bytes negotiated on a connection.

The registry validator tracks ALPN and symbolic-name uniqueness separately. Duplicate ALPN entries and duplicate symbols each add diagnostics. Valid-looking entries can still appear in the returned collection when the aggregate validation decision is `deny`; callers should inspect the decision rather than treating a nonempty entry list as approval. Canonical encoding supplies stable content identity for the statement. It does not make the statement an authority grant.

The inspected default registry input includes `molten/node-control/1`, with explicit owner and handler-profile constants, a limit reference, and receipt-schema references. This observation is not an inventory of every protocol an extension may use. The extension-facing fabric registry is a separate surface with a different descriptor and admission function.

## Router mutation is a guarded state change

The [router evaluator](../../../src/node/parts/iroh/p001/body.rs) validates input formatting, generation range, evidence-reference collections, and membership in the reviewed ALPN registry. For a known ALPN it compares the requested owner namespace and handler profile to the reviewed entry, and checks that lifecycle is active. Ordinary mutation additionally requires nonempty authority, policy, resource, and evidence-reference groups.

Installation rejects an existing route. Replacement requires an existing route, exact prior generation, a strictly advancing new generation, and shutdown evidence. Removal also requires the exact prior generation and shutdown evidence. Denied mutations preserve the handler registry. An unsupported-ALPN observation has its own path and preserves the registry rather than manufacturing a handler for the unknown protocol.

These are local evaluator rules. A syntactically valid shutdown evidence reference is not, by itself, a demonstration that a remote machine terminated all work. The surrounding shell and evidence producers determine what observation the reference actually binds. The [governing registry document](../../iroh-alpn-routing-registry.md) describes admission before live router mutation; the evaluator should not be confused with the live Iroh router itself.

## Fabric registration adds service and generation scope

The fabric `ProtocolDescriptor` binds protocol/version, ALPN, extension/service identity, generation, framing, requested capabilities, listener limits, cleanup policy, registration authority, and profile. [Registration and transfer](../../../crates/molten-core/src/fabric_transport/transition.rs) reject duplicate ALPN ownership and conflicting protocol/version identity. A transfer retains protocol/version, extension, service, and registration authority while advancing generation and supplying cleanup evidence.

This identity structure is stronger than checking that two peers present the same ALPN string, but it remains a transport contract. Matching ALPN does not establish application-level schema negotiation, semantic compatibility, durable processing, or permission to execute node-control operations. A descriptor's framing profile also describes admitted bounds; it is not a claim about Rust object layout or an invitation to serialize native memory.

The two registries should not be collapsed into an invented universal API. The node router checks a reviewed ALPN entry and handler metadata. The fabric transition core manages extension-owned descriptors and scoped sessions. Both defend ownership and evidence boundaries, but their fields and state machines are not interchangeable.

## Worked reasoning: a stale replacement

Imagine an illustrative node route currently installed at generation 8. An operator prepares a replacement at generation 9 using the correct ALPN, owner namespace, and handler profile, but the operation declares prior generation 7. The replacement denies even if every reference has valid syntax: the prior-generation comparison protects the currently installed owner from a stale mutation.

Changing only the prior generation to 8 is still insufficient without shutdown evidence. Supplying that evidence permits evaluation of the replacement under the remaining gates; it does not authorize a client to send privileged commands. A subsequent client connection negotiating that ALPN establishes routing compatibility at the transport selection boundary, not application authority.

For a fabric descriptor, an analogous generation advance must also preserve the bound extension, service, protocol/version, and registration-authority identity. Reusing an ALPN while silently changing those ownership fields is not the inspected transfer operation.

## Verification and limits

Suggested review should follow an entry from canonical construction through router admission and receipt binding, then examine stale replacement, duplicate registration, wrong owner, wrong profile, and unknown-ALPN paths. Existing [router tests](../../../src/node/parts/iroh/tests/m000/p000/body.rs) exercise installation, replacement, removal, unsupported ALPN, and stale mutation. These tests were inspected as source, not executed for this article.

An ALPN receipt is routing evidence only. It cannot stand in for provenance, source trust, retention clearance, resource authority, execution permission, or the peer admission relation described in the [peer session document](../../peer-session-transition-relation.md). No production interoperability or network availability qualification follows from a local evaluator pass.

## Sources

- [Technical companion](../README.md)
- [Iroh ALPN routing registry](../../iroh-alpn-routing-registry.md)
- [Fabric transport session runtime](../../fabric-transport-session-runtime.md)
- [Peer session transition relation](../../peer-session-transition-relation.md)
- [Canonical registry entries and validation](../../../src/node/parts/iroh/p006/body.rs)
- [Router admission and mutation](../../../src/node/parts/iroh/p001/body.rs)
- [Fabric registration and transfer](../../../crates/molten-core/src/fabric_transport/transition.rs)
- [Router behavioral tests](../../../src/node/parts/iroh/tests/m000/p000/body.rs)
