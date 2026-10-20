# F10 design

## Reviewed boundary

Finding F10 refers to source revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.
The accepted contract is `.cairn/specs/fabric-time-scheduling/spec.md`.
`docs/fabric-time-scheduler-runtime.md` assigns transitions to the pure core and effects to shells.

The executed trigger uses queue bound one and active capacity for A and B.
`Wake(A) -> choose(A) -> Wake(B) -> Yield(A)` leaves both entries Ready.
`apply_scheduler_command` routes Yield directly to `transition_phase`, which refreshes the sequence without queue admission.

## Shared core admission

Add a narrow internal decision for entry into Ready.
Its inputs include current counts, named profile bounds, overload policy, source phase, and whether the operation creates an occurrence.
New Wake requires active and ready slots. Blocked Wake and Yield require ready slots but no new active slot.
Use checked count conversion and sequence arithmetic before state publication.
This helper is deterministic internal policy, not an external port.

Under Reject or Backpressure, preserve the whole state, including Running ownership and choice and enqueue sequences.
Under success, Yield returns the existing occurrence to Ready at a fresh FIFO position.
An invalid Yield phase remains an error before any queue mutation.
Do not silently complete, cancel, drop, or execute a callback to obtain queue space.

## Shell and capacity integration

The shell receives the core result before it updates owned capacity or emits wake effects.
Rejected Yield keeps its running reservation. Successful Yield exchanges running eligibility for ready occupancy according to the existing capacity contract.
Allocation reservations must not widen after activation.
Adapters must not retry overload through a hidden fallback or report `Yielded` after denial.
Live and simulation paths consume the same decision and canonical action.

## Compatibility and replay

Existing successful Yield behavior remains stable when queue space exists.
A saturated Yield now returns `RejectedOverload` or `Backpressure` rather than `Yielded`.
Document that the caller retains the Running occurrence and must use an explicit later command.
Old traces with over-capacity Ready states require visible divergence or explicit unsupported-cohort rejection.
Do not silently rewrite receipts or claim that the callback yielded after rejection.
No new public command or dependency is required for this correction.

## Sibling overlap and order

F10 owns shared Ready admission and overload-state preservation.
F09 then adds blocked-resume classification and removes the shell charge for an existing active occurrence.
Both packages must exercise the same queue rule, regardless of landing order.
F11 adds bounded terminal retention without changing active-versus-ready accounting.
F12 is independent retry arithmetic work. There are no circular blocking dependencies.

## Evidence and validation

Only the core counterexample has executed F10 evidence from the audit.
Adapter and capacity consequences need new conformance evidence before acceptance.
Run existing scheduler and capacity tests before edits, then the normal repository reproduction and transition matrix after edits.
Include both overload policies, invalid phase, sequence exhaustion, stale generation, replay, and unchanged-state assertions.
Retain required Octet, Clippy, workspace, Nix, and Cairn gates without weaker scopes or suppression.
Molten fabric-time maintainers own the durable tests, docs, and bounded receipt claims.
No result establishes measured performance, fairness, or global liveness.
