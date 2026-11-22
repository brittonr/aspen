---- MODULE RaftReadIndexWindow ----
\* A finite observation model, starting after a leader change. It checks the
\* ReadIndex window only, not elections, replication, persistence or liveness.
\* r[impl molten.testing.live_raft_trace.model]
EXTENDS Naturals, FiniteSets

CONSTANTS Voters, MaxTerm, MaxIndex

VARIABLES leader, term, logEnd, commitIndex, commitTerm, appliedIndex,
          priorAcknowledged, phase, requiredIndex, readAcks, observedIndex

vars == <<leader, term, logEnd, commitIndex, commitTerm, appliedIndex,
          priorAcknowledged, phase, requiredIndex, readAcks, observedIndex>>

ReplicaFacts == <<leader, term, logEnd, commitIndex, commitTerm, appliedIndex, priorAcknowledged>>
ReadFacts == <<phase, requiredIndex, readAcks, observedIndex>>

ASSUME /\ Cardinality(Voters) = 3
       /\ MaxTerm >= 2
       /\ MaxIndex >= 2

Quorum(acks) == 2 * Cardinality(acks) > Cardinality(Voters)
Ready == commitTerm = term \/ commitIndex = logEnd

TypeOK ==
    /\ leader \in Voters
    /\ term \in 2..MaxTerm
    /\ logEnd \in 1..MaxIndex
    /\ priorAcknowledged \in 1..logEnd
    /\ commitIndex \in 0..logEnd
    /\ commitTerm \in 0..term
    /\ (commitIndex = 0) <=> (commitTerm = 0)
    /\ appliedIndex \in 0..commitIndex
    /\ phase \in {"ready", "retryable", "pending", "current"}
    /\ requiredIndex \in 0..MaxIndex
    /\ readAcks \subseteq Voters
    /\ observedIndex \in 0..MaxIndex

Init ==
    /\ leader \in Voters
    /\ term \in 2..MaxTerm
    /\ logEnd \in 1..MaxIndex
    /\ priorAcknowledged \in 1..logEnd
    /\ commitIndex \in 0..priorAcknowledged
    /\ commitTerm = IF commitIndex = 0 THEN 0 ELSE term - 1
    /\ appliedIndex = commitIndex
    /\ phase = "ready"
    /\ requiredIndex = 0
    /\ readAcks = {}
    /\ observedIndex = 0

ReadRetry ==
    /\ phase = "ready"
    /\ ~Ready
    /\ phase' = "retryable"
    /\ observedIndex' = appliedIndex
    /\ UNCHANGED <<ReplicaFacts, requiredIndex, readAcks>>

\* One selected new write is appended and applied before this checkpoint. The
\* observation does not assign hypothetical intermediate RPC or durable states.
ObserveCommit ==
    /\ phase \in {"ready", "retryable"}
    /\ logEnd < MaxIndex
    /\ logEnd' = logEnd + 1
    /\ commitIndex' = logEnd'
    /\ commitTerm' = term
    /\ appliedIndex' = commitIndex'
    /\ UNCHANGED <<leader, term, priorAcknowledged, ReadFacts>>

ReadStart ==
    /\ phase \in {"ready", "retryable"}
    /\ Ready
    /\ phase' = "pending"
    /\ requiredIndex' = commitIndex
    /\ readAcks' = {leader}
    /\ observedIndex' = 0
    /\ UNCHANGED ReplicaFacts

\* The selected service handles the actual matching acknowledgement and returns
\* its Current result in the same transition. No synthetic pending quorum row.
ReadComplete ==
    /\ phase = "pending"
    /\ \E from \in Voters \ readAcks :
        /\ readAcks' = readAcks \cup {from}
        /\ Quorum(readAcks')
    /\ requiredIndex <= appliedIndex
    /\ phase' = "current"
    /\ observedIndex' = requiredIndex
    /\ UNCHANGED <<ReplicaFacts, requiredIndex>>

Next == ReadRetry \/ ObserveCommit \/ ReadStart \/ ReadComplete
Spec == Init /\ [][Next]_vars

\* This is a prefix obligation, not a proof of the application value.
ReadLinearizable == phase = "current" => observedIndex >= priorAcknowledged
====
