## Context

`classify_attempt_observation` mapped both `Succeeded` and `Failed` observations to state `Observed` and kept only the observation reference, so `acknowledge_attempt` and `plan_world_promotion_observation_commit` could not tell them apart. `dispatch_world_reservation` stored an `attempting` record, then propagated any dispatcher error with `?`.

## Decisions

### Decision: Make the outcome a separate field

**Choice:** Add `WorldAttemptOutcome { Unknown, Succeeded, Failed }` to `WorldAttemptRecord` instead of overloading the release state or `external_completion_proven`. Classification records `Succeeded` or `Failed` from the observation and `Unknown` for unknown, duplicate and conflicting observations. Acknowledgement and observation commit both require `Succeeded`.

**Rationale:** State describes where the release is in its lifecycle; the outcome describes what the effect reported. Keeping them separate lets a failed release stay `observed` without becoming completion evidence, and lets an unknown outcome stay distinct from both.

### Decision: Version the attempt record

**Choice:** The canonical attempt and observation records gain an `outcome` field before `external-completion-proven`; the attempt schema becomes `molten.world-release-attempt.v2` with arity 8, and `schemas/preserves-boundaries/molten-world-release-attempt-v2.preserves` records it. The parser rejects any other outcome string and the seven-field v1 layout.

**Rationale:** A v1 record cannot express failure, so reading it with a default outcome would invent a fact. No production dispatcher has written v1 attempts; a promotion that needs one is re-run.

### Decision: Record uncertainty before returning a dispatcher error

**Choice:** `record_dispatch_uncertainty` stores an `uncertain` attempt with an unknown outcome, moves the reservation to `uncertain`, and publishes the uncertain observation receipt, then returns the dispatcher's error unchanged. If any of those writes fails, it returns one error naming both the dispatcher failure and the recording failure.

**Rationale:** After a dispatcher error the effect may or may not have happened. `uncertain` is the state from which the existing retry (with acknowledged duplicate risk) and abandon (with acknowledged unknown outcome) paths start. Silently dropping a recording failure would recreate the stuck `attempting` state.

### Decision: Reconcile classifies interrupted attempts

**Choice:** `recover_interrupted_attempts` lists attempts still `attempting` and records each, with its reservation, as uncertain through the same helper as the dispatcher-error path. The `reconcile` command runs it before reporting. The bookkeeping port gains `list_attempts_in_progress`, bounded by the reservation inventory because each reservation has at most one in-progress attempt.

**Rationale:** A process can stop after storing `attempting` and before recording the result. The Redb store is opened by one process at a time, so an `attempting` record seen by `reconcile` cannot belong to a running dispatcher. Classification does not retry; retry and abandon keep their explicit acknowledgements.

## Risks / Trade-offs

- The combined error is still a harness-class failure; typed failure classes are tracked separately in #58.
- This change does not add a production dispatcher, prove exactly-once effects, or change retry and abandon authority.
