# Design: eventual propagation surfaces

## Scope

This change defines eventual propagation semantics for surfaces carried by gossip, docs, remote dataspace envelopes, and federation. It does not add consensus, global ordering, or implicit trust.

## Surface manifest

`eventual-surface-manifest-v1` declares:

- surface id and scope,
- carrier class such as local-gossip, live-gossip, docs, or federation inventory,
- payload type and canonical envelope/schema refs,
- idempotency key and duplicate policy,
- merge or conflict-resolution law,
- retraction/tombstone/expiry law,
- anti-entropy or pull-sync policy,
- replay evidence requirement,
- authority and resource boundaries.

A surface may claim eventual convergence only when its merge law is deterministic, commutative where needed, idempotent for duplicate delivery, and explicit about tombstones or retractions.

## Evidence

Propagation receipts record publish, deliver, observe, merge, deny, anti-entropy query, missing-set calculation, fetch, import, and replay-gate decisions. Live timing observations are diagnostic unless included in a recorded delivery log, snapshot, or anti-entropy receipt.

## Relationship to consensus

Eventual surfaces may carry messages that request control-plane operations, but they do not decide those operations. Consensus state remains owned by Raft/control-plane receipts. Eventual propagation cannot satisfy authority, policy, resource, provenance, source-gate, retention, execution, or linearizable-read requirements.

## Functional core

The core validates surface manifests, merge-law declarations, idempotency/retraction inputs, and convergence claims over in-memory values. Shells own transport IO, docs access, federation fetches, and receipt persistence.

## Non-goals

- No global total order for actor messages.
- No global dataspace or global Raft.
- No convergence claim without a reviewed merge/retraction law.
- No deterministic pass evidence from unrecorded live gossip timing.
