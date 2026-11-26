---- MODULE ConsensusFastPath ----
\* Bounded finite design model of the blocked fast-path acceleration package
\* (crates/molten-fastpath-core), not a live-engine proof. Every replica
\* keeps its own acceptance log; client requests reach replicas in any order;
\* fast admission counts real per-replica acknowledgements against
\* SuperQuorumSize; view-change recovery sees only the logs of a
\* QuorumSize-replica recovery quorum and keeps what RecoveryThreshold of them
\* agree on. Configurations with QuorumsDerived = TRUE use the quorums
\* derive_quorums() derives (majority N \div 2 + 1, superquorum
\* 3N \div 4 + 1). With three replicas the derived superquorum is every
\* replica, so recovery-quorum selection is trivial. The four-replica
\* configurations are an explicitly NON-derived intersection exercise
\* (superquorum 3, where derive_quorums(4) gives 4) in which recovery
\* selection is load-bearing and an undersized recovery quorum must fail; the
\* derived five-replica instance does not fit the campaign's time limits.
\* Mutants and that exercise set QuorumsDerived = FALSE. The original (base)
\* engine is the separately model-checked production Raft (AspenRaft.tla);
\* here it is an abstract durable commit log fed by the active proposer.
\*
\* A compatible promise is modeled as the order-preserving active proposer
\* having itself acknowledged the command at that position. Rust
\* evaluate_stable_view enforces it (FallbackReason::MissingProposerAcknowledgement:
\* every active proposer must be among the same-view acknowledgers, with
\* conflict_free standing for the equal prefix). Without this conjunct the
\* four-replica baseline violates CommittedOrderAgreement; see the resolved
\* source discrepancy in docs/consensus-fastpath-tla.md.
\* r[impl molten.consensus.production_raft_formal_model]
EXTENDS Naturals, FiniteSets, Sequences

CONSTANTS Replicas, Proposer, QuorumSize, SuperQuorumSize, QuorumsDerived, Commands, MaxView,
          EnforcePromises

VARIABLES
    rlog,       \* per-replica acceptance order in its current view
    rview,      \* per-replica view
    rstatus,    \* per-replica "normal" (accepting) or "paused" (in recovery)
    accelView,  \* coordinator acceleration view
    baseView,   \* coordinator base view
    phase,      \* coordinator recovery phase (RecoveryPhase)
    promise,    \* active proposer's promise for the current view
    recovered,  \* agreed recovery log (prefix every replica installs on resume)
    baseLog,    \* original engine's durable committed order
    chosen      \* every decision <<position, command>>, fast or original path

replicaVars == <<rlog, rview, rstatus>>
coordinatorVars == <<accelView, baseView, phase, promise, recovered>>
vars == <<replicaVars, coordinatorVars, baseLog, chosen>>

N == Cardinality(Replicas)

ASSUME /\ Proposer \in Replicas
       /\ QuorumSize \in 1..N
       /\ SuperQuorumSize \in 1..N
       /\ QuorumsDerived \in BOOLEAN
       \* derive_quorums(): majority and three-quarter superquorum
       /\ QuorumsDerived => /\ QuorumSize = N \div 2 + 1
                            /\ SuperQuorumSize = (3 * N) \div 4 + 1
       /\ MaxView >= 1
       /\ EnforcePromises \in BOOLEAN

\* Every recovery quorum shares at least this many replicas with every fast
\* superquorum, so a fast-committed command is held by at least this many
\* reports. Recovery keeps what this many reports agree on; that choice is
\* unique only when 2 * RecoveryThreshold > QuorumSize, i.e.
\* QuorumSize + 2 * SuperQuorumSize > 2 * N.
RecoveryThreshold == QuorumSize + SuperQuorumSize - N

Items(s) == {s[i] : i \in 1..Len(s)}
Unique(s) == Cardinality(Items(s)) = Len(s)
IsPrefix(a, b) == Len(a) <= Len(b) /\ SubSeq(b, 1, Len(a)) = a
\* Bounded sequences and duplicate-free orders over a finite set.
Seqs(S) == UNION {[1..n -> S] : n \in 0..Cardinality(S)}
Orders(S) == {s \in Seqs(S) : Unique(s)}

\* Replica r acknowledged command c behind prefix p in view v. A replica
\* acknowledges each accepted command with its whole acceptance prefix; it
\* stays valid while r remains normal in v (its log only grows there).
Acknowledged(r, v, p, c) ==
    /\ rview[r] = v
    /\ rstatus[r] = "normal"
    /\ IsPrefix(Append(p, c), rlog[r])

