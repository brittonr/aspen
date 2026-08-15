## Context

Molten will have multiple coordination modes:

- local dataspace assertions and subscriptions for reactive actor routing,
- choreography sessions for typed multi-party protocol flow,
- Iroh gossip/blobs/docs for remote transport and content movement,
- policy/evidence gates for admission and auditability,
- and a small set of control-plane records that require linearizable agreement.

Raft belongs only in the last category. It should not become the default path for every actor message. The consensus layer should replicate decisions that need strong ordering and durability, then publish resulting envelopes or facts back into the rest of the runtime.

Trellis provides verified Raft building blocks that fit this layer: election, quorum, log matching, append-entries build/check, commit advancement, read-index, lease-read, config-change, joint consensus, snapshots, client sessions, linearizability, and state-machine safety primitives. Molten should wrap those primitives in protocol artifacts, command envelopes, transport adapters, stores, policy checks, and receipts.

## Goals

- Define a strongly consistent Molten control-plane boundary.
- Use Trellis Raft primitives as the normative bounded specification/admission layer.
- Represent Raft commands, snapshots, reads, and membership changes as canonical envelope-backed artifacts.
- Keep replicated state machines deterministic and free of direct filesystem, network, process, clock, scripting, or adapter side effects.
- Provide idempotent client sessions for exactly-once command admission at the replicated state-machine boundary.
- Support linearizable reads through Trellis read-index or explicitly admitted lease-read conditions.
- Bind log entries, snapshots, membership changes, and policy decisions to Cairn receipts and evidence references.
- Keep Raft message transport independent from any specific Iroh or dataspace implementation.

## Non-Goals

- Do not require Raft for normal actor messages, ordinary choreography steps, gossip fanout, blob transfer, or local-only dataspace facts.
- Do not implement Byzantine fault tolerance in the first consensus surface.
- Do not treat Trellis admission predicates as proof that application commands are semantically correct.
- Do not allow replicated commands to perform side effects during pure state-machine application.
- Do not expose clock or lease assumptions without explicit policy, configuration, and evidence.
- Do not couple the consensus state machine to Redb, Iroh, Syndicate, Wasmtime, or Steel internals.

## Architecture

```text
Raft group manifest
  group id, members, timeouts, read mode, snapshot policy, state-machine kind, policy refs
        |
        v
Molten consensus compiler/admission
  canonical ids and config validation
  Basalt/Nickel/Steel policy gates for install and membership
        |
        v
Trellis Raft layer
  election/quorum/log/append/commit/read/membership/snapshot/session predicates
        |
        v
Replicated command log
  canonical Molten command envelopes, hashes, sessions, sequence numbers, receipts
        |
        v
Deterministic state machine
  pure apply/read/snapshot/restore over explicit state values
        |
        v
Adapters
  envelope transport over dataspace/Iroh, durable log/snapshot store, observability, receipts
```

## Consensus Scope

Initial Raft-backed control-plane records should include:

- installed protocol artifacts and choreography manifest registry,
- node and Raft-group membership records,
- capability/grant or policy-bundle version records,
- durable receipt indexes and replay/session sequence ledgers,
- linearizable runtime configuration and admission policy changes,
- control-plane leases or locks that explicitly require linearizable ownership.

The following stay outside Raft by default:

- normal actor-to-actor messages,
- per-step choreography protocol messages,
- local dataspace assertions,
- gossip topics,
- blob content transfer,
- Wasmtime hostcall traffic except when the hostcall mutates a Raft-backed control-plane resource.

## Raft Group Manifest

A Raft group manifest should contain:

- `group_id`: stable identity for the consensus group,
- `members`: initial voter/learner node ids and transport identities,
- `state_machine`: declared state-machine kind and schema version,
- `command_kinds`: allowed command families and payload schema ids,
- `read_mode`: read-index by default; lease-read only with explicit timing assumptions,
- `timeouts`: election, heartbeat, snapshot, retry, and lease parameters,
- `snapshot_policy`: thresholds, chunking, content-reference policy, and integrity hash algorithm,
- `persistence_policy`: durable log/snapshot requirements,
- `policy_refs`: Nickel/Basalt/Cairn contract ids and required capabilities.

The manifest is installed only after static validation, policy admission, and receipt emission.

## Command Entries

Every replicated command entry should include:

- `group_id`, `term`, `index`, and command kind,
- canonical command body as Preserves or a content reference,
- Blake3 command hash over canonical bytes,
- client id, client session id, and sequence number,
- required capability/evidence references,
- admission receipt reference,
- optional resulting runtime envelope ids emitted after commit.

Command application must be deterministic and must return a new state plus declared output records. Any real side effects are performed by adapters after the entry is committed, admitted, and recorded.

## Reads

Linearizable reads should use read-index by default. Lease reads may be admitted only when the Raft group manifest explicitly declares lease assumptions and policy gates admit those assumptions for the deployment. Stale local reads may exist for diagnostics, but they must be labeled as non-linearizable and must not satisfy requirements that demand a linearizable control-plane decision.

## Membership Changes

Membership updates should be represented as replicated commands and use Trellis config-change and joint-consensus primitives where applicable. The policy layer must authorize membership changes before proposal, and the consensus layer must reject changes that violate bounded membership, quorum, learner promotion, or leadership-transfer rules.

## Snapshots and Recovery

Snapshots should be content-addressed and integrity checked. A snapshot artifact should bind:

- group id, last included term/index, and membership/config state,
- state-machine schema version,
- canonical snapshot hash and optional blob/content references,
- install/restore receipt references,
- Trellis snapshot-integrity predicate evidence where applicable.

Recovery replays durable log entries after the latest admitted snapshot and must reconstruct the same deterministic state-machine value.

## Policy and Evidence

Consensus operations cross trust boundaries and must emit evidence:

- group installation,
- membership change proposal and commit,
- command proposal admission,
- append/commit advancement where exposed to operators,
- linearizable read admission,
- snapshot creation/install/restore,
- client-session replay rejection.

Nickel contracts should cover static group configuration, command schemas, snapshot policy, bounds, and allowed command families. Steel contracts may appear only for explicitly reviewed dynamic predicates. Basalt enforces capability-bearing requests. Cairn validates receipts before they are used as evidence. Trellis supplies bounded predicates for Raft-specific safety/admission surfaces.

## Transport

Raft messages should be represented as Molten envelopes with a `raft` subject family or equivalent protocol-message body. Transport may be local dataspace, Iroh gossip, direct QUIC streams, or test harness channels. Consensus semantics must depend only on the envelope content, group state, and Trellis admission rules, not on transport identity alone.

## Open Questions

- Which control-plane state machine should be implemented first: protocol registry, receipt index, or capability/policy version registry?
- Should the first transport use local in-process channels before Iroh integration?
- How much of Trellis Raft should be called directly in runtime code versus wrapped in Molten admission functions?
- Should snapshots be canonical Preserves values, Redb-native exports, or both?
- What test harness should model node crashes and partitions before a full deterministic simulator exists?
