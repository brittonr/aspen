# Change: chain-hashed-evidence-ledger

## Why

Molten already content-addresses receipts, reports, bundles, chunks, and ledger artifacts, and signed receipts can name parent receipts. That proves individual bytes and explicit dependencies, but it does not yet provide a tamper-evident continuity story for evidence streams: an operator can omit a receipt, reorder local ledger events, fork a publication history, or present an old head unless the surrounding workflow adds ad hoc checks.

Molten should add chain hashing as scoped evidence continuity, not as a cryptocurrency or global blockchain. The goal is to make receipt histories, actor turn journals, artifact publication lineages, and control-plane checkpoints auditable with canonical Preserves links and Trellis-backed append predicates.

## What

- Define canonical `<chain-link-v1 "molten.evidence.chain-link.v1" ...>` records whose Blake3 refs commit to chain scope, sequence, previous link ref, payload ref, context refs, producer identity, and verification checks.
- Use many scoped hash chains rather than one global chain: evidence ledger chains, actor/session turn-journal chains, artifact/chunk publication chains, and Raft/Trellis control-plane checkpoint chains.
- Add append and verification receipts for chain links, including gap, stale-head, and fork diagnostics.
- Integrate with signed receipts so signatures can bind chain links and chain links can name signed receipt refs without changing the subject receipt hash.
- Use Trellis predicates for bounded append validity: genesis shape, previous-link binding, sequence monotonicity, payload/context ref binding, no-gap continuity, no-fork policy for a scoped writer/epoch, and descent from trusted anchors/checkpoints.
- Allow production gate profiles to require selected pass evidence to descend from an accepted chain head or checkpoint.

## Impact

This strengthens Molten's audit and replay story without turning the runtime into a blockchain. Chain hashing provides tamper-evident continuity at evidence boundaries and durable journals, while ordinary actor messages remain dataspace traffic and control-plane agreement remains scoped to Trellis/Raft checkpoints.
