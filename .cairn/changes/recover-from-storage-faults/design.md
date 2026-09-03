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
