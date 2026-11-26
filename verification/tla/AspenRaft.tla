---- MODULE AspenRaft ----
\* Bounded design model of Aspen's production replicated Raft transition
\* function (crates/molten-raft-core/src/transition*). It starts from the
\* empty durable state of a fixed three-voter group: nothing about leaders,
\* logs or commitment is assumed in Init. Election Safety, Log Matching,
\* Leader Completeness, State Machine Safety and ReadIndex linearizability are
\* checked invariants. Finite-model evidence only; not a proof of the Rust code.
\* r[impl molten.consensus.production_raft_formal_model]
EXTENDS Naturals, FiniteSets, Sequences, TLC

CONSTANTS
    Server,            \* static voter set (STATIC_VOTER_COUNT)
    Nil,               \* "no vote" sentinel, not a server
    QuorumCount,       \* STATIC_QUORUM_COUNT
    MaxTerm,           \* election timeouts stop at this term (model bound)
    MaxLogLen,         \* proposals stop at this log length (model bound)
    MaxMessageEntries, \* profile.max_message_entries
    MaxInFlight,       \* network capacity (model bound)
    MaxReads,          \* linearizable read requests issued (model bound)
    Duplicates,        \* whether the network may deliver a message twice
    Restarts,          \* whether processes may crash and replay durable state
    Mutation           \* "none" or one seeded protocol bug, see Mutations

\* Each mutation removes exactly one guard of the Rust transition function.
Mutations == {
    "none",
    \* highest_quorum_index without `term_at(candidate) == current_term`
    "commit-previous-term",
    \* handle_request_vote without candidate_log_is_up_to_date
    "vote-without-up-to-date",
    \* handle_read without is_commit_index_current
    "read-without-current-commit",
    \* handle_append_entries without the previous_log_matches term comparison
    \* (only the previous index must exist)
    "append-without-prev-match"
}

ASSUME /\ Nil \notin Server
       /\ QuorumCount \in 1..Cardinality(Server)
       /\ MaxTerm \in Nat /\ MaxLogLen \in Nat
       /\ MaxMessageEntries \in Nat \ {0}
       /\ MaxInFlight \in Nat \ {0}
       /\ MaxReads \in Nat
       /\ Duplicates \in BOOLEAN
       /\ Restarts \in BOOLEAN
       /\ Mutation \in Mutations

VARIABLES
    currentTerm,   \* durable hard state
    votedFor,      \* durable hard state
    role,          \* volatile
    log,           \* full log as terms by index; 1..snapIdx are compacted
    snapIdx,       \* durable snapshot boundary (last_included_index)
    commitIndex,   \* durable commit (PersistCommit); last_applied == commitIndex
    votesGranted,  \* candidate volatile state
    nextIndex,     \* leader volatile state
    matchIndex,    \* leader volatile state
    pendingRead,   \* leader volatile ReadIndex request
    messages,      \* lossy, reordering, duplicating network
    readSeq,       \* request-ref allocator for linearizable reads
    \* Ghost history (never read by protocol actions):
    committed,     \* <<index, term, commitTerm>> committed by a leader
    applied,       \* <<index, term>> ever handed to any state machine
    reads          \* completed linearizable reads

serverVars == <<currentTerm, votedFor, role, votesGranted, nextIndex, matchIndex, pendingRead>>
logVars == <<log, snapIdx, commitIndex>>
ghostVars == <<committed, applied, reads>>
vars == <<serverVars, logVars, messages, readSeq, ghostVars>>

Follower == "Follower"
Candidate == "Candidate"
Leader == "Leader"

Min(a, b) == IF a < b THEN a ELSE b
Max(a, b) == IF a > b THEN a ELSE b
Range(f) == {f[x] : x \in DOMAIN f}
Cleared == [p \in Server |-> 0]
NoRead == [id |-> 0, term |-> 0, required |-> 0, acks |-> {}, mustObserve |-> 0]
\* Sentinel for term_at() == None; larger than every reachable term.
Absent == MaxTerm + 1

LastIndex(s) == Len(log[s])
LastTerm(s) == IF Len(log[s]) = 0 THEN 0 ELSE log[s][Len(log[s])]

