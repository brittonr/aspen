# Design: chain-hashed evidence ledger

## Scope

Chain hashing is an evidence-continuity mechanism. It is not a token ledger, proof-of-work chain, proof-of-stake chain, fork-choice protocol, or total ordering layer for all Molten traffic.

Molten should maintain scoped hash chains where continuity matters:

- local evidence ledger append history;
- gate/report/repro/signed receipt lifecycle history;
- actor, session, or vat turn journals where replay/debugging authority permits;
- artifact, chunk manifest, and catalog publication lineages;
- control-plane checkpoints agreed through Trellis/Raft.

Each scope can define its own writer set, epoch, retention policy, and checkpoint policy. A chain head is evidence about a scoped history, not ambient authority by itself.

## Link shape

A chain link should be a canonical Preserves record. The exact DTO can evolve, but the first schema should include the following fields:

```preserves
<chain-link-v1
  "molten.evidence.chain-link.v1"
  <chain <scope "evidence-ledger"> <id "node-or-ledger-id"> <epoch "epoch-ref">>
  <seq 42>
  <prev <some "blake3:previous-link-ref">>
  <payload <kind "gate-receipt"> <ref "blake3:payload-ref"> <schema "molten.harness.gate-receipt.v1">>
  <context [<ref "policy" "blake3:..."> <ref "capability" "blake3:...">]>
  <producer <id "node:..."> <key "blake3:key-ref">>
  <trellis <predicate "molten.chain.append_valid.v1"> <input "blake3:input-ref"> <decision "pass">>
  <checks [<check "previous-link-binding" "pass"> <check "payload-ref-binding" "pass">]>>
```

The `chain-link` canonical ref is the Blake3 hash of the canonical Preserves link bytes. Payload refs identify existing canonical artifacts and MUST NOT be re-encoded or mutated by linking. Context refs bind the policy/capability/schema/budget/checkpoint environment that made the append admissible.

Genesis links use `<prev <none>>` and sequence `0`. Non-genesis links use `<prev <some ...>>` and sequence `previous.seq + 1` within the same chain scope/id/epoch.

## Append rules

A link append is valid only if:

1. the link has the expected schema and canonical encoding profile;
2. the payload ref hashes to an available payload of the declared kind/schema;
3. the context refs required by that payload kind are present and match embedded evidence where applicable;
4. genesis links have `seq = 0` and no previous link;
5. non-genesis links name an available previous link in the same chain scope/id/epoch;
6. sequence numbers are monotonic with no gaps for the verified segment;
7. the producer is authorized for the chain scope and epoch;
8. the append policy for the scope either denies forks or emits explicit fork evidence;
9. the link descends from an accepted anchor when a gate profile requires anchored evidence.

Because a content-addressed store cannot prevent someone from creating two children for the same previous link, fork handling is a verification and policy problem. Production evidence profiles should reject unexpected forks. Diagnostic profiles may preserve fork evidence for investigation.

## Receipts

Appending a link should emit a canonical append receipt:

```preserves
<chain-append-receipt-v1
  "molten.evidence.chain-append-receipt.v1"
  <decision "pass">
  <link "blake3:link-ref">
  <head-before "blake3:old-head-or-none">
  <head-after "blake3:new-head">
  <checks [...]>>
```

Verifying a segment should emit a canonical verify receipt naming the start anchor, expected or discovered head, verified link refs, payload refs, fork/gap/stale-head diagnostics, and Trellis predicate receipt refs.

Signed receipt envelopes can wrap append/verify receipts. Conversely, a chain link can name a signed receipt envelope as its payload. Signing a link or append receipt attributes the append; hashing the link provides continuity.

## Trellis integration

Trellis should provide bounded predicates for chain validity, initially over finite segments:

- `genesis_valid`: genesis shape and anchor context;
- `append_valid`: previous-link binding, payload/context binding, and sequence increment;
- `segment_no_gap`: contiguous sequence range from anchor to head;
- `segment_no_fork`: at most one accepted child for each parent under a no-fork policy;
- `descends_from_anchor`: head reaches a trusted anchor or checkpoint;
- `checkpoint_covers_range`: a Raft/Trellis checkpoint commits to a chain head and verified range.

These predicates do not implement blockchain consensus. They provide machine-checkable append and continuity evidence that Molten gates can reference.

## Control-plane checkpoints

For scopes that need agreement, Molten can checkpoint a chain head through the existing Trellis/Raft control-plane layer. The checkpoint command should be a canonical Molten command envelope naming:

- chain scope/id/epoch;
- prior accepted checkpoint ref;
- new head ref;
- verified range or segment receipt ref;
- policy and membership refs.

Raft agreement is used only for selected control-plane facts such as accepted evidence heads, protocol registry lineages, capability/policy version chains, and receipt index checkpoints. Ordinary actor messages MUST NOT depend on a global chain head.

## Storage and indexes

The local evidence ledger should store chain links as immutable canonical artifacts keyed by link ref. Indexes are derived and rebuildable:

- chain scope/id/epoch to link refs;
- parent link to children;
- sequence to link ref;
- payload ref to link refs;
- current heads;
- anchors and checkpoints;
- fork and gap diagnostics.

Indexes are hints. Canonical link bytes and payload bytes are authoritative.

## Replay and runtime journals

Actor/turn chains should be opt-in per evidence profile. A turn link can commit to the previous turn link, input event ref, admission decision refs, effect-log refs, before/after state refs, and emitted trace refs. This improves first-divergence debugging but must not serialize unrelated actors behind one global head.

Replay should be able to verify that a report's turn chain descends from the expected start state and that each turn link's payload/context refs match the report's canonical observations.

## Iroh chain exchange

Remote transport may carry scoped chain segments, but it does not make them trusted. An exchanged `<chain-segment-bundle-v1 "molten.evidence.chain-segment-bundle.v1" ...>` should include chain links, payload artifacts, predicate receipts, fork evidence, anchors, verify receipts, and checkpoints for one `(scope, id, epoch)` range. Fetchers must recompute canonical refs, validate no-gap continuity, require predicate receipts named by the verify receipt, validate checkpoint range bindings when checkpoints are present, and apply the local fork policy before importing artifacts into the evidence ledger. Production profiles reject fetched fork diagnostics; diagnostic profiles may retain explicit fork evidence.

## Confidentiality

Links should avoid embedding sensitive payload bytes. They should name refs, redacted refs, encrypted refs, or protected commitments according to the existing confidentiality policy. If a link's metadata would reveal sensitive information, the scope policy should require redacted labels or private/encrypted chain segments before export.

## Non-goals

- No native currency, token supply, balances, gas, mining, staking, slashing, or fee market.
- No permissionless mempool or public transaction acceptance protocol.
- No global total order for all messages.
- No fork-choice rule beyond scoped evidence policies and optional control-plane checkpoints.
- No replacement for Preserves canonical refs, signed receipts, or Trellis/Raft control-plane agreement.
