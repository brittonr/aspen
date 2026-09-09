# System Extension Runtime Delta

## ADDED Requirements

### Requirement: Recovery groups plan ordered recovery
r[molten.system_extension.recovery_groups] Aspen MUST admit an optional recovery-group declaration per extension that binds members, an explicit ordering, per-member restart classes, and shutdown bounds. A failure in one member MUST plan recovery only for the members the group declaration identifies, in the declared order, leaving unaffiliated services untouched. The recovery plan MUST be computed by the pure core and executed by the shell through existing lifecycle ports.

#### Scenario: Dependent worker follows its dependency
- GIVEN a recovery group orders a session manager before a dependent subscription worker, with an independent content verifier outside the group
- WHEN the session manager fails with a retryable failure under budget
- THEN the plan restarts the session manager and then the subscription worker, and the verifier keeps running.

#### Scenario: Unaffiliated failure stays contained
- GIVEN the same composition
- WHEN the subscription worker fails
- THEN the plan does not tear down the session manager unless the group declaration says so, and the verifier keeps running.

#### Scenario: Completed task is not restarted
- GIVEN a member declared temporary or transient exits successfully
- WHEN supervision plans recovery
- THEN the member transitions to stopped without consuming restart budget, and only an abnormal transient exit consumes budget.

### Requirement: Restart budgets are time-windowed and survive supervisor replacement
r[molten.system_extension.restart_rate_budget] Restart budgets MUST combine a bounded per-member restart history, an explicit time window backed by admitted elapsed observations, backoff, and a group-level budget that a supervisor replacement MUST NOT reset. When the windowed budget is exhausted, supervision MUST quarantine or escalate deterministically. Replay MUST consume recorded observations or an explicitly equivalent virtual-time profile.

#### Scenario: Restart storm terminates
- GIVEN a member with a windowed budget of N restarts
- WHEN the member fails N plus one times inside the window
- THEN supervision quarantines or escalates instead of restarting, and the decision names the exhausted budget.

#### Scenario: Supervisor replacement cannot hide a storm
- GIVEN a group consumed most of its windowed budget before its supervisor was replaced
- WHEN the replacement supervisor faces further failures inside the same window
- THEN the accumulated group budget still applies and the storm still terminates.
