------------------------------ MODULE trust_init ------------------------------
\* Trust initialization protocol for Aspen cluster secret sharing.
\*
\* Models the initial cluster secret creation and share distribution:
\* 1. Leader generates a cluster secret
\* 2. Leader splits into N shares with threshold K
\* 3. Shares are distributed to nodes via Raft log entry
\* 4. Each node stores its share and ACKs
\*
\* Safety: After init completes, every member has exactly one share.
\* The secret can be reconstructed from any K shares.

EXTENDS Integers, FiniteSets, Sequences

CONSTANTS
    N,          \* Number of nodes
    K,          \* Reconstruction threshold
    NODES       \* Set of node IDs (e.g., {1, 2, 3})

VARIABLES
    leader,         \* The coordinator node (CHOOSE from NODES)
    leader_state,   \* "idle" | "created" | "distributing" | "done"
    node_share,     \* node_share[n] = share for node n (0 = none, 1..N = share index)
    node_acked,     \* node_acked[n] = TRUE if node acknowledged receiving share
    msgs            \* Set of messages in flight

vars == <<leader, leader_state, node_share, node_acked, msgs>>

\* Message types
DistributeMsg(src, dst, share_idx) == [type |-> "distribute", src |-> src, dst |-> dst, share |-> share_idx]
AckMsg(src, dst) == [type |-> "ack", src |-> src, dst |-> dst]

---------------------------------------------------------------------------
\* Type invariant
TypeOK ==
    /\ leader \in NODES
    /\ leader_state \in {"idle", "created", "distributing", "done"}
    /\ \A n \in NODES : node_share[n] \in 0..N
    /\ \A n \in NODES : node_acked[n] \in BOOLEAN

\* Safety: After init completes, every member has a share
InvAllNodesHaveShares ==
    leader_state = "done" => \A n \in NODES : node_share[n] > 0

\* Safety: No two nodes have the same share index
InvSharesUnique ==
    \A m, n \in NODES : (m # n /\ node_share[m] > 0 /\ node_share[n] > 0) => node_share[m] # node_share[n]

\* Safety: Leader only moves to "done" after all nodes ACKed
InvDoneImpliesAllAcked ==
    leader_state = "done" => \A n \in NODES : node_acked[n] = TRUE

---------------------------------------------------------------------------
\* Initial state
Init ==
    /\ leader = CHOOSE n \in NODES : TRUE
    /\ leader_state = "idle"
    /\ node_share = [n \in NODES |-> 0]
    /\ node_acked = [n \in NODES |-> FALSE]
    /\ msgs = {}

\* Leader creates the secret and prepares shares
LeaderCreateSecret ==
    /\ leader_state = "idle"
    /\ leader_state' = "created"
    /\ UNCHANGED <<leader, node_share, node_acked, msgs>>

\* Leader distributes shares to all nodes (via Raft log apply)
LeaderDistributeShares ==
    /\ leader_state = "created"
    /\ leader_state' = "distributing"
    /\ LET share_assignment == CHOOSE f \in [NODES -> 1..N] : \A m, n \in NODES : m # n => f[m] # f[n]
       IN msgs' = msgs \cup {DistributeMsg(leader, n, share_assignment[n]) : n \in NODES}
    /\ UNCHANGED <<leader, node_share, node_acked>>

\* A node receives its share
NodeReceiveShare(n) ==
    /\ \E msg \in msgs :
        /\ msg.type = "distribute"
        /\ msg.dst = n
        /\ node_share[n] = 0  \* Haven't received yet
        /\ node_share' = [node_share EXCEPT ![n] = msg.share]
        /\ msgs' = (msgs \ {msg}) \cup {AckMsg(n, leader)}
    /\ UNCHANGED <<leader, leader_state, node_acked>>

\* Leader receives an ACK
LeaderReceiveAck ==
    /\ leader_state = "distributing"
    /\ \E msg \in msgs :
        /\ msg.type = "ack"
        /\ msg.dst = leader
        /\ node_acked' = [node_acked EXCEPT ![msg.src] = TRUE]
        /\ msgs' = msgs \ {msg}
    /\ UNCHANGED <<leader, leader_state, node_share>>

\* Leader marks init as complete when all ACKs received
LeaderComplete ==
    /\ leader_state = "distributing"
    /\ \A n \in NODES : node_acked[n] = TRUE
    /\ leader_state' = "done"
    /\ UNCHANGED <<leader, node_share, node_acked, msgs>>

---------------------------------------------------------------------------
\* Terminal stutter: protocol is complete, no more steps
Done ==
    /\ leader_state = "done"
    /\ UNCHANGED vars

\* Next state relation
Next ==
    \/ LeaderCreateSecret
    \/ LeaderDistributeShares
    \/ \E n \in NODES : NodeReceiveShare(n)
    \/ LeaderReceiveAck
    \/ LeaderComplete
    \/ Done

\* Fairness: all actions eventually execute if enabled
Fairness == WF_vars(Next)

Spec == Init /\ [][Next]_vars /\ Fairness

\* Liveness: init eventually completes
LivenessInitCompletes == <>(leader_state = "done")

=============================================================================