TypeOK ==
    /\ rlog \in [Replicas -> Orders(Commands)]
    /\ rview \in [Replicas -> 1..MaxView]
    /\ rstatus \in [Replicas -> {"normal", "paused"}]
    /\ accelView \in 1..MaxView
    /\ baseView \in accelView..MaxView
    /\ phase \in {"normal", "paused", "agreed", "marked"}
    /\ promise \in {"preserving", "reordering"}
    /\ recovered \in Orders(Commands)
    /\ baseLog \in Orders(Commands)
    /\ chosen \subseteq (1..Cardinality(Commands)) \X Commands

Init ==
    /\ rlog = [r \in Replicas |-> <<>>]
    /\ rview = [r \in Replicas |-> 1]
    /\ rstatus = [r \in Replicas |-> "normal"]
    /\ accelView = 1
    /\ baseView = 1
    /\ phase = "normal"
    \* ProposerPromise.proposal_order_preserved, fixed for the whole view.
    /\ promise \in {"preserving", "reordering"}
    /\ recovered = <<>>
    /\ baseLog = <<>>
    /\ chosen = {}

\* A client request reaches replica r. Requests are broadcast and the network
\* reorders them, so replicas may accept commands in different orders.
Accept(r, c) ==
    /\ rstatus[r] = "normal"
    /\ c \notin Items(rlog[r])
    /\ rlog' = [rlog EXCEPT ![r] = Append(@, c)]
    /\ UNCHANGED <<rview, rstatus, coordinatorVars, baseLog, chosen>>

\* evaluate_stable_view: same acceleration/base view, a superquorum of
\* same-view acknowledgements of one command at one position (all commands
\* conflict, so equal prefixes are the conflict-free condition), and a
\* compatible promise from the active proposer: it preserves its proposal
\* order for the view and (enforced in Rust, see header) has itself
\* acknowledged this command at this position.
FastCommit(r0, k) ==
    /\ phase = "normal"
    /\ accelView = baseView
    /\ k \in 1..Len(rlog[r0])
    /\ LET p == SubSeq(rlog[r0], 1, k - 1)
           c == rlog[r0][k]
       IN /\ Cardinality({r \in Replicas : Acknowledged(r, accelView, p, c)}) >= SuperQuorumSize
          /\ EnforcePromises =>
                 promise = "preserving" /\ Acknowledged(Proposer, accelView, p, c)
          /\ chosen' = chosen \cup {<<k, c>>}
    /\ UNCHANGED <<replicaVars, coordinatorVars, baseLog>>

\* The original engine commits the active proposer's next proposal. An
\* order-preserving proposer proposes in its own acceptance order; a
\* reordering proposer may propose any command it accepted.
OriginalCommit(c) ==
    /\ phase = "normal"
    /\ rview[Proposer] = baseView
    /\ rstatus[Proposer] = "normal"
    /\ c \in Items(rlog[Proposer]) \ Items(baseLog)
    /\ promise = "preserving" =>
           Len(rlog[Proposer]) > Len(baseLog) /\ rlog[Proposer][Len(baseLog) + 1] = c
    /\ baseLog' = Append(baseLog, c)
    /\ chosen' = chosen \cup {<<Len(baseLog) + 1, c>>}
    /\ UNCHANGED <<replicaVars, coordinatorVars>>

\* A replica executes, in its own acceptance order, every command whose
\* position is decided with that command; it stops at its first undecided or
\* differently decided position (a conflicting predecessor blocks execution).
ExecutedLength(r) ==
    CHOOSE k \in 0..Len(rlog[r]) :
        /\ \A i \in 1..k : <<i, rlog[r][i]>> \in chosen
        /\ k = Len(rlog[r]) \/ <<k + 1, rlog[r][k + 1]>> \notin chosen
Executed(r) == SubSeq(rlog[r], 1, ExecutedLength(r))

\* transition_recovery: BeginViewChange (also interrupts a recovery).
BeginViewChange ==
    /\ baseView < MaxView
    /\ baseView' = baseView + 1
    /\ phase' = "paused"
    /\ UNCHANGED <<replicaVars, accelView, promise, recovered, baseLog, chosen>>

\* Replica r learns the new base view, stops accepting and reports its log.
JoinView(r) ==
    /\ rview[r] < baseView
    /\ rview' = [rview EXCEPT ![r] = baseView]
    /\ rstatus' = [rstatus EXCEPT ![r] = "paused"]
    /\ UNCHANGED <<rlog, coordinatorVars, baseLog, chosen>>