\* support::term_at: index 0, the snapshot boundary, or a retained entry.
TermAt(s, i) ==
    IF i = 0 THEN 0
    ELSE IF i >= snapIdx[s] /\ i >= 1 /\ i <= Len(log[s]) THEN log[s][i]
    ELSE Absent

\* support::match_index_for
MatchFor(s, v) == IF v = s THEN LastIndex(s) ELSE matchIndex[s][v]

\* Highest committed index known anywhere; the ReadIndex obligation.
CommittedMax == IF committed = {} THEN 0
                ELSE CHOOSE i \in {e[1] : e \in committed} : \A e \in committed : e[1] <= i

Prefix(seq, n) == SubSeq(seq, 1, n)

----
\* Network. A response that would exceed the bound is lost, which the model's
\* lossy network permits anyway.
Put(base, r) == IF Cardinality(base \cup {r}) <= MaxInFlight THEN base \cup {r} ELSE base

Send(m) ==
    /\ m \notin messages
    /\ Cardinality(messages) < MaxInFlight
    /\ messages' = messages \cup {m}

\* Consume m, or keep it for a later duplicate delivery.
Base(m, keep) == IF keep THEN messages ELSE messages \ {m}

----
TypeOK ==
    /\ currentTerm \in [Server -> 0..MaxTerm]
    /\ votedFor \in [Server -> Server \cup {Nil}]
    /\ role \in [Server -> {Follower, Candidate, Leader}]
    /\ \A s \in Server :
        /\ log[s] \in Seq(1..MaxTerm)
        /\ Len(log[s]) <= MaxLogLen
        /\ snapIdx[s] \in 0..commitIndex[s]
        /\ commitIndex[s] \in 0..Len(log[s])
    /\ votesGranted \in [Server -> SUBSET Server]
    /\ nextIndex \in [Server -> [Server -> 0..(MaxLogLen + 1)]]
    /\ matchIndex \in [Server -> [Server -> 0..MaxLogLen]]
    /\ readSeq \in 0..MaxReads
    /\ Cardinality(messages) <= MaxInFlight

Init ==
    /\ currentTerm = [s \in Server |-> 0]
    /\ votedFor = [s \in Server |-> Nil]
    /\ role = [s \in Server |-> Follower]
    /\ log = [s \in Server |-> <<>>]
    /\ snapIdx = [s \in Server |-> 0]
    /\ commitIndex = [s \in Server |-> 0]
    /\ votesGranted = [s \in Server |-> {}]
    /\ nextIndex = [s \in Server |-> Cleared]
    /\ matchIndex = [s \in Server |-> Cleared]
    /\ pendingRead = [s \in Server |-> NoRead]
    /\ messages = {}
    /\ readSeq = 0
    /\ committed = {}
    /\ applied = {}
    /\ reads = {}

----
\* Local events.

\* election::handle_timeout (any non-leader voter).
Timeout(s) ==
    /\ role[s] # Leader
    /\ currentTerm[s] < MaxTerm
    /\ currentTerm' = [currentTerm EXCEPT ![s] = @ + 1]
    /\ votedFor' = [votedFor EXCEPT ![s] = s]
    /\ role' = [role EXCEPT ![s] = Candidate]
    /\ votesGranted' = [votesGranted EXCEPT ![s] = {s}]
    /\ nextIndex' = [nextIndex EXCEPT ![s] = Cleared]
    /\ matchIndex' = [matchIndex EXCEPT ![s] = Cleared]
    /\ pendingRead' = [pendingRead EXCEPT ![s] = NoRead]
    /\ UNCHANGED <<logVars, messages, readSeq, ghostVars>>

\* RequestVote effects of handle_timeout; a candidate's log and term are
\* stable while it remains candidate, so a later send carries the same fields.
SendRequestVote(s, p) ==
    /\ role[s] = Candidate
    /\ p # s
    /\ Send([mtype |-> "RV", mterm |-> currentTerm[s], msource |-> s, mdest |-> p,
             mlastIndex |-> LastIndex(s), mlastTerm |-> LastTerm(s)])
    /\ UNCHANGED <<serverVars, logVars, readSeq, ghostVars>>

\* client::handle_proposal -> append_proposal.
ClientRequest(s) ==
    /\ role[s] = Leader
    /\ Len(log[s]) < MaxLogLen
    /\ log' = [log EXCEPT ![s] = Append(@, currentTerm[s])]
    /\ UNCHANGED <<serverVars, snapIdx, commitIndex, messages, readSeq, ghostVars>>

