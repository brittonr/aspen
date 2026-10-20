# Proposal: Admit recovery-group supervision

## Why

`plan_supervision` in `crates/molten-core/src/system_extension/supervision.rs:356-367` makes one decision per failure: retryable failure under the lifetime attempt budget restarts; anything else quarantines. This primitive cannot express that some children depend on others, that a restart loop should terminate within a time window rather than a lifetime count, or that a completed finite task differs from a failed long-running service.

OTP's supervision model supplies the reference cases: restart only the failed child, restart a tightly coupled group, or restart the failed child plus children later in startup order (`rest_for_one`, which is ordered, not a dependency-graph solver); bounded restarts per time interval with escalation; and permanent, transient, and temporary child policies.

## What Changes

- Extend the supervision core with an admitted recovery-group policy that identifies, per group: the affected children, ordering constraints, restart conditions, shutdown bounds, and required current authority. The pure core plans the recovery sequence; the shell performs it through existing lifecycle and execution ports.
- Add a time-windowed restart budget alongside the lifetime attempt count, using fabric-time elapsed observations (not logical event ticks), with backoff and a group-level budget so replacing a supervisor cannot reset child budgets and conceal a continuing failure. Replay consumes recorded elapsed observations or an explicitly equivalent virtual-time profile.
- Distinguish long-running services from finite tasks: a successfully completed task MUST NOT be restarted merely because its process exited (temporary/transient/permanent reference cases).
- Keep supervisor restart, message redelivery, and external-effect retry as three separate decisions; this change touches only the first.

## Impact

- **Files**: `molten-core` supervision model and manifest vocabulary, lifecycle restart transitions that consult supervision, and supervision fixtures and tests.
- **Testing**: a dependent worker restarts with its session manager while an independent sibling continues; ordering is enforced; a restart storm terminates into quarantine or escalates within the admitted window; a completed task is not restarted; supervisor replacement does not reset group budgets.
- **Non-goals**: no change to delivery redelivery or unknown-effect retry semantics, no general dependency-graph solver (ordered groups only), and no claim of OTP API or protocol compatibility.

## Dependencies

- `fence-runtime-local-work-across-restarts` supplies the incarnation identity that restarted children re-fence under; recovery planning should bind to it when both land.
- Fabric-time elapsed observation admissions for the restart-rate window.