\* Extension e of the durable base log is held by at least
\* RecoveryThreshold of the reporting replicas at every position.
Supported(Q, e) ==
    \A k \in 1..Len(e) :
        Cardinality({r \in Q : IsPrefix(baseLog \o SubSeq(e, 1, k), rlog[r])}) >= RecoveryThreshold

\* AgreeRecoverySet from the reports of a recovery quorum Q only.
AgreeRecoverySet ==
    /\ phase = "paused"
    /\ \E Q \in SUBSET {r \in Replicas : rview[r] = baseView /\ rstatus[r] = "paused"} :
        /\ Cardinality(Q) >= QuorumSize
        /\ \E e \in Orders(Commands \ Items(baseLog)) :
            /\ Supported(Q, e)
            /\ ~\E c \in Commands \ (Items(baseLog) \union Items(e)) : Supported(Q, Append(e, c))
            /\ recovered' = baseLog \o e
    /\ phase' = "agreed"
    /\ UNCHANGED <<replicaVars, accelView, baseView, promise, baseLog, chosen>>

\* Recovered work is committed through the original engine, in order.
RecoveryCommit ==
    /\ phase = "agreed"
    /\ Len(baseLog) < Len(recovered)
    /\ baseLog' = Append(baseLog, recovered[Len(baseLog) + 1])
    /\ chosen' = chosen \cup {<<Len(baseLog) + 1, recovered[Len(baseLog) + 1]>>}
    /\ UNCHANGED <<replicaVars, coordinatorVars>>

CommitRecoveryMarker ==
    /\ phase = "agreed"
    /\ baseLog = recovered
    /\ phase' = "marked"
    /\ UNCHANGED <<replicaVars, accelView, baseView, promise, recovered, baseLog, chosen>>

ResumeNormalView ==
    /\ phase = "marked"
    /\ accelView' = baseView
    /\ phase' = "normal"
    /\ promise' \in {"preserving", "reordering"}
    /\ UNCHANGED <<replicaVars, baseView, recovered, baseLog, chosen>>

\* A paused replica installs the new view's recovered prefix and resumes.
InstallView(r) ==
    /\ phase = "normal"
    /\ rview[r] = baseView
    /\ rstatus[r] = "paused"
    /\ rlog' = [rlog EXCEPT ![r] = recovered]
    /\ rstatus' = [rstatus EXCEPT ![r] = "normal"]
    /\ UNCHANGED <<rview, coordinatorVars, baseLog, chosen>>

RecoveryProgress ==
    \/ \E r \in Replicas : JoinView(r)
    \/ AgreeRecoverySet
    \/ RecoveryCommit
    \/ CommitRecoveryMarker
    \/ ResumeNormalView

Next ==
    \/ \E r \in Replicas, c \in Commands : Accept(r, c)
    \/ \E r \in Replicas, k \in 1..Cardinality(Commands) : FastCommit(r, k)
    \/ \E c \in Commands : OriginalCommit(c)
    \/ \E r \in Replicas : InstallView(r)
    \/ BeginViewChange
    \/ RecoveryProgress

Spec == Init /\ [][Next]_vars /\ WF_vars(RecoveryProgress)

\* No two decisions (fast or original path, in any view) give one position
\* two commands or one command two positions.
CommittedOrderAgreement ==
    \A a, b \in chosen : (a[1] = b[1]) <=> (a[2] = b[2])

\* Once recovery commits its marker, every decision made in any earlier view
\* survives at its position in the original engine's durable order.
Recoverability ==
    phase = "marked" =>
        \A d \in chosen : d[1] <= Len(baseLog) /\ baseLog[d[1]] = d[2]

\* The three properties below are derived projections kept to name the
\* consensus profile invariants. Execution is computed from each replica's
\* log and `chosen` (no independent execution state), so each one is implied
\* by CommittedOrderAgreement and none is independent evidence.

\* Each replica's execution agrees with the original engine's order.
ExecutionOrderAgreement ==
    \A r \in Replicas :
        \A k \in 1..Len(Executed(r)) : k <= Len(baseLog) => Executed(r)[k] = baseLog[k]

\* All replicas execute conflicting commands in one serial order.
LinearizableConflictOrder ==
    \A r, s \in Replicas : IsPrefix(Executed(r), Executed(s)) \/ IsPrefix(Executed(s), Executed(r))

\* No command is decided at two positions (the second half of
\* CommittedOrderAgreement), so no state machine can apply it twice.
AtMostOnceApplication ==
    \A a, b \in chosen : a[2] = b[2] => a[1] = b[1]

RecoveryEventuallyResumes == (phase # "normal") ~> (phase = "normal")
====
