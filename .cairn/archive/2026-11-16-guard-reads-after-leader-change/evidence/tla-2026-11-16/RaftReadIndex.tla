---- MODULE RaftReadIndex ----
\* Bounded TLA+ model of the live Raft linearizable read path (issue #44).
\* It is a model, not a live-engine proof or a production profile.
\*
\* Source mapping (brittonr/aspen at 8ae93a5e5):
\*   BecomeLeader    src/fabric_consistency/raft/transition/election.rs  become_leader
\*   ReadStart       src/fabric_consistency/raft/transition/client.rs    handle_read, begin_linearizable_read
\*   ReadAck         src/fabric_consistency/raft/transition/message/read.rs handle_probe, handle_acknowledgement
\*   AdvanceCommit   src/fabric_consistency/raft/transition/message/replication.rs highest_quorum_index
\*   AppendEntries   replication.rs handle_append_entries (whole-suffix delivery abstraction)
\*
\* Abstractions: each RPC is one atomic step between two nodes; AppendEntries
\* delivers the leader's whole log and its commit index at send time. Messages
\* are not lost or reordered. These abstractions only remove behaviours, so a
\* counterexample here is also a behaviour of the message-passing protocol.
\*
\* ReadGuard = TRUE adds the Raft section 6.4 rule: serve a ReadIndex read only
\* after the leader has committed an entry in its current term.
\* NoopOnElection = TRUE appends a no-op entry when a leader is elected.
\* CommittedLogException = TRUE also serves a read when the leader's commit index
\* already reaches its last log index; ExceptionSlack > 0 is a deliberately wrong
\* mutant of that exception, used to show the property detects a loose bound.
EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS
    Nodes,
    MaxTerm,
    MaxLogLen,
    MaxReads,
    ReadGuard,
    NoopOnElection,
    CommittedLogException,
    ExceptionSlack

Nil == "nil"

VARIABLES
    currentTerm,
    votedFor,
    role,
    log,
    commitIndex,
    matchIndex,
    votes,
    acked,
    reads

vars == <<currentTerm, votedFor, role, log, commitIndex, matchIndex, votes, acked, reads>>

Quorums == {Q \in SUBSET Nodes : 2 * Cardinality(Q) > Cardinality(Nodes)}
IsQuorum(S) == 2 * Cardinality(S) > Cardinality(Nodes)

Entry == [term : 1..MaxTerm, kind : {"write", "noop"}]
Max(S) == IF S = {} THEN 0 ELSE CHOOSE x \in S : \A y \in S : x >= y
LastIndex(n) == Len(log[n])
LastTerm(n) == IF log[n] = <<>> THEN 0 ELSE log[n][Len(log[n])].term
TermAt(n, i) == IF i = 0 THEN 0 ELSE log[n][i].term

Read == [node : Nodes, term : 1..MaxTerm, requiredIndex : 0..MaxLogLen,
         mustObserve : 0..MaxLogLen, acks : SUBSET Nodes, done : BOOLEAN,
         viaException : BOOLEAN]

TypeOK ==
    /\ currentTerm \in [Nodes -> 0..MaxTerm]
    /\ votedFor \in [Nodes -> Nodes \cup {Nil}]
    /\ role \in [Nodes -> {"follower", "candidate", "leader"}]
    /\ \A n \in Nodes : log[n] \in Seq(Entry) /\ Len(log[n]) <= MaxLogLen
    /\ commitIndex \in [Nodes -> 0..MaxLogLen]
    /\ matchIndex \in [Nodes -> [Nodes -> 0..MaxLogLen]]
    /\ votes \in [Nodes -> SUBSET Nodes]
    /\ acked \subseteq 1..MaxLogLen
    /\ reads \subseteq Read

Init ==
    /\ currentTerm = [n \in Nodes |-> 0]
    /\ votedFor = [n \in Nodes |-> Nil]
    /\ role = [n \in Nodes |-> "follower"]
    /\ log = [n \in Nodes |-> <<>>]
    /\ commitIndex = [n \in Nodes |-> 0]
    /\ matchIndex = [n \in Nodes |-> [m \in Nodes |-> 0]]
    /\ votes = [n \in Nodes |-> {}]
    /\ acked = {}
    /\ reads = {}

\* election.rs handle_timeout: new term, vote for self.
Timeout(n) ==
    /\ role[n] # "leader"
    /\ currentTerm[n] < MaxTerm
    /\ currentTerm' = [currentTerm EXCEPT ![n] = @ + 1]
    /\ votedFor' = [votedFor EXCEPT ![n] = n]
    /\ role' = [role EXCEPT ![n] = "candidate"]
    /\ votes' = [votes EXCEPT ![n] = {n}]
    /\ UNCHANGED <<log, commitIndex, matchIndex, acked, reads>>

\* election.rs handle_request_vote + handle_vote_response, one granted vote.
GrantVote(c, v) ==
    /\ c # v
    /\ role[c] = "candidate"
    /\ currentTerm[v] <= currentTerm[c]
    /\ \/ currentTerm[v] < currentTerm[c]
       \/ votedFor[v] \in {Nil, c}
    /\ \/ LastTerm(c) > LastTerm(v)
       \/ (LastTerm(c) = LastTerm(v) /\ LastIndex(c) >= LastIndex(v))
    /\ currentTerm' = [currentTerm EXCEPT ![v] = currentTerm[c]]
    /\ votedFor' = [votedFor EXCEPT ![v] = c]
    /\ role' = [role EXCEPT ![v] = "follower"]
    /\ votes' = [votes EXCEPT ![c] = @ \cup {v}]
    /\ UNCHANGED <<log, commitIndex, matchIndex, acked, reads>>

\* election.rs become_leader: no entry is appended in the current code.
BecomeLeader(c) ==
    /\ role[c] = "candidate"
    /\ IsQuorum(votes[c])
    /\ \A other \in Nodes : ~(role[other] = "leader" /\ currentTerm[other] = currentTerm[c])
    /\ role' = [role EXCEPT ![c] = "leader"]
    /\ IF NoopOnElection /\ LastIndex(c) < MaxLogLen
          THEN log' = [log EXCEPT ![c] = Append(@, [term |-> currentTerm[c], kind |-> "noop"])]
          ELSE UNCHANGED log
    /\ matchIndex' = [matchIndex EXCEPT ![c] = [m \in Nodes |-> 0]]
    /\ UNCHANGED <<currentTerm, votedFor, commitIndex, votes, acked, reads>>

\* client.rs handle_proposal / append_proposal.
ClientWrite(l) ==
    /\ role[l] = "leader"
    /\ LastIndex(l) < MaxLogLen
    /\ log' = [log EXCEPT ![l] = Append(@, [term |-> currentTerm[l], kind |-> "write"])]
    /\ UNCHANGED <<currentTerm, votedFor, role, commitIndex, matchIndex, votes, acked, reads>>

\* replication.rs handle_append_entries + handle_append_response, whole-log delivery.
AppendEntries(l, f) ==
    /\ l # f
    /\ role[l] = "leader"
    /\ currentTerm[f] <= currentTerm[l]
    /\ currentTerm' = [currentTerm EXCEPT ![f] = currentTerm[l]]
    /\ votedFor' = IF currentTerm[f] < currentTerm[l] THEN [votedFor EXCEPT ![f] = Nil] ELSE votedFor
    /\ role' = [role EXCEPT ![f] = "follower"]
    /\ log' = [log EXCEPT ![f] = log[l]]
    /\ commitIndex' = [commitIndex EXCEPT ![f] = Max({@, commitIndex[l]})]
    /\ matchIndex' = [matchIndex EXCEPT ![l][f] = LastIndex(l)]
    /\ UNCHANGED <<votes, acked, reads>>

MatchFor(l, m) == IF m = l THEN LastIndex(l) ELSE matchIndex[l][m]

\* replication.rs advance_leader_commit: current-term entries only; committed
\* write entries are acknowledged to clients (ProposalOutcome Committed).
AdvanceCommit(l) ==
    /\ role[l] = "leader"
    /\ \E i \in (commitIndex[l] + 1)..LastIndex(l) :
          /\ TermAt(l, i) = currentTerm[l]
          /\ IsQuorum({m \in Nodes : MatchFor(l, m) >= i})
          /\ commitIndex' = [commitIndex EXCEPT ![l] = i]
          /\ acked' = acked \cup {j \in (commitIndex[l] + 1)..i : log[l][j].kind = "write"}
    /\ UNCHANGED <<currentTerm, votedFor, role, log, matchIndex, votes, reads>>

\* client.rs handle_read / begin_linearizable_read. mustObserve records every
\* write already acknowledged to a client when the read starts (real time).
ReadStart(l) ==
    /\ role[l] = "leader"
    /\ Cardinality(reads) < MaxReads
    /\ ReadGuard => \/ TermAt(l, commitIndex[l]) = currentTerm[l]
                    \/ CommittedLogException /\ commitIndex[l] + ExceptionSlack >= LastIndex(l)
    /\ reads' = reads \cup {[node |-> l, term |-> currentTerm[l], requiredIndex |-> commitIndex[l],
                             mustObserve |-> Max(acked), acks |-> {l}, done |-> FALSE,
                             viaException |-> TermAt(l, commitIndex[l]) # currentTerm[l]]}
    /\ UNCHANGED <<currentTerm, votedFor, role, log, commitIndex, matchIndex, votes, acked>>

\* read.rs handle_probe at the follower + handle_acknowledgement at the leader.
ReadAck(r, f) ==
    /\ r \in reads
    /\ ~r.done
    /\ f \notin r.acks
    /\ role[r.node] = "leader" /\ currentTerm[r.node] = r.term
    /\ currentTerm[f] <= r.term
    /\ r.requiredIndex <= LastIndex(f)
    /\ currentTerm' = [currentTerm EXCEPT ![f] = r.term]
    /\ votedFor' = IF currentTerm[f] < r.term THEN [votedFor EXCEPT ![f] = Nil] ELSE votedFor
    /\ role' = [role EXCEPT ![f] = "follower"]
    /\ reads' = (reads \ {r}) \cup {[r EXCEPT !.acks = @ \cup {f}]}
    /\ UNCHANGED <<log, commitIndex, matchIndex, votes, acked>>

\* read.rs handle_acknowledgement: quorum reached, ReadOutcome Current at requiredIndex.
ReadComplete(r) ==
    /\ r \in reads
    /\ ~r.done
    /\ IsQuorum(r.acks)
    /\ role[r.node] = "leader" /\ currentTerm[r.node] = r.term
    /\ reads' = (reads \ {r}) \cup {[r EXCEPT !.done = TRUE]}
    /\ UNCHANGED <<currentTerm, votedFor, role, log, commitIndex, matchIndex, votes, acked>>

Next ==
    \/ \E n \in Nodes : Timeout(n) \/ BecomeLeader(n) \/ ClientWrite(n) \/ AdvanceCommit(n) \/ ReadStart(n)
    \/ \E a, b \in Nodes : GrantVote(a, b) \/ AppendEntries(a, b)
    \/ \E r \in reads : ReadComplete(r) \/ \E f \in Nodes : ReadAck(r, f)

Spec == Init /\ [][Next]_vars

\* Sanity invariant: the abstraction behaves like Raft. (Election safety is
\* enforced directly by BecomeLeader's guard, so it is not checked separately.)
CommittedPrefixAgreement ==
    \A a, b \in Nodes : \A i \in 1..MaxLogLen :
        (i <= commitIndex[a] /\ i <= commitIndex[b]) => log[a][i] = log[b][i]

\* The property under test: a completed linearizable read observes every write
\* acknowledged before the read began.
ReadLinearizable ==
    \A r \in reads : r.done => r.requiredIndex >= r.mustObserve
\* Witness (expected to be VIOLATED when the scenario is reachable): no read
\* completes that must observe a write acknowledged from an earlier term. Used to
\* show that a passing configuration actually exercises post-election reads.
NoPostElectionRead ==
    ~(\E r \in reads :
        /\ r.done
        /\ \E i \in acked :
              /\ i <= r.mustObserve
              /\ i <= Len(log[r.node])
              /\ log[r.node][i].term < r.term)
\* Witness (expected to be VIOLATED): no completed read admitted only by the
\* committed-log exception must observe a write acknowledged in an earlier term.
NoExceptionPostElectionRead ==
    ~(\E r \in reads :
        /\ r.done
        /\ r.viaException
        /\ \E i \in acked :
              /\ i <= r.mustObserve
              /\ i <= Len(log[r.node])
              /\ log[r.node][i].term < r.term)
====
