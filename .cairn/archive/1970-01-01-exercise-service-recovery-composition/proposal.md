# Proposal: Exercise a service recovery composition end to end

## Why

The restart-fencing, time-sliced execution, recovery-group, and facade changes each carry focused tests. None of them proves the properties that matter when the pieces work together under failure. The strongest validation is one small composition, not another isolated happy-path fixture: a durable stateful service, a dependent worker, and an unrelated sibling, exercised through crashes, delayed old events, CPU saturation, queue overload, restart storms, uncertain effects, and an implementation upgrade.

## What Changes

- Add a deterministic composition fixture that composes the three services through the facade profiles and recovery-group declarations, exercising every failure class above and asserting the explicit properties:
  - unaffected services continue while a member fails and recovers;
  - stale instances lose influence (delayed timers, callback completions, and failure notifications from a pre-restart instance cannot act);
  - durable logical work is not silently lost across restart and is readmitted under a current claim;
  - uncertain external effects are surfaced as unknown outcomes and not blindly repeated;
  - restart storms terminate or escalate within the admitted window.
- Add the live-process twin of the same composition through the real-process harness track, keeping the deterministic and live observations comparable.
- Run the CPU-saturation case under the time-sliced execution profile so execution fairness is exercised in composition, not in isolation.
- Exercise one generation upgrade (drain, checkpoint, admitted state transformation, activation, stale-work rejection) inside the composition.

## Impact

- **Files**: a new composition fixture module in the deterministic harness and a matching real-process case, plus shared service definitions.
- **Testing**: the composition IS the test; each property is a named assertion with positive and negative variants (for example, a deliberately fencing-broken variant must fail the stale-instance property).
- **Non-goals**: no new runtime features; failures to satisfy a property file issues against the owning change rather than patching inside this one. No operator inspection view or upgrade-workflow UI; the upgrade case exercises existing admission contracts only.

## Dependencies

- `fence-runtime-local-work-across-restarts`, `admit-time-sliced-component-execution`, `admit-recovery-group-supervision`, `admit-typed-service-facades`.
- `exercise-node-processes-under-faults` for the real-process track conventions.
