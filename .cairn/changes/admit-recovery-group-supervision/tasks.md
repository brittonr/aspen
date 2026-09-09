# Tasks: Admit recovery-group supervision

## Group admission

- [ ] [serial] Extend the admitted manifest with the optional recovery-group declaration (members, ordering, restart classes, window, backoff, budget) and reject cycles, duplicates, unknown children, and windows without an elapsed-time source. r[molten.system_extension.recovery_groups]
- [ ] [parallel] Add negative admission fixtures for each rejected declaration shape and a positive fixture for a valid ordered group. r[molten.system_extension.recovery_groups]

## Recovery planning

- [ ] [serial] Implement the pure `plan_recovery` over failure, group declaration, bounded restart history, and admitted elapsed observations, producing an ordered stop/restart/quarantine/escalate plan; keep the one-member group behavior equivalent to the current `plan_supervision`. r[molten.system_extension.recovery_groups] r[molten.system_extension.restart_rate_budget]
- [ ] [parallel] Add the dependent-composition fixture: session-manager failure restarts the dependent subscription worker in order while the independent verifier continues, and a subscription-worker failure does not tear down the session manager. r[molten.system_extension.recovery_groups]
- [ ] [parallel] Add restart-class fixtures: a successfully completed temporary and transient task stops without consuming budget; a permanent member restarts within budget; a transient member's abnormal exit consumes budget. r[molten.system_extension.recovery_groups]

## Time-windowed budgets

- [ ] [serial] Implement the bounded per-member restart history and windowed budget with backoff and the group-level budget that survives supervisor replacement. r[molten.system_extension.restart_rate_budget]
- [ ] [parallel] Add the storm fixture: restarts within the window terminate into quarantine or escalation; replacing the supervisor does not reset the group budget; replay with recorded observations is deterministic. r[molten.system_extension.restart_rate_budget]

## Validation and closeout

- [ ] [serial] Run focused supervision, lifecycle, and fabric-time tests, formatting, Clippy, Octet, Cairn validation, and the proposal, design, and tasks gates. r[molten.system_extension.recovery_groups] r[molten.system_extension.restart_rate_budget]
- [ ] [serial] Retain the redelivery/retry separation, ordered-group, and admitted-observation non-claims before sync or archive. r[molten.system_extension.recovery_groups] r[molten.system_extension.restart_rate_budget]