\* support::append_effect_for: snapshot below nextIndex, else AppendEntries.
SendAppend(s, p) ==
    /\ role[s] = Leader
    /\ p # s
    /\ LET ni == nextIndex[s][p] IN
       IF snapIdx[s] > 0 /\ ni <= snapIdx[s]
       THEN Send([mtype |-> "IS", mterm |-> currentTerm[s], msource |-> s, mdest |-> p,
                  msnapshot |-> Prefix(log[s], snapIdx[s])])
       ELSE LET prevTerm == TermAt(s, ni - 1) IN
            Send([mtype |-> "AE", mterm |-> currentTerm[s], msource |-> s, mdest |-> p,
                  mprevIndex |-> ni - 1,
                  mprevTerm |-> IF prevTerm = Absent THEN 0 ELSE prevTerm,
                  mentries |-> SubSeq(log[s], ni, Min(Len(log[s]), ni + MaxMessageEntries - 1)),
                  mcommit |-> commitIndex[s]])
    /\ UNCHANGED <<serverVars, logVars, readSeq, ghostVars>>

IsCommitIndexCurrent(s) ==
    TermAt(s, commitIndex[s]) = currentTerm[s] \/ commitIndex[s] >= LastIndex(s)

\* client::handle_read, Linearizable mode -> begin_linearizable_read.
ReadStart(s) ==
    /\ role[s] = Leader
    /\ pendingRead[s] = NoRead
    /\ readSeq < MaxReads
    /\ Mutation = "read-without-current-commit" \/ IsCommitIndexCurrent(s)
    /\ readSeq' = readSeq + 1
    /\ pendingRead' = [pendingRead EXCEPT ![s] =
           [id |-> readSeq + 1, term |-> currentTerm[s], required |-> commitIndex[s],
            acks |-> {s}, mustObserve |-> CommittedMax]]
    /\ UNCHANGED <<currentTerm, votedFor, role, votesGranted, nextIndex, matchIndex,
                   logVars, messages, ghostVars>>

SendReadProbe(s, p) ==
    /\ pendingRead[s] # NoRead
    /\ p # s
    /\ Send([mtype |-> "RP", mterm |-> currentTerm[s], msource |-> s, mdest |-> p,
             mread |-> pendingRead[s].id, mrequired |-> pendingRead[s].required])
    /\ UNCHANGED <<serverVars, logVars, readSeq, ghostVars>>

\* client::handle_create_snapshot at the applied (== committed) boundary.
CreateSnapshot(s) ==
    /\ commitIndex[s] > snapIdx[s]
    /\ snapIdx' = [snapIdx EXCEPT ![s] = commitIndex[s]]
    /\ UNCHANGED <<serverVars, log, commitIndex, messages, readSeq, ghostVars>>

\* Process restart: durable hard state, log, snapshot and commit survive
\* (durable replay); every volatile field resets.
Restart(s) ==
    /\ role' = [role EXCEPT ![s] = Follower]
    /\ votesGranted' = [votesGranted EXCEPT ![s] = {}]
    /\ nextIndex' = [nextIndex EXCEPT ![s] = Cleared]
    /\ matchIndex' = [matchIndex EXCEPT ![s] = Cleared]
    /\ pendingRead' = [pendingRead EXCEPT ![s] = NoRead]
    /\ UNCHANGED <<currentTerm, votedFor, logVars, messages, readSeq, ghostVars>>

DropMessage(m) ==
    /\ messages' = messages \ {m}
    /\ UNCHANGED <<serverVars, logVars, readSeq, ghostVars>>

----
\* Message receipt. observe_higher_term runs first; an Err from the Rust
\* handler rejects the whole transition, so such a receipt is disabled here.

Higher(s, m) == m.mterm > currentTerm[s]
Term1(s, m) == Max(currentTerm[s], m.mterm)
Voted1(s, m) == IF Higher(s, m) THEN Nil ELSE votedFor[s]
Role1(s, m) == IF Higher(s, m) THEN Follower ELSE role[s]
Votes1(s, m) == IF Higher(s, m) THEN {} ELSE votesGranted[s]
Next1(s, m) == IF Higher(s, m) THEN Cleared ELSE nextIndex[s]
Match1(s, m) == IF Higher(s, m) THEN Cleared ELSE matchIndex[s]
Read1(s, m) == IF Higher(s, m) THEN NoRead ELSE pendingRead[s]

