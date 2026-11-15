---- MODULE ConsensusFastPath ----
\* Authoritative finite design model, not a live-engine proof.
EXTENDS Naturals, FiniteSets, Sequences

CONSTANTS Replicas, Quorum, SuperQuorum, Commands, MaxView, EnforcePromises
VARIABLES accepted, committed, fastCommitted, decisionOrder, commitOrder,
          executionOrder, replied, promised, ackView, accelView, baseView,
          phase, recoverySet, recoveryMarker, certificates
vars == <<accepted, committed, fastCommitted, decisionOrder, commitOrder,
          executionOrder, replied, promised, ackView, accelView, baseView,
          phase, recoverySet, recoveryMarker, certificates>>

ASSUME /\ Quorum \subseteq Replicas
       /\ SuperQuorum \subseteq Replicas
       /\ Cardinality(Quorum) * 2 > Cardinality(Replicas)
       /\ Cardinality(SuperQuorum) * 4 > Cardinality(Replicas) * 3
       /\ MaxView >= 1
       /\ EnforcePromises \in BOOLEAN

Items(s) == {s[i] : i \in 1..Len(s)}
Unique(s) == Cardinality(Items(s)) = Len(s)
Prefix(a, b) == /\ Len(a) <= Len(b)
                /\ \A i \in 1..Len(a) : a[i] = b[i]
Conflict(a, b) == a # b
Pending(c) == {d \in accepted \ committed : Conflict(d, c)}

TypeOK ==
    /\ accepted \subseteq Commands
    /\ committed \subseteq accepted
    /\ fastCommitted \subseteq accepted
    /\ decisionOrder \in Seq(Commands)
    /\ commitOrder \in Seq(Commands)
    /\ Len(decisionOrder) <= Cardinality(Commands)
    /\ Len(commitOrder) <= Cardinality(Commands)
    /\ executionOrder \in [Replicas -> Seq(Commands)]
    /\ \A r \in Replicas : Len(executionOrder[r]) <= Cardinality(Commands)
    /\ replied \in Seq(Commands)
    /\ Len(replied) <= Cardinality(Commands)
    /\ promised \subseteq Replicas
    /\ ackView \in [Replicas -> 0..MaxView]
    /\ accelView \in 1..MaxView
    /\ baseView \in accelView..MaxView
    /\ phase \in {"normal", "paused", "agreed", "marked"}
    /\ recoverySet \subseteq accepted
    /\ recoveryMarker \in BOOLEAN
    /\ certificates \subseteq
         [command : Commands, acceleration : 1..MaxView, base : 1..MaxView,
          acknowledgements : [Replicas -> 0..MaxView], promises : SUBSET Replicas,
          predecessors : SUBSET Commands]

Init ==
    /\ accepted = {}
    /\ committed = {}
    /\ fastCommitted = {}
    /\ decisionOrder = <<>>
    /\ commitOrder = <<>>
    /\ executionOrder = [r \in Replicas |-> <<>>]
    /\ replied = <<>>
    /\ promised = {}
    /\ ackView = [r \in Replicas |-> 0]
    /\ accelView = 1
    /\ baseView = 1
    /\ phase = "normal"
    /\ recoverySet = {}
    /\ recoveryMarker = FALSE
    /\ certificates = {}

Propose(c) ==
    /\ phase = "normal"
    /\ c \notin accepted
    /\ accepted' = accepted \union {c}
    /\ UNCHANGED <<committed, fastCommitted, decisionOrder, commitOrder,
                   executionOrder, replied, promised, ackView, accelView,
                   baseView, phase, recoverySet, recoveryMarker, certificates>>

Acknowledge(r) ==
    /\ phase = "normal"
    /\ ackView[r] # accelView
    /\ ackView' = [ackView EXCEPT ![r] = accelView]
    /\ UNCHANGED <<accepted, committed, fastCommitted, decisionOrder, commitOrder,
                   executionOrder, replied, promised, accelView, baseView,
                   phase, recoverySet, recoveryMarker, certificates>>

Promise(r) ==
    /\ phase = "normal"
    /\ r \notin promised
    /\ promised' = promised \union {r}
    /\ UNCHANGED <<accepted, committed, fastCommitted, decisionOrder, commitOrder,
                   executionOrder, replied, ackView, accelView, baseView,
                   phase, recoverySet, recoveryMarker, certificates>>

FastCommit(c) ==
    /\ phase = "normal"
    /\ baseView = accelView
    /\ c \in accepted \ Items(decisionOrder)
    /\ \A r \in SuperQuorum : ackView[r] = accelView
    /\ (~EnforcePromises \/ promised = Replicas)
    /\ Pending(c) = {}
    /\ fastCommitted' = fastCommitted \union {c}
    /\ decisionOrder' = Append(decisionOrder, c)
    /\ certificates' = certificates \union
         {[command |-> c, acceleration |-> accelView, base |-> baseView,
           acknowledgements |-> ackView, promises |-> promised,
           predecessors |-> Pending(c)]}
    /\ UNCHANGED <<accepted, committed, commitOrder, executionOrder, replied,
                   promised, ackView, accelView, baseView, phase,
                   recoverySet, recoveryMarker>>

\* The original engine's proposal/commit order is an assumption, not proved here.
Commit(c) ==
    /\ c \in accepted \ committed
    /\ IF c \in Items(decisionOrder)
          THEN /\ Len(commitOrder) < Len(decisionOrder)
               /\ decisionOrder[Len(commitOrder) + 1] = c
               /\ UNCHANGED decisionOrder
          ELSE /\ Len(commitOrder) = Len(decisionOrder)
               /\ decisionOrder' = Append(decisionOrder, c)
    /\ committed' = committed \union {c}
    /\ commitOrder' = Append(commitOrder, c)

