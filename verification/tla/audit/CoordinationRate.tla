---- MODULE CoordinationRate ----
\* Bounded TLA+ model of the coordination rate-limit primitive (issue #57).
\* It is a model, not a live-engine proof or a production profile.
\*
\* Source mapping (brittonr/aspen at 8ae93a5e5):
\*   Acquire  src/coordination/parts/mod/p005/body.rs prepare_rate_acquire
\*            (denied when state.rates[key] >= manifest.rate_limit; otherwise +1)
\*   No code decrements or clears state.rates, and the manifest has no window.
\*
\* Windowed = TRUE models the suggested fix: the count resets when the logical
\* window advances (fabric_time logical ticks keep replay deterministic).
EXTENDS Naturals

CONSTANTS RateLimit, WindowTicks, Windowed

VARIABLES used, tick, denied

vars == <<used, tick, denied>>

TypeOK == used \in 0..RateLimit /\ tick \in 0..(WindowTicks - 1) /\ denied \in BOOLEAN

Init == used = 0 /\ tick = 0 /\ denied = FALSE

Acquire ==
    IF used < RateLimit
        THEN used' = used + 1 /\ denied' = FALSE /\ UNCHANGED tick
        ELSE denied' = TRUE /\ UNCHANGED <<used, tick>>

\* Logical time advances; tick is kept modulo the window to stay finite.
Tick ==
    /\ tick' = (tick + 1) % WindowTicks
    /\ used' = IF Windowed /\ tick' = 0 THEN 0 ELSE used
    /\ UNCHANGED denied

Next == Acquire \/ Tick

Spec == Init /\ [][Next]_vars /\ WF_vars(Acquire) /\ WF_vars(Tick)

\* A rate limit admits work again once time passes.
AlwaysEventuallyAdmits == []<>(~denied)
====