\* Apply observe_higher_term only.
ObserveOnly(s, m) ==
    /\ currentTerm' = [currentTerm EXCEPT ![s] = Term1(s, m)]
    /\ votedFor' = [votedFor EXCEPT ![s] = Voted1(s, m)]
    /\ role' = [role EXCEPT ![s] = Role1(s, m)]
    /\ votesGranted' = [votesGranted EXCEPT ![s] = Votes1(s, m)]
    /\ nextIndex' = [nextIndex EXCEPT ![s] = Next1(s, m)]
    /\ matchIndex' = [matchIndex EXCEPT ![s] = Match1(s, m)]
    /\ pendingRead' = [pendingRead EXCEPT ![s] = Read1(s, m)]

\* Higher-term observation followed by stepping down to follower in the
\* message term with the given vote (non-leader volatile state is canonical).
StepDownVoting(s, m, vote) ==
    /\ currentTerm' = [currentTerm EXCEPT ![s] = Term1(s, m)]
    /\ votedFor' = [votedFor EXCEPT ![s] = vote]
    /\ role' = [role EXCEPT ![s] = Follower]
    /\ votesGranted' = [votesGranted EXCEPT ![s] = {}]
    /\ nextIndex' = [nextIndex EXCEPT ![s] = Cleared]
    /\ matchIndex' = [matchIndex EXCEPT ![s] = Cleared]
    /\ pendingRead' = [pendingRead EXCEPT ![s] = NoRead]

StepDown(s, m) == StepDownVoting(s, m, Voted1(s, m))

UpToDate(s, m) ==
    \/ m.mlastTerm > LastTerm(s)
    \/ m.mlastTerm = LastTerm(s) /\ m.mlastIndex >= LastIndex(s)

\* election::handle_request_vote
HandleRequestVote(s, m, keep) ==
    LET t == Term1(s, m)
        granted == /\ m.mterm >= t
                   /\ Voted1(s, m) \in {Nil, m.msource}
                   /\ Mutation = "vote-without-up-to-date" \/ UpToDate(s, m)
    IN /\ IF granted
             THEN StepDownVoting(s, m, m.msource)
             ELSE ObserveOnly(s, m)
       /\ messages' = Put(Base(m, keep), [mtype |-> "RVR", mterm |-> t, msource |-> s,
                                           mdest |-> m.msource, mgranted |-> granted])
       /\ UNCHANGED <<logVars, readSeq, ghostVars>>

\* election::handle_vote_response and become_leader.
HandleVoteResponse(s, m, keep) ==
    LET t == Term1(s, m)
        votes == Votes1(s, m) \cup (IF m.mgranted THEN {m.msource} ELSE {})
    IN /\ messages' = Base(m, keep)
       /\ IF m.mterm < t \/ Role1(s, m) # Candidate
             THEN /\ ObserveOnly(s, m)
             ELSE IF Cardinality(votes) >= QuorumCount
             THEN /\ currentTerm' = [currentTerm EXCEPT ![s] = t]
                  /\ votedFor' = [votedFor EXCEPT ![s] = Voted1(s, m)]
                  /\ role' = [role EXCEPT ![s] = Leader]
                  /\ votesGranted' = [votesGranted EXCEPT ![s] = {}]
                  /\ nextIndex' = [nextIndex EXCEPT ![s] =
                         [p \in Server |-> IF p = s THEN 0 ELSE LastIndex(s) + 1]]
                  /\ matchIndex' = [matchIndex EXCEPT ![s] = Cleared]
                  /\ pendingRead' = [pendingRead EXCEPT ![s] = NoRead]
             ELSE /\ votesGranted' = [votesGranted EXCEPT ![s] = votes]
                  /\ UNCHANGED <<currentTerm, votedFor, role, nextIndex, matchIndex,
                                 pendingRead>>
       /\ UNCHANGED <<logVars, readSeq, committed, applied, reads>>

\* Entry at index i of the receiver as merge_entries sees it (compacted
\* entries are absent from the retained log).
Retained(s, i) == IF i > snapIdx[s] /\ i <= Len(log[s]) THEN log[s][i] ELSE Absent

