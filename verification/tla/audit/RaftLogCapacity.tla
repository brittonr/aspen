---- MODULE RaftLogCapacity ----
\* Bounded TLA+ model of Raft log capacity and the duplicate-request map
\* (issue #56). It abstracts one leader's log to counts.
\* It is a model, not a live-engine proof or a production profile.
\*
\* Source mapping (brittonr/aspen at 8ae93a5e5):
\*   Propose     src/fabric_consistency/raft/transition/client.rs handle_proposal
\*               (Denied when log.len() >= max_log_entries)
\*   Commit      transition/message/replication.rs advance_leader_commit
\*               (record_completed_requests inserts into completed_requests)
\*   Snapshot    client.rs handle_create_snapshot (drops entries <= last_applied,
\*               copies completed_requests into the snapshot, never evicts)
\*   The reliability service never issues CreateSnapshot (no caller under src/reliability);
\*   collaboration scope and the live-process harness do.
\*
\* Compaction = TRUE: the host snapshots when committed entries exist.
\* EvictDedup = TRUE: completed request refs at or below a watermark are evicted
\* (the suggested fix; the watermark rule itself is a design decision).
EXTENDS Naturals

CONSTANTS
    MaxLogEntries,
    DedupLimit,
    Compaction,
    EvictDedup

VARIABLES
    uncommitted,   \* entries in the log not yet committed
    committedInLog,\* committed entries still in the log (not yet compacted)
    dedup,         \* size of completed_requests
    denied         \* the most recent proposal was denied

vars == <<uncommitted, committedInLog, dedup, denied>>

LogLen == uncommitted + committedInLog

TypeOK ==
    /\ uncommitted \in 0..MaxLogEntries
    /\ committedInLog \in 0..MaxLogEntries
    /\ dedup \in 0..(DedupLimit + 1)
    /\ denied \in BOOLEAN

Init == uncommitted = 0 /\ committedInLog = 0 /\ dedup = 0 /\ denied = FALSE

Propose ==
    IF LogLen < MaxLogEntries
        THEN /\ uncommitted' = uncommitted + 1 /\ denied' = FALSE
             /\ UNCHANGED <<committedInLog, dedup>>
        ELSE /\ denied' = TRUE
             /\ UNCHANGED <<uncommitted, committedInLog, dedup>>

\* dedup is capped one past DedupLimit so the state space stays finite; reaching
\* DedupLimit + 1 already violates DedupBounded.
Commit ==
    /\ uncommitted > 0
    /\ uncommitted' = uncommitted - 1
    /\ committedInLog' = committedInLog + 1
    /\ dedup' = IF dedup <= DedupLimit THEN dedup + 1 ELSE dedup
    /\ UNCHANGED denied

Snapshot ==
    /\ Compaction
    /\ committedInLog > 0
    /\ committedInLog' = 0
    /\ dedup' = IF EvictDedup THEN 0 ELSE dedup
    /\ UNCHANGED <<uncommitted, denied>>

Next == Propose \/ Commit \/ Snapshot

\* Clients keep proposing; commits and host-driven snapshots eventually happen.
Spec == Init /\ [][Next]_vars /\ WF_vars(Propose) /\ WF_vars(Commit) /\ WF_vars(Snapshot)

\* Liveness: the service keeps accepting writes.
AlwaysEventuallyAccepts == []<>(~denied)

\* Safety: the duplicate-request map stays within a fixed bound.
DedupBounded == dedup <= DedupLimit
====
