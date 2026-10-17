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

## Implemented representation and executed observations

The selected representation is canonical `node-control-run-v1` over state-root,
startup and previous-run refs, with a run-bound active lock and a separate
`node-control-run-shutdown-v1` completion over run/startup/shutdown/control refs.
The previous-run content link distinguishes otherwise identical startups without
adding a clock or entropy read to the pure classifier. Legacy control and shutdown
values keep their original schemas and bytes; missing lineage cannot be synthesized
from historical success. No existing canonical loop fields were added or rewritten.

`src/node/parts/runtime/p008/body.rs` owns allocation-free lifecycle classification;
`p009/body.rs` owns the new canonical values. The daemon's `p058/body.rs` gathers
capability-rooted observations and resolves the immutable control reference named
by completion. Status/restart therefore survive a subsequent denied direct stop,
while unreadable or conflicting observations reject before new shutdown effects.
Service decision and current stopped state remain independent even when pending
work denies the bounded loop after shutdown completed.

Attempt 16 retains the original failing normal restart-duplicate regression, an
actual pre-fix CLI reproduction, the review-discovered service-state reproduction,
and passing current CLI/core/adapter observations under
`evidence/cairn-source-remediation-2026-10-15/attempt-16-current-run-shutdown-observation/`.
Its native pre-sync traceability manifest covers exactly the four F14 requirements
from eight executed positive/expected-deny verification receipts. This bounded
runtime evidence is not source-gate, lifecycle, remote-shutdown or release authority;
the final task retains every required acceptance gate.

## Attempt 17 acceptance checkpoint

Attempt 17 replaced the Clippy-reported cloned one-element test slices with borrowed
slices. Fresh default and compatible-feature workspace tests, all three denied-warning
Clippy cohorts, formatting, configured workspace/library Octet producers and the
unchanged strict source gate pass. Fresh actual CLI consumers exercised 15 scenarios
through 89 node commands; eight native positive/expected-deny receipts cover exactly
the four F14 requirements with no missing or stale coverage. Two bounded static
reviews found no blocking core or daemon findings.

Acceptance remains open: the required `node-state-authority` and `nextest` Nix build
timed out after 3,500 seconds waiting for the global garbage-collector lock. It did
not produce passing check results. Post-blocker native structural validation, the
proposal/design/tasks gates and the inherited-debt guard pass, but do not replace
that missing execution. No F14 sync or archive is authorized by this checkpoint.

Exact commands, canonical artifacts, reviews and the blocked attempt result are
retained under
`evidence/cairn-source-remediation-2026-10-15/attempt-17-current-run-acceptance-repair/`.
Node-runtime maintainers retain lifecycle semantics; node-host maintainers retain
observation mechanics. The final acceptance task remains unchecked pending host GC
remediation and the unchanged required checks.

## Resumed acceptance after host recovery

The unchanged `node-state-authority` and `nextest` Nix checks completed successfully
in 651.24 seconds. Hermetic nextest executed 1,716 tests, all passing with no skips,
and emitted canonical CI receipt
`blake3:bc660b19ac58e502f7f4624e1157eabdb31b3787c71bfb62dafbdf83d5947a41`.
The retained packed receipt and its Cargo metadata, binary metadata and JUnit
members match their native BLAKE3 references.

[Resumed acceptance](evidence/resumed-acceptance.json) records the unchanged
attempt-17 source and runtime cohort and the new required Nix execution. The
earlier failed attempt is preserved. Final native review, gates, sync, accepted-spec
traceability and archive identities are recorded under
`evidence/cairn-source-remediation-2026-10-15/attempt-17-resumed-nix-acceptance/`.
Passing checks do not themselves authorize lifecycle mutation or widen the existing
physical-termination, crash-durability, remote-authority and liveness nonclaims.