AppendFailure(s, m, t) ==
    [mtype |-> "AER", mterm |-> t, msource |-> s, mdest |-> m.msource, msuccess |-> FALSE,
     mprevIndex |-> m.mprevIndex, mmatchIndex |-> LastIndex(s), mconflictIndex |-> LastIndex(s) + 1]

\* replication::handle_append_entries (with merge_entries and apply_leader_commit).
HandleAppendEntries(s, m, keep) ==
    LET t == Term1(s, m) IN
    IF m.mterm < t
    THEN /\ messages' = Put(Base(m, keep), AppendFailure(s, m, t))
         /\ UNCHANGED <<serverVars, logVars, readSeq, ghostVars>>
    ELSE
      /\ Role1(s, m) # Leader            \* "Raft observed two leaders in one term"
      /\ StepDown(s, m)
      /\ IF IF Mutation = "append-without-prev-match"
            THEN m.mprevIndex > Len(log[s])
            ELSE TermAt(s, m.mprevIndex) # m.mprevTerm
         THEN /\ messages' = Put(Base(m, keep), AppendFailure(s, m, t))
              /\ UNCHANGED <<logVars, readSeq, ghostVars>>
         ELSE
           LET entries == m.mentries
               n == Len(entries)
               changes == {k \in 1..n : Retained(s, m.mprevIndex + k) # entries[k]}
               first == IF changes = {} THEN 0
                        ELSE CHOOSE k \in changes : \A j \in changes : k <= j
               newLog == IF first = 0 THEN log[s]
                         ELSE Prefix(log[s], m.mprevIndex + first - 1)
                              \o SubSeq(entries, first, n)
               ack == m.mprevIndex + n
               newCommit == Min(Min(m.mcommit, ack), Len(newLog))
           IN /\ first = 0 \/ m.mprevIndex + first > commitIndex[s]   \* overwrite of a committed entry is an Err
              /\ log' = [log EXCEPT ![s] = newLog]
              /\ commitIndex' = [commitIndex EXCEPT ![s] = Max(@, newCommit)]
              /\ applied' = applied \cup {<<i, newLog[i]>> : i \in (commitIndex[s] + 1)..newCommit}
              /\ messages' = Put(Base(m, keep),
                     [mtype |-> "AER", mterm |-> t, msource |-> s, mdest |-> m.msource,
                      msuccess |-> TRUE, mprevIndex |-> m.mprevIndex, mmatchIndex |-> ack,
                      mconflictIndex |-> Len(newLog) + 1])
              /\ UNCHANGED <<snapIdx, readSeq, committed, reads>>

\* advance_leader_commit / highest_quorum_index over the post-response state.
QuorumIndex(s, match) ==
    LET mf(v) == IF v = s THEN LastIndex(s) ELSE match[v]
        ok(n) == /\ Mutation = "commit-previous-term" \/ TermAt(s, n) = currentTerm[s]
                 /\ Cardinality({v \in Server : mf(v) >= n}) >= QuorumCount
        candidates == {n \in (commitIndex[s] + 1)..LastIndex(s) : ok(n)}
    IN IF candidates = {} THEN commitIndex[s]
       ELSE CHOOSE n \in candidates : \A k \in candidates : k <= n

