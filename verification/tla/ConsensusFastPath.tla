---- MODULE ConsensusFastPath ----
\* Bounded TLA+ model of the Molten consensus fast-path hazard contract.
\* It is a model, not a live-engine proof or a production profile.
EXTENDS Naturals, FiniteSets, TLC

CONSTANTS
    Replicas,
    Quorum,
    SuperQuorum,
    Commands,
    MaxView

VARIABLES
    accepted,
    committed,
    fastCommitted,
    applied,
    replied,
    promised,
    ackView,
    accelView,
    phase,
    recoverySet,
    recoveryMarker

vars == <<accepted, committed, fastCommitted, applied, replied, promised, ackView,
          accelView, phase, recoverySet, recoveryMarker>>

ASSUME Quorum \subseteq Replicas
ASSUME SuperQuorum \subseteq Replicas
ASSUME SuperQuorum # {}

TypeOK ==
    /\ accepted \subseteq Commands
    /\ committed \subseteq Commands
    /\ fastCommitted \subseteq Commands
    /\ applied \subseteq Commands
    /\ replied \subseteq Commands
    /\ promised \subseteq Replicas
    /\ \A r \in Replicas : ackView[r] \in 0..accelView
    /\ accelView \in 1..MaxView
    /\ phase \in {"normal", "recovering"}
    /\ recoverySet \subseteq Commands
    /\ recoveryMarker \in BOOLEAN

Init ==
    /\ accepted = {}
    /\ committed = {}
    /\ fastCommitted = {}
    /\ applied = {}
    /\ replied = {}
    /\ promised = {}
    /\ ackView = [r \in Replicas |-> 0]
    /\ accelView = 1
    /\ phase = "normal"
    /\ recoverySet = {}
    /\ recoveryMarker = FALSE

Conflict(d, c) == d # c

ConflictFree(c) ==
    \A d \in committed : ~Conflict(d, c)

SameViewSuperQuorum ==
    /\ \A r \in SuperQuorum : ackView[r] = accelView
    /\ promised = Replicas

Propose(c) ==
    /\ c \in Commands
    /\ c \notin accepted
    /\ phase = "normal"
    /\ accepted' = accepted \union {c}
    /\ UNCHANGED <<committed, applied, replied, promised, ackView,
                   accelView, phase, recoverySet, recoveryMarker, fastCommitted>>

FastCommit(c) ==
    /\ c \in accepted
    /\ c \notin committed
    /\ phase = "normal"
    /\ SameViewSuperQuorum
    /\ ConflictFree(c)
    /\ committed' = committed \union {c}
    /\ fastCommitted' = fastCommitted \union {c}
    /\ UNCHANGED <<accepted, applied, replied, promised, ackView,
                   accelView, phase, recoverySet, recoveryMarker>>

OriginalCommit(c) ==
    /\ c \in accepted
    /\ c \notin committed
    /\ phase = "normal"
    /\ committed' = committed \union {c}
    /\ UNCHANGED <<accepted, applied, replied, promised, ackView,
                   accelView, phase, recoverySet, recoveryMarker, fastCommitted>>

Acknowledge(r) ==
    /\ r \in Replicas
    /\ ackView' = [ackView EXCEPT ![r] = accelView]
    /\ UNCHANGED <<accepted, committed, applied, replied, promised,
                   accelView, phase, recoverySet, recoveryMarker, fastCommitted>>

Promise(r) ==
    /\ r \in Replicas
    /\ promised' = promised \union {r}
    /\ UNCHANGED <<accepted, committed, applied, replied, ackView,
                   accelView, phase, recoverySet, recoveryMarker, fastCommitted>>

NewView ==
    /\ phase = "normal"
    /\ recoveryMarker
    /\ accelView < MaxView
    /\ accelView' = accelView + 1
    /\ ackView' = [r \in Replicas |-> 0]
    /\ promised' = {}
    /\ UNCHANGED <<accepted, committed, applied, replied, phase,
                   recoverySet, recoveryMarker, fastCommitted>>

StartRecovery ==
    /\ phase = "normal"
    /\ phase' = "recovering"
    /\ recoverySet' = accepted \ committed
    /\ recoveryMarker' = FALSE
    /\ UNCHANGED <<accepted, committed, applied, replied, promised,
                   ackView, accelView, fastCommitted>>

RecoveryCommit(c) ==
    /\ phase = "recovering"
    /\ c \in recoverySet
    /\ c \notin committed
    /\ committed' = committed \union {c}
    /\ UNCHANGED <<accepted, applied, replied, promised, ackView,
                   accelView, phase, recoverySet, recoveryMarker, fastCommitted>>

FinishRecovery ==
    /\ phase = "recovering"
    /\ recoverySet \subseteq committed
    /\ recoveryMarker' = TRUE
    /\ phase' = "normal"
    /\ UNCHANGED <<accepted, committed, applied, replied, promised,
                   ackView, accelView, recoverySet, fastCommitted>>

Apply(c) ==
    /\ c \in committed
    /\ c \notin applied
    /\ applied' = applied \union {c}
    /\ UNCHANGED <<accepted, committed, replied, promised, ackView,
                   accelView, phase, recoverySet, recoveryMarker, fastCommitted>>

Reply(c) ==
    /\ c \in applied
    /\ c \notin replied
    /\ replied' = replied \union {c}
    /\ UNCHANGED <<accepted, committed, applied, promised, ackView,
                   accelView, phase, recoverySet, recoveryMarker, fastCommitted>>

Next ==
    \/ \E c \in Commands : Propose(c) \/ FastCommit(c) \/ OriginalCommit(c) \/
                            Apply(c) \/ Reply(c) \/ RecoveryCommit(c)
    \/ \E r \in Replicas : Acknowledge(r) \/ Promise(r)
    \/ NewView
    \/ StartRecovery
    \/ FinishRecovery

Spec == Init /\ [][Next]_vars

CommittedAccepted == committed \subseteq accepted
AppliedCommitted == applied \subseteq committed
RepliedApplied == replied \subseteq applied
RecoveryPreserves ==
    phase = "recovering" => accepted \ committed \subseteq recoverySet
RecoveryBeforeNewView ==
    (phase = "normal" /\ accelView > 1) => recoveryMarker
NoFastConflict ==
    \A d \in fastCommitted : \A c \in fastCommitted : ~Conflict(d, c)
NoMixedViewCommit ==
    \A r \in SuperQuorum : ackView[r] <= accelView
\* The model checks these finite invariants; it does not prove the external
\* Jetpack artifact, live transport, durability, timing, or production behavior.
====
