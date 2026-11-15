---- MODULE WorldReleaseAttempt ----
\* Bounded TLA+ model of one world-promotion release reservation and its
\* dispatch attempts (issues #54 and #55).
\* It is a model, not a live-engine proof or a production profile.
\*
\* Source mapping (brittonr/aspen at 8ae93a5e5):
\*   Claim            src/world_promotion/store/outbox.rs claim_reservation
\*   BeginDispatch    src/world_promotion/service.rs dispatch_world_reservation (store Attempting)
\*   DispatchReturns  service.rs + crates/molten-core/src/world_promotion/dispatch.rs classify_attempt_observation
\*   DispatchErrors   service.rs `ports.dispatcher.dispatch(&dispatch)?`, or a crash after Attempting is stored
\*   Acknowledge      dispatch.rs acknowledge_attempt
\*   Abandon          dispatch.rs abandon_uncertain_attempt (requires Uncertain)
\*   Reconcile        store/outbox.rs allowed_transition (Uncertain -> Reconciled)
\*   RecordEffect     crates/molten-core/src/world_promotion/observation.rs plan_world_promotion_observation_commit
\*
\* State changes follow the store's transition table (outbox.rs allowed_transition),
\* which has no edge from Uncertain back to Claimed or Attempting, so retry is not
\* modelled; plan_world_retry only produces a plan today. Blocked and Denied
\* (admission failure before dispatch) are omitted.
\* No production code implements WorldEffectDispatcherPort or calls
\* acknowledge_attempt today, so both findings are latent.
\*
\* CarryOutcome = TRUE models the #54 fix: the attempt record keeps succeeded
\* versus failed and only a succeeded attempt can be acknowledged as completion.
\* ErrorBecomesUncertain = TRUE models the #55 fix: a dispatcher error or a
\* restart that finds an Attempting record moves it to Uncertain.
EXTENDS Naturals, TLC

CONSTANTS
    MaxAttempts,
    CarryOutcome,
    ErrorBecomesUncertain

States == {"Committed", "Claimed", "Attempting", "Observed", "Acknowledged",
           "Uncertain", "Conflict", "Abandoned", "Reconciled", "Recorded"}
\* States from which no further action is needed or possible by design.
Terminal == {"Conflict", "Abandoned", "Reconciled", "Recorded"}

VARIABLES
    state,          \* reservation/attempt state (the shell keeps them equal)
    outcome,        \* outcome stored in the attempt record: "none", "succeeded", "failed"
    world,          \* what actually happened externally: "none", "applied", "failed"
    attempts,
    inFlight,       \* TRUE while one dispatcher call is outstanding
    recordedApplied \* TRUE once the effect is recorded as completed in world history

vars == <<state, outcome, world, attempts, inFlight, recordedApplied>>

TypeOK ==
    /\ state \in States
    /\ outcome \in {"none", "succeeded", "failed"}
    /\ world \in {"none", "applied", "failed"}
    /\ attempts \in 0..MaxAttempts
    /\ inFlight \in BOOLEAN
    /\ recordedApplied \in BOOLEAN

Init ==
    /\ state = "Committed"
    /\ outcome = "none"
    /\ world = "none"
    /\ attempts = 0
    /\ inFlight = FALSE
    /\ recordedApplied = FALSE

Claim ==
    /\ state = "Committed"
    /\ state' = "Claimed"
    /\ UNCHANGED <<outcome, world, attempts, inFlight, recordedApplied>>

BeginDispatch ==
    /\ state = "Claimed"
    /\ attempts < MaxAttempts
    /\ state' = "Attempting"
    /\ attempts' = attempts + 1
    /\ inFlight' = TRUE
    /\ UNCHANGED <<outcome, world, recordedApplied>>

\* The external system may apply the effect, fail it, or do nothing, independent
\* of what the dispatcher reports back.
ExternalEffect ==
    /\ inFlight
    /\ world = "none"
    /\ world' \in {"applied", "failed"}
    /\ UNCHANGED <<state, outcome, attempts, inFlight, recordedApplied>>

\* Dispatcher returns an observation consistent with the world.
DispatchReturns ==
    /\ inFlight
    /\ inFlight' = FALSE
    /\ \/ /\ world = "applied"
          /\ state' = "Observed"
          /\ outcome' = IF CarryOutcome THEN "succeeded" ELSE "none"
       \/ /\ world = "failed"
          \* classify_attempt_observation maps Failed to Observed as well.
          /\ state' = "Observed"
          /\ outcome' = IF CarryOutcome THEN "failed" ELSE "none"
       \/ /\ state' = "Uncertain"   \* Unknown or Duplicate observation
          /\ UNCHANGED outcome
       \/ /\ state' = "Conflict"
          /\ UNCHANGED outcome
    /\ UNCHANGED <<world, attempts, recordedApplied>>

\* Dispatcher returns Err (possibly after the effect happened) or the process
\* restarts after storing Attempting. Current code leaves the record Attempting.
DispatchErrors ==
    /\ inFlight
    /\ inFlight' = FALSE
    /\ state' = IF ErrorBecomesUncertain THEN "Uncertain" ELSE "Attempting"
    /\ UNCHANGED <<outcome, world, attempts, recordedApplied>>

Acknowledge ==
    /\ state = "Observed"
    /\ CarryOutcome => outcome = "succeeded"
    /\ state' = "Acknowledged"
    /\ UNCHANGED <<outcome, world, attempts, inFlight, recordedApplied>>

\* With the fix, a failed observation is a terminal, non-completed outcome.
CloseFailed ==
    /\ CarryOutcome
    /\ state = "Observed"
    /\ outcome = "failed"
    /\ state' = "Abandoned"
    /\ UNCHANGED <<outcome, world, attempts, inFlight, recordedApplied>>

\* An operator reconciles an uncertain attempt against an external observation.
Reconcile ==
    /\ state = "Uncertain"
    /\ state' = "Reconciled"
    /\ UNCHANGED <<outcome, world, attempts, inFlight, recordedApplied>>

Abandon ==
    /\ state = "Uncertain"
    /\ state' = "Abandoned"
    /\ UNCHANGED <<outcome, world, attempts, inFlight, recordedApplied>>

RecordEffect ==
    /\ state = "Acknowledged"
    /\ state' = "Recorded"
    /\ recordedApplied' = TRUE
    /\ UNCHANGED <<outcome, world, attempts, inFlight>>

Next ==
    \/ Claim \/ BeginDispatch \/ ExternalEffect \/ DispatchReturns \/ DispatchErrors
    \/ Acknowledge \/ CloseFailed \/ Reconcile \/ Abandon \/ RecordEffect

\* Operators and shells eventually take every enabled step. An outstanding
\* dispatcher call eventually returns or errors (it is bounded by a timeout).
Fairness ==
    /\ WF_vars(Claim) /\ WF_vars(BeginDispatch) /\ WF_vars(DispatchReturns \/ DispatchErrors)
    /\ WF_vars(Acknowledge) /\ WF_vars(CloseFailed) /\ WF_vars(Abandon) /\ WF_vars(RecordEffect)

Spec == Init /\ [][Next]_vars /\ Fairness

\* #54: world history records a completed effect only if it was applied.
RecordedOnlyIfApplied == recordedApplied => world = "applied"

\* #55: from every reachable state some step remains, unless the reservation is terminal.
NoStuckReservation == (state \notin Terminal) => ENABLED Next

\* #55 as liveness: every reservation eventually reaches a terminal state.
EventuallyTerminal == <>(state \in Terminal)
====