\* replication::handle_append_response
HandleAppendResponse(s, m, keep) ==
    LET t == Term1(s, m)
        src == m.msource
    IN
    IF m.mterm < t \/ Role1(s, m) # Leader
    THEN /\ ObserveOnly(s, m)
         /\ messages' = Base(m, keep)
         /\ UNCHANGED <<logVars, readSeq, ghostVars>>
    ELSE IF m.msuccess
    THEN
      /\ m.mmatchIndex >= m.mprevIndex /\ m.mmatchIndex <= LastIndex(s)  \* else Err
      /\ messages' = Base(m, keep)
      /\ IF m.mmatchIndex < matchIndex[s][src]
         THEN UNCHANGED <<serverVars, logVars, readSeq, ghostVars>>
         ELSE LET match == [matchIndex[s] EXCEPT ![src] = m.mmatchIndex]
                  n == QuorumIndex(s, match)
              IN /\ matchIndex' = [matchIndex EXCEPT ![s] = match]
                 /\ nextIndex' = [nextIndex EXCEPT ![s][src] = m.mmatchIndex + 1]
                 /\ commitIndex' = [commitIndex EXCEPT ![s] = n]
                 /\ committed' = committed \cup
                        {<<i, log[s][i], currentTerm[s]>> : i \in (commitIndex[s] + 1)..n}
                 /\ applied' = applied \cup {<<i, log[s][i]>> : i \in (commitIndex[s] + 1)..n}
                 /\ UNCHANGED <<currentTerm, votedFor, role, votesGranted, pendingRead,
                                log, snapIdx, readSeq, reads>>
    ELSE
      LET localNext == nextIndex[s][src]
          decremented == Max(localNext - 1, 1)
          retry == Max(1, Min(m.mconflictIndex, decremented))
      IN /\ messages' = Base(m, keep)
         /\ IF m.mprevIndex + 1 # localNext
            THEN UNCHANGED <<serverVars, logVars, readSeq, ghostVars>>
            ELSE /\ nextIndex' = [nextIndex EXCEPT ![s][src] = retry]
                 /\ UNCHANGED <<currentTerm, votedFor, role, votesGranted, matchIndex,
                                pendingRead, logVars, readSeq, ghostVars>>

\* snapshot::handle_install
HandleInstallSnapshot(s, m, keep) ==
    LET t == Term1(s, m)
        snap == m.msnapshot
        idx == Len(snap)
    IN
    IF m.mterm < t
    THEN /\ messages' = Put(Base(m, keep),
                [mtype |-> "ISR", mterm |-> t, msource |-> s, mdest |-> m.msource,
                 msnapIndex |-> idx, maccepted |-> FALSE])
         /\ UNCHANGED <<serverVars, logVars, readSeq, ghostVars>>
    ELSE
      /\ snap[idx] <= t                  \* snapshot term above leader term is an Err
      /\ Role1(s, m) # Leader            \* "two leaders in one term"
      /\ StepDown(s, m)
      /\ IF idx > commitIndex[s]
         THEN LET matching == TermAt(s, idx) = snap[idx]
                  newLog == IF matching THEN snap \o SubSeq(log[s], idx + 1, Len(log[s])) ELSE snap
              IN /\ log' = [log EXCEPT ![s] = newLog]
                 /\ snapIdx' = [snapIdx EXCEPT ![s] = idx]
                 /\ commitIndex' = [commitIndex EXCEPT ![s] = idx]
                 /\ applied' = applied \cup {<<i, snap[i]>> : i \in 1..idx}
         ELSE UNCHANGED <<logVars, applied>>
      /\ messages' = Put(Base(m, keep),
             [mtype |-> "ISR", mterm |-> t, msource |-> s, mdest |-> m.msource,
              msnapIndex |-> idx, maccepted |-> TRUE])
      /\ UNCHANGED <<readSeq, committed, reads>>

\* snapshot::handle_response
HandleSnapshotResponse(s, m, keep) ==
    LET t == Term1(s, m)
        src == m.msource
    IN /\ messages' = Base(m, keep)
       /\ IF \/ m.mterm < t \/ Role1(s, m) # Leader \/ ~m.maccepted
             \/ m.msnapIndex # snapIdx[s] \/ m.msnapIndex < matchIndex[s][src]
          THEN ObserveOnly(s, m)
          ELSE /\ matchIndex' = [matchIndex EXCEPT ![s][src] = m.msnapIndex]
               /\ nextIndex' = [nextIndex EXCEPT ![s][src] = m.msnapIndex + 1]
               /\ UNCHANGED <<currentTerm, votedFor, role, votesGranted, pendingRead>>
       /\ UNCHANGED <<logVars, readSeq, ghostVars>>

\* read::handle_probe. The required-index availability check is not modeled
\* as a guard: an Err only loses the probe, which the lossy network already
\* allows, so the model covers both the code with and without that check.
HandleReadProbe(s, m, keep) ==
    LET t == Term1(s, m) IN
    IF m.mterm < t
    THEN /\ messages' = Base(m, keep)
         /\ UNCHANGED <<serverVars, logVars, readSeq, ghostVars>>
    ELSE /\ Role1(s, m) # Leader          \* "two leaders in one term"
         /\ StepDown(s, m)
         /\ messages' = Put(Base(m, keep),
                [mtype |-> "RA", mterm |-> t, msource |-> s, mdest |-> m.msource, mread |-> m.mread])
         /\ UNCHANGED <<logVars, readSeq, ghostVars>>

