# Design: Admit recovery-group supervision

## Context

`SupervisionDecision` is `{Restart, Quarantine}` and `plan_supervision(failure, restart_attempts, max_restart_attempts)` is a pure function of the failure class and lifetime attempt count. `LifecycleState` carries no restart history beyond `restart_attempts` and no group membership. The manifest vocabulary (`manifest.rs`) admits resource envelopes and execution profiles but no recovery relationships.

## Approach

1. **Recovery-group admission.** Extend the admitted manifest with an optional recovery-group declaration: group members, an explicit ordering (startup order; shutdown in reverse), each member's restart class (`permanent`, `transient`, `temporary`), and the group's restart-rate window, backoff, and budget. Admission rejects cycles, duplicate members, unknown children, and windows without an elapsed-time source.
2. **Pure recovery planning.** Replace the single-child decision with a pure `plan_recovery` over the failure, the group declaration, the per-member restart history, and admitted elapsed observations. Output is an ordered recovery plan: which members to stop, which to restart, in which order, or quarantine/escalate. The existing single-service case is the one-member group, keeping the current behavior as the degenerate case.
3. **Time-windowed budget.** Record restart timestamps from admitted elapsed-time observations in a bounded per-member history (a fixed-size ring, like existing bounded state). The plan restarts only while the windowed count is under budget and the backoff has elapsed; otherwise it quarantines the member or escalates the group. Group-level budgets accumulate across supervisor replacement so a replacement cannot reset them.
4. **Task completion.** A `temporary` member that exits successfully transitions to `Stopped` without consuming restart budget. A `transient` member that exits successfully also stops; only abnormal exits consume budget. `permanent` members restart within the budget regardless of exit status.
5. **Authority and effect separation.** Every planned restart carries required current authority checks and binds to the current incarnation from `fence-runtime-local-work-across-restarts` when that change has landed. Redelivery and external-effect retry remain owned by the delivery extension and are untouched.

## Alternatives considered

- A general dependency graph with transitive closure. Rejected: OTP's ordered groups cover the realistic compositions without graph-solver complexity; dependencies beyond ordering can be expressed as group membership.
- Resetting budgets on supervisor replacement and relying on quarantine. Rejected: it conceals restart storms across replacements, which is the failure the group budget exists to expose.
- Wall-clock timestamps from the shell. Rejected for the pure core: planning consumes admitted elapsed observations so replay stays deterministic.

## Non-claims

- Recovery plans are advisory to the shell until executed; planning does not prove the restart succeeded.
- Ordered groups do not solve arbitrary dependency graphs or distributed failure detection.
- Real-time restart-rate claims hold only against admitted elapsed observations, not logical ticks.

## Risks

- Manifest shape change affects existing extension fixtures; the field is optional and absent declarations keep current behavior.
- Bounded restart history sizing must be admitted (named constant bound), or a chatty history could grow unbounded.
