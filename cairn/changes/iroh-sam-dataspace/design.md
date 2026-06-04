## Context

The runtime architecture says ordinary actor traffic should use a SAM/Syndicate-inspired dataspace and not Raft. The remote architecture says Iroh is the first transport substrate, but transport identity is not application authority. This change joins those two commitments with a narrow evidence-bearing remote dataspace rail.

## Goals

- Preserve SAM-style public semantics: actors/entities/facets maintain assertions, retract them, observe patterns, and send messages in turn boundaries.
- Represent every remote dataspace action as canonical Preserves before transport, admission, storage, replay, or receipt hashing.
- Use Iroh gossip for envelope-sized records and Iroh blobs/chunks for large immutable payload bytes referenced by the envelope.
- Keep transport authority separate from capability authority: an Iroh endpoint id or gossip membership does not grant actor authority by itself.
- Support deterministic harness and repro playback through recorded delivery logs.

## Non-Goals

- Do not make Molten compatible with Syndicate wire protocols.
- Do not make Iroh gossip topics into application semantics or authority.
- Do not use Raft for ordinary actor messages, observe traffic, or assertion propagation.
- Do not deliver large payload bytes to actors until declared content refs verify.
- Do not make unrecorded live network timing part of deterministic replay.

## Canonical Records

### Remote dataspace envelope

```preserves
<remote-dataspace-envelope-v1
  "molten.remote-dataspace.envelope.v1"
  <from-peer "peer:a">
  <from-actor "producer">
  <to-peer "peer:b">
  <topic "services">
  <operation "assert">
  <payload <service-ready "db">>
  <content-refs []>
  <capability-refs ["blake3:..."]>
  <evidence-refs ["blake3:..."]>>
```

Operations are initially `message`, `assert`, `retract`, and `observe`. The payload is a canonical Preserves value. Exact-value observe patterns are enough for the first slice; richer Preserves pattern subsets can be added behind explicit schema refs.

### Transport receipt

```preserves
<remote-dataspace-transport-receipt-v1
  "molten.remote-dataspace.transport-receipt.v1"
  <operation "publish">
  <decision "pass">
  <transport "iroh-gossip">
  <envelope "blake3:...">
  <node "peer:a">
  <from-peer "peer:a">
  <to-peer "peer:b">
  <topic "services">
  <content-refs []>
  <diagnostics []>
  <checks [
    <check "canonical-envelope-ref" "pass">
    <check "content-refs-verified" "pass">
    <check "transport-is-not-authority" "pass">]>>
```

Deny receipts use the same record with `decision "deny"` and diagnostics. A pass receipt is not enough for delivery; it only proves transport publication or fetch. Delivery still requires peer bootstrap, policy, capability, and resource admission evidence.

## Flow

1. Actor runtime emits a pending SAM action: message, assert, retract, or observe.
2. Adapter canonicalizes the action into a remote dataspace envelope.
3. Preflight validates envelope hash, schema, peer/topic strings, content refs, and explicit authority/evidence refs.
4. Iroh gossip publishes the envelope bytes to an authorized topic.
5. Receiver fetches bytes, recomputes the envelope ref, validates declared content refs via Iroh blobs/chunks, and emits transport receipt evidence.
6. Receiver runs policy/capability/resource admission.
7. If admitted, the envelope is applied as a local runtime turn: assertion commit/retract, observer registration, message delivery, and observer notifications.
8. Replay uses recorded envelope bytes, content bytes/refs, transport receipts, and admission decisions instead of live network timing.

## First implementation slice

The first drain does not need a live multi-process network. It should add the canonical records plus a deterministic `iroh-local-gossip` adapter that stores canonical envelope bytes under a local Iroh-shaped blob/gossip root. This follows the existing local Iroh exchange pattern used for repro bundles, chain segments, and chunks while establishing the precise Preserves rail and receipt shape.

## Later slices

- Replace/augment `iroh-local-gossip` with real `iroh-gossip` publish/subscribe integration.
- Bind peer bootstrap agreements to topic joins and delivery admission.
- Add harness fixtures for two deterministic peers and recorded transport logs.
- Add gate receipts that bind envelope refs, transport receipts, peer bootstrap receipts, authority receipts, resource receipts, and actor turn-journal chain refs.
- Add richer Preserves pattern subsets for remote observe traffic after local predicate receipts exist.
