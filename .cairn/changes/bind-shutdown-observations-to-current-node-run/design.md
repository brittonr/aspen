## Context and evidence

F14 applies to `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

`src/node/parts/daemon/p027/body.rs:238-271` returns prior dispatch evidence after request-reference checks. `p018/body.rs:289-293` suppresses duplicate execution. `p018/body.rs:353-355` treats every passing shutdown result as current stopped state. `p036/body.rs:115-147` removes the old shutdown file during restart but retains outbox history.

Trigger: dispatch valid shutdown request R in run A. Restart as run B. Submit exact R again and process it through the control loop. The old passing receipt returns without new shutdown effects. Actual classification sets `has_stopped=true` and exits while run B retains its active lock. Expected classification retains duplicate suppression and does not report run B as stopped.

This is static evidence only. The audit ran no live restart or shutdown reproduction. Its 359 passing core baseline tests and eight unrelated failing regression assertions do not establish execution coverage for F14.

## Separate decisions

Duplicate classification answers whether the request already has a recorded result. Current stopped classification answers whether the current run has an observed completed shutdown. A historical passing result answers only the first question.

The pure core receives dispatch provenance, current run identity, shutdown observations, and active-state observations. It returns duplicate status separately from stopped, active, or unresolved lifecycle status. It performs no filesystem access or clock reads.

The shell gathers these facts through node-host capabilities and executes only admitted current operations. It must not reexecute a historical shutdown merely to make current status match an old receipt. A new shutdown requires a distinct request and fresh admission.

## Current-run binding

The run identity must distinguish restarts even when canonical startup inputs remain identical. A startup content reference alone is insufficient unless the existing contract guarantees a unique run binding. Review active-lock and startup identity contracts before selecting the representation.

The shell supplies any nondeterministic run identity explicitly. The core validates exact equality. Current stopped state requires matching shutdown observations and lifecycle completion, not historical success or lock absence alone. Conflicting or unavailable observations produce an unresolved result without shutdown effects.

## Compatibility and replay

Preserve exact duplicate suppression and archived receipt bytes. Old receipts without sufficient run binding remain readable as historical results. They cannot establish current stopped state. Receipt readers and loop outputs must keep duplicate provenance distinct from current lifecycle observations.

Review whether an internal typed dispatch outcome suffices or canonical receipts require a versioned field. Replay must retain the original run context and must not reinterpret historical results against a later run. The operator documentation must explain why duplicate success no longer implies that the current node stopped.

## Tests and shell observations

Baseline `control_loop_processes_queue_idempotently_and_stops_on_shutdown` and `local_node_init_run_status_stop_and_restart_recovery_are_receipted` before core edits.

Add a normal repository regression for shutdown R, restart, and duplicate R. Assert no repeated shutdown calls, unchanged current active lock, and no false stopped result. Add a valid current-run shutdown control, same-run duplicate controls, conflicting run bindings, missing observation errors, and legacy receipt cases.

Controlled adapter tests cover call order and capability-rooted observation failures. Pure tests cover the decision matrix with equal startup content across distinct runs. Rejection tests compare lifecycle state before and after the decision.

Run focused Octet and Clippy error gates, workspace tests, relevant Nix node-state checks, and required Cairn gates. Preserve every existing required check and record blocked checks explicitly.

## Ownership, reuse, and order

Node-runtime maintainers own duplicate and lifecycle meaning. Node-host maintainers own observation mechanics. Existing components remain the reuse boundary. No new dependency is mandated.

F01 first establishes shutdown admission where shared code needs ordering changes. F14 then separates historical results from current observations without weakening that admission. F02 supplies real startup gate evidence and shares fixture composition. F03 can reuse dispatch observations for local ingress recovery but does not own their lifecycle meaning. These are review overlaps, not circular blocking dependencies.

## Nonclaims

Current-run receipt linkage does not prove physical process termination, crash durability, remote shutdown authority, or whole-system liveness. Static analysis remains static until normal tests execute. This package grants no implementation or publication authority.