OriginalCommit(c) ==
    /\ phase = "normal"
    /\ Commit(c)
    /\ UNCHANGED <<accepted, fastCommitted, executionOrder, replied, promised,
                   ackView, accelView, baseView, phase, recoverySet,
                   recoveryMarker, certificates>>

BeginViewChange ==
    /\ baseView < MaxView
    /\ baseView' = baseView + 1
    /\ phase' = "paused"
    /\ recoveryMarker' = FALSE
    /\ recoverySet' = {}
    /\ promised' = {}
    \* Old acknowledgements deliberately survive: mixed-view admission must fail.
    /\ UNCHANGED <<accepted, committed, fastCommitted, decisionOrder, commitOrder,
                   executionOrder, replied, ackView, accelView, certificates>>

AgreeRecoverySet ==
    /\ phase = "paused"
    /\ recoverySet' = accepted \ committed
    /\ phase' = "agreed"
    /\ UNCHANGED <<accepted, committed, fastCommitted, decisionOrder, commitOrder,
                   executionOrder, replied, promised, ackView, accelView,
                   baseView, recoveryMarker, certificates>>

RecoveryCommit(c) ==
    /\ phase = "agreed"
    /\ c \in recoverySet
    /\ Commit(c)
    /\ UNCHANGED <<accepted, fastCommitted, executionOrder, replied, promised,
                   ackView, accelView, baseView, phase, recoverySet,
                   recoveryMarker, certificates>>

CommitRecoveryMarker ==
    /\ phase = "agreed"
    /\ recoverySet \subseteq committed
    /\ recoveryMarker' = TRUE
    /\ phase' = "marked"
    /\ UNCHANGED <<accepted, committed, fastCommitted, decisionOrder, commitOrder,
                   executionOrder, replied, promised, ackView, accelView,
                   baseView, recoverySet, certificates>>

ResumeNormalView ==
    /\ phase = "marked"
    /\ recoveryMarker
    /\ recoverySet \subseteq committed
    /\ accelView' = baseView
    /\ phase' = "normal"
    /\ UNCHANGED <<accepted, committed, fastCommitted, decisionOrder, commitOrder,
                   executionOrder, replied, promised, ackView, baseView,
                   recoverySet, recoveryMarker, certificates>>

Apply(r) ==
    /\ Len(executionOrder[r]) < Len(decisionOrder)
    /\ executionOrder' = [executionOrder EXCEPT
           ![r] = Append(@, decisionOrder[Len(@) + 1])]
    /\ UNCHANGED <<accepted, committed, fastCommitted, decisionOrder, commitOrder,
                   replied, promised, ackView, accelView, baseView, phase,
                   recoverySet, recoveryMarker, certificates>>

Reply ==
    /\ Len(replied) < Len(decisionOrder)
    /\ \E r \in Replicas : Len(executionOrder[r]) > Len(replied)
    /\ replied' = Append(replied, decisionOrder[Len(replied) + 1])
    /\ UNCHANGED <<accepted, committed, fastCommitted, decisionOrder, commitOrder,
                   executionOrder, promised, ackView, accelView, baseView,
                   phase, recoverySet, recoveryMarker, certificates>>

RecoveryProgress ==
    \/ AgreeRecoverySet
    \/ \E c \in Commands : RecoveryCommit(c)
    \/ CommitRecoveryMarker
    \/ ResumeNormalView
Next ==
    \/ \E c \in Commands : Propose(c) \/ FastCommit(c) \/ OriginalCommit(c)
    \/ \E r \in Replicas : Acknowledge(r) \/ Promise(r) \/ Apply(r)
    \/ BeginViewChange
    \/ RecoveryProgress
    \/ Reply
Spec == Init /\ [][Next]_vars /\ WF_vars(RecoveryProgress)

Recoverability ==
    /\ fastCommitted \subseteq accepted
    /\ phase \in {"agreed", "marked"} => accepted \ committed \subseteq recoverySet
NoConflictingPredecessor == \A cert \in certificates : cert.predecessors = {}
CommittedOrderAgreement ==
    /\ Prefix(commitOrder, decisionOrder)
    /\ committed = Items(commitOrder)
ExecutionOrderAgreement == \A r \in Replicas : Prefix(executionOrder[r], decisionOrder)
LinearizableConflictOrder ==
    /\ Unique(decisionOrder)
    /\ Items(decisionOrder) = committed \union fastCommitted
    /\ Prefix(replied, decisionOrder)
AtMostOnceApplication ==
    /\ \A r \in Replicas : Unique(executionOrder[r])
    /\ Unique(replied)
RepliesAfterApplication ==
    \A i \in 1..Len(replied) : \E r \in Replicas : i <= Len(executionOrder[r])
NoMissingPromise == \A cert \in certificates : cert.promises = Replicas
NoMixedViewCommit ==
    \A cert \in certificates :
        /\ cert.base = cert.acceleration
        /\ \A r \in SuperQuorum : cert.acknowledgements[r] = cert.acceleration
RecoveryBeforeNewView ==
    /\ (phase = "normal" /\ accelView > 1) => recoveryMarker
    /\ phase = "normal" => baseView = accelView
    /\ phase = "marked" => recoverySet \subseteq committed
RecoveryEventuallyResumes == (phase # "normal") ~> (phase = "normal")
====
