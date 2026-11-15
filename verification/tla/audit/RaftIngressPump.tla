---- MODULE RaftIngressPump ----
\* Bounded TLA+ model of the live Raft ingress pump and the reliability host
\* loop (issue #59).
\* It is a model, not a live-engine proof or a production profile.
\*
\* Source mapping (brittonr/aspen at 8ae93a5e5):
\*   Deliver, Malformed  src/fabric_consistency/raft/iroh/mod.rs ReplicaIngressPump::spawn
\*                       (`while delivered < delivery_limit`, `result?` on poll errors,
\*                        parse_replica_message failure is an Err)
\*   Host exit           src/reliability/service/host/runtime.rs serve_bound
\*                       (`event = self.ingress.next() => Incoming::Peer(event?.event)`)
\*
\* LifetimeLimit = TRUE: delivery_limit counts every message over the pump's life.
\* StopOnMalformed = TRUE: one undecodable frame ends the pump.
\* The fix sets both FALSE: a per-window budget and drop-and-log for bad frames.
EXTENDS Naturals

CONSTANTS DeliveryLimit, LifetimeLimit, StopOnMalformed

VARIABLES delivered, pump, host

vars == <<delivered, pump, host>>

TypeOK ==
    /\ delivered \in 0..DeliveryLimit
    /\ pump \in {"running", "stopped"}
    /\ host \in {"serving", "exited"}

Init == delivered = 0 /\ pump = "running" /\ host = "serving"

Deliver ==
    /\ pump = "running"
    /\ delivered < DeliveryLimit
    /\ delivered' = delivered + 1
    /\ UNCHANGED <<pump, host>>

\* Without a lifetime limit the budget is per window and refills.
Refill ==
    /\ ~LifetimeLimit
    /\ delivered = DeliveryLimit
    /\ delivered' = 0
    /\ UNCHANGED <<pump, host>>

Exhausted ==
    /\ LifetimeLimit
    /\ pump = "running"
    /\ delivered = DeliveryLimit
    /\ pump' = "stopped"
    /\ UNCHANGED <<delivered, host>>

Malformed ==
    /\ pump = "running"
    /\ pump' = IF StopOnMalformed THEN "stopped" ELSE "running"
    /\ UNCHANGED <<delivered, host>>

\* The host loop propagates the pump's termination with `?` and exits.
HostObservesStop ==
    /\ pump = "stopped"
    /\ host = "serving"
    /\ host' = "exited"
    /\ UNCHANGED <<delivered, pump>>

Next == Deliver \/ Refill \/ Exhausted \/ Malformed \/ HostObservesStop

\* Peers keep sending well-formed traffic; a malformed frame may arrive but is
\* not required to.
Spec == Init /\ [][Next]_vars
        /\ WF_vars(Deliver) /\ WF_vars(Refill) /\ WF_vars(Exhausted) /\ WF_vars(HostObservesStop)

\* A long-running replica keeps receiving peer traffic.
KeepsServing == []<>(pump = "running" /\ host = "serving")

\* Safety form for the malformed-frame case.
NeverExits == host = "serving"
====