\* read::handle_acknowledgement
HandleReadAck(s, m, keep) ==
    LET t == Term1(s, m)
        pending == pendingRead[s]
        acks == pending.acks \cup {m.msource}
    IN /\ messages' = Base(m, keep)
       /\ IF \/ m.mterm < t \/ Role1(s, m) # Leader
             \/ pending = NoRead \/ pending.id # m.mread \/ pending.term # t
          THEN /\ ObserveOnly(s, m)
               /\ UNCHANGED reads
          ELSE IF Cardinality(acks) >= QuorumCount
          THEN /\ pendingRead' = [pendingRead EXCEPT ![s] = NoRead]
               /\ reads' = reads \cup {[id |-> pending.id, leader |-> s, term |-> t,
                                        observed |-> pending.required,
                                        mustObserve |-> pending.mustObserve]}
               /\ UNCHANGED <<currentTerm, votedFor, role, votesGranted, nextIndex, matchIndex>>
          ELSE /\ pendingRead' = [pendingRead EXCEPT ![s].acks = acks]
               /\ UNCHANGED <<currentTerm, votedFor, role, votesGranted, nextIndex, matchIndex, reads>>
       /\ UNCHANGED <<logVars, readSeq, committed, applied>>

Receive(m, keep) ==
    LET s == m.mdest IN
    CASE m.mtype = "RV"  -> HandleRequestVote(s, m, keep)
      [] m.mtype = "RVR" -> HandleVoteResponse(s, m, keep)
      [] m.mtype = "AE"  -> HandleAppendEntries(s, m, keep)
      [] m.mtype = "AER" -> HandleAppendResponse(s, m, keep)
      [] m.mtype = "IS"  -> HandleInstallSnapshot(s, m, keep)
      [] m.mtype = "ISR" -> HandleSnapshotResponse(s, m, keep)
      [] m.mtype = "RP"  -> HandleReadProbe(s, m, keep)
      [] m.mtype = "RA"  -> HandleReadAck(s, m, keep)

----
Next ==
    \/ \E s \in Server :
        \/ Timeout(s) \/ ClientRequest(s) \/ ReadStart(s)
        \/ CreateSnapshot(s)
        \/ Restarts /\ Restart(s)
        \/ \E p \in Server : SendRequestVote(s, p) \/ SendAppend(s, p) \/ SendReadProbe(s, p)
    \/ \E m \in messages :
        \/ DropMessage(m)
        \/ \E keep \in (IF Duplicates THEN BOOLEAN ELSE {FALSE}) : Receive(m, keep)

Spec == Init /\ [][Next]_vars

Symmetry == Permutations(Server)

----
\* Safety properties (Raft paper Figure 3 and ReadIndex, dissertation 6.4).

\* At most one leader per term (no two servers lead the same term at once).
ElectionSafety ==
    \A s, t \in Server :
        role[s] = Leader /\ role[t] = Leader /\ currentTerm[s] = currentTerm[t] => s = t

\* Logs that agree at an index agree on the whole prefix through it.
LogMatching ==
    \A s, t \in Server :
        \A i \in 1..Min(Len(log[s]), Len(log[t])) :
            log[s][i] = log[t][i] => Prefix(log[s], i) = Prefix(log[t], i)

\* Every entry committed in a term no later than a leader's term is in that
\* leader's log at the same index.
LeaderCompleteness ==
    \A s \in Server : role[s] = Leader =>
        \A e \in committed : e[3] <= currentTerm[s] =>
            e[1] <= Len(log[s]) /\ log[s][e[1]] = e[2]

\* No two state machines (across servers, restarts, snapshot restores and
\* time) are ever handed different entries at one index.
StateMachineSafety ==
    \A a, b \in applied : a[1] = b[1] => a[2] = b[2]

\* A completed ReadIndex read observes every index committed (and therefore
\* acknowledged to a client) before the read started.
ReadIndexLinearizable ==
    \A r \in reads : r.observed >= r.mustObserve

\* Local durable structure (validate_replica_state).
CommitWithinLog ==
    \A s \in Server : snapIdx[s] <= commitIndex[s] /\ commitIndex[s] <= Len(log[s])
====
