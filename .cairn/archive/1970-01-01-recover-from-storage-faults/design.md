# Design: Recover from storage faults

## Context

Molten has a pure fabric durability core and Redb-backed shell adapters. The current outcome vocabulary merges requested durability with an observed successful adapter call.

Storage faults need a separate recovery protocol. After a synchronization error, the shell cannot safely infer whether the mutation reached persistent media. After local repair, the consensus core cannot safely infer that committed history remains complete.

## Success Contract

A mutation receives a durable acknowledgement only after the selected adapter reports the required observation. Any synchronization or commit uncertainty poisons that adapter instance.

A poisoned Raft node cannot serve durable reads, acknowledge writes, vote, lead, or rejoin through local repair alone. Recovery needs peer-validated protocol facts.

## Decisions

### Decision: The core classifies explicit observations

The pure core receives a requested durability level and one typed shell observation. Observations distinguish `NotApplied`, `AppliedProcessVisible`, `SyncObserved`, `OutcomeUnknown`, and `CorruptOrInconsistent`.

The core never receives Redb errors, file handles, or operating-system error values.

### Decision: The shell poisons uncertain stores

Any commit, synchronization, repair callback, decode, checksum, or invariant failure moves the active adapter into quarantine. The shell rejects normal operations until an explicit reopen and reconciliation workflow completes.

Retrying the same synchronization call cannot clear quarantine.

### Decision: Snapshot publication is ordered

The shell stages the payload, synchronizes it, publishes it without replacement, synchronizes the parent, and then commits metadata.

A parent synchronization failure produces committed durability unknown. Startup reconciliation checks the destination and its BLAKE3 identity before metadata repair.

### Decision: Consensus repair uses protocol facts

The core models local entries as present, missing, or faulty. Repair gathers bounded peer responses keyed by term and index.

A committed entry is never truncated. An entry is discarded only when a quorum establishes that it was not committed. Unknown commitment produces a wait decision.

A snapshot repair needs matching content identity at one committed index. Node-specific term and vote state cannot be copied from a peer.

### Decision: Recovery progress is participant-scoped

The pure core distinguishes pass, fail, not-evaluated, and incomplete recovery results. Each result binds the participant, required item, local sufficiency facts, admitted peer set, disruptive faults, observation completeness, and virtual progress horizon.

If admitted local durable state is sufficient, the core does not plan remote repair. If an admitted peer has a required committed item, a stable recovery window requires repair or a typed failure. The core reports global unavailability only after complete admitted peer observations show that every permitted source lacks the exact item.

### Decision: Repaired nodes rejoin through admission

A local Redb repair callback, checksum failure, or uncertain persistent-state event removes the node from voting readiness.

The shell completes storage reopen, protocol reconciliation, snapshot or log validation, and current-membership checks before the core can admit voting again.

The live Raft shell executes canonical `RecoveryQuery`/`RecoveryReport` exchanges even when no leader exists. Recovering participants answer queries but cannot campaign, grant votes, acknowledge normal replication or quorum reads, return client results, or publish snapshots. Each retry round is fenced by its own timer reference; group, generation, membership, configuration and fencing epochs remain the existing transport admission boundary.

The reporting set must intersect every commit quorum without counting the suspect participant itself. All held indices are considered, including peer-held entries beyond reported commitment; a missing local slot cannot establish absence when a peer still holds that entry. An available committed source may repair data before this reporting set is complete, without granting voting or truncation authority. Initial reports discover the term frontier. Before absence-based truncation or admission, the shell persists a strictly higher-term recovery barrier and gathers new reports at that term. Recovering peers join the negotiated barrier rather than creating competing term increments. This fences delayed old-term messages whose acknowledgements might otherwise depend on the lost copy. Higher-term traffic invalidates those observations. Partial pages survive timer retries; progress remains scoped to the declared stable recovery window.

A peer observation includes its durable commit index, complete held log (not only committed entries), and optional snapshot. Messages carry at most 128 contiguous entries; the receiver assembles at most 4,096 and recomputes the complete canonical corpus identity. A changed peer corpus restarts pagination. Missing pages, duplicate identities, coverage holes, compacted indices, and oversized observations never become evidence of absence.

Pure reconciliation plans repairs; the shell uses existing persistence, flush, snapshot restore and committed-application effects. Snapshot installation precedes entry repair. Committed conflicts preserve evidence and prevent every repair effect. A quorum-held matching uncommitted suffix can remain without being promoted to committed history. Unreplayable durable record streams remain denied; this protocol does not silently reconstruct malformed storage records or reset a store.

The recovery-required marker is stored with term and vote in hard-state-v2 before startup effects or higher-term recovery traffic. Legacy v1 history remains readable, but new writes use v2. Clean restart before admission cannot clear suspicion. Successful reconciliation atomically clears the marker with a conservative self-vote, fencing the recovered term before voting resumes in a strictly later term. A log or snapshot above the recovered hard-state term also forces admission. Failed durability or application effects stop the replica pending reopen/read-back. Failed sends instead remain bounded typed loss observations, preserve timers and the planned transition, and degrade health without fabricated delivery evidence.

Every persistence/application batch, including an ordinary healthy-voter mutation, durably holds the recovery-required marker until all planned effects finish. Intermediate hard-state writes cannot clear it. A successful non-recovering batch finishes with the exact planned term and vote plus a cleared marker. Fence writes count against the admitted effect budget before any effect runs. This preserves uncertainty across a clean close/open even when no subsequent Redb repair callback occurs.

Snapshot restart selects the highest Raft boundary when durable coverage ties. It replays the required suffix against the latest snapshot's coverage rather than requiring older installed snapshots to reconstruct a missing prefix. Peer repair retains all above-boundary entries pending per-index reconciliation, including a mismatched repaired prefix; ordinary snapshot installation retains its matching-boundary behavior. The durable port records that disposition before metadata publication, so restart cannot silently discard unresolved or committed suffix evidence.

### Decision: Evidence stays cohort-bound

Receipts bind the candidate BLAKE3, Molten profile, Redb version, kernel, filesystem, mount options, virtual or physical device profile, fault schedule, and recovery result.

Receipts state that the campaign does not prove arbitrary hardware, kernels, filesystems, schedules, or whole-cluster correctness.

## Architecture

```text
storage or consensus request
  -> application shell
  -> pure durability or recovery decision
  -> typed persistence or peer-query plan
  -> Redb, file-publication, or peer adapter
  -> typed observation
  -> pure classification
  -> state, quarantine, receipt facts, or next plan
```

## Test Design

Positive tests cover synchronized commits, durable snapshot publication, clean restart, valid peer repair, and admitted rejoin.

Negative tests cover `fsync` failure, commit uncertainty, retained stale page-cache bytes, corrupt records, partial snapshots, failed parent sync, local Redb repair, conflicting peer histories, lost term or vote state, and unknown commitment.

A three-node campaign must include one faulty quorum-intersection node, one unavailable correct node, and one lagging node.

## Claim Boundary

This change establishes explicit storage-fault behavior for the selected Molten and campaign cohorts. It does not prove arbitrary device durability or whole-system correctness.
