## Context

The architecture scopes Raft to control-plane state: installed protocol registry, membership, capability/policy versions, receipt/replay ledgers, and explicit coordination services. Existing evidence-chain and ledger work provides local integrity, but no replicated state machine exists.

## Goals

- Define `raft-group-manifest-v1`, `raft-command-envelope-v1`, `raft-log-entry-v1`, `raft-commit-receipt-v1`, `raft-read-receipt-v1`, `raft-snapshot-v1`, and `control-registry-receipt-v1`.
- Implement a bounded control registry state machine with operations: install/update/remove protocol pointer, artifact name pointer, policy version pointer, capability version pointer, and receipt index pointer.
- Add Trellis-backed predicate receipts for append consistency, quorum commit, read-index freshness, client-session idempotency, and snapshot restore.
- Persist local log/snapshot/index state through Redb/chunk store with canonical refs.
- Keep normal actor messages, ordinary choreography traffic, gossip, docs, and blobs out of Raft.

## Non-Goals

- No production multi-node transport in the first slice; start with deterministic local/madsim-style channels.
- No global total order for actor messages.
- No arbitrary key-value store semantics beyond explicit control-plane schemas.
- No lease reads unless a manifest and policy explicitly admit them.

## Records

```preserves
<raft-group-manifest-v1 "molten.raft.group-manifest.v1"
  <group-id "raft:control">
  <members [<node-ref> ...]>
  <state-machine "control-registry-v1">
  <command-schemas ["install-protocol" "set-policy-version" ...]>
  <read-mode "read-index">
  <snapshot-policy <snapshot-policy-ref>>
  <policy [<policy-ref> ...]>
  <resource [<resource-ref> ...]>
  <checks [<check "control-plane-only" "pass"> ...]>>
```

```preserves
<raft-command-envelope-v1 "molten.raft.command-envelope.v1"
  <group <group-ref>>
  <client-session <session-id>>
  <sequence 42>
  <command <control-registry-command-v1 ...>>
  <authority [<authority-context-ref> ...]>
  <policy [<policy-ref> ...]>
  <evidence [<receipt-ref> ...]>
  <checks [<check "schema-admitted" "pass"> ...]>>
```

## State Machine

The first state machine is a deterministic map of registry namespaces to canonical refs:

- `protocol/<name> -> protocol-install-receipt-ref`
- `artifact-name/<name> -> artifact-ref`
- `policy/<scope> -> policy-ref`
- `capability/<scope> -> authority-policy-ref`
- `receipt-index/<scope> -> chain/checkpoint/ref`

Apply returns a canonical state delta and a `control-registry-receipt-v1`. Reads use read-index by default and bind the committed index/term.

## Recovery

On restart, the node loads the latest admitted snapshot, verifies snapshot content refs, replays committed log entries, verifies client-session dedup state, and emits recovery receipts before serving reads/writes.
