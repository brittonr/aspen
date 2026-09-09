# F11 design

## Current source and evidence

Finding F11 refers to source revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.
The accepted contract is `.cairn/specs/fabric-time-scheduling/spec.md`.
Architecture and capacity claims appear in `docs/fabric-time-scheduler-runtime.md`.

`SchedulerState.runnables` retains Completed and Cancelled entries indefinitely.
Wake counts nonterminal entries against `max_runnables`, then appends to that vector.
`cleanup_scheduler_generation` changes phases but does not reclaim entries.
The executed sequence completes A, then B, under `max_runnables = 1` and retains both records.

## Retention and identity contract

Define a named finite terminal-retention bound derived from the admitted profile and capacity plan.
Do not add an independent unreviewed configuration value or silently enlarge initialized storage.
Keep active usage, retained terminal usage, and physical capacity as distinct checked counts.
Prefer deterministic oldest-terminal reclamation at terminal transitions and before new-occurrence admission.
Reject unrepresentable plans and arithmetic overflow before mutation or activation.
A retained tombstone permits bounded historical diagnostics, not indefinite duplicate protection.

Bounded tombstones alone cannot prevent ABA after eviction.
Adopt an explicit monotonically admitted occurrence sequence within each service generation, separate from the reusable logical runnable name.
The pure core tracks a bounded high-water fence and admits only fresh issuance beyond that fence.
An existing blocked occurrence resumes through exact occurrence identity, not fresh issuance.
Terminal callbacks and duplicate Wake cannot create a new occurrence, even after their record disappears.
On sequence exhaustion, reject fresh issuance. A generation change requires the existing lifecycle authority and cannot occur as a hidden fallback.

The shell supplies occurrence issuance requests and binds every callback to the admitted occurrence identity.
The core owns freshness checks, retention selection, and resulting state.
A random identifier without a retained freshness proof does not satisfy the fence.
Retain no unbounded set of retired IDs as a replacement for the unbounded runnable vector.

## Capacity and effect boundaries

Update the checked plan to account for the chosen finite retention representation within initialized physical storage.
The shell owns reservation and release effects. It must not widen storage after activation.
Use the existing application port boundary. No generic retention framework or new dependency is required.
Existing shared capacity and lifecycle components are review candidates, not mandatory dependencies.

## Compatibility, replay, and receipts

Fresh occurrence identity changes key and callback interpretation.
The parent must review the exact canonical schema and profile-version boundary before implementation.
Legacy callbacks without an occurrence fence require explicit rejection outside a reviewed migration cohort.
Replay must bind occurrence identity and deterministic reclamation order.
Old traces must not silently map an old logical name onto a fresh occurrence.
After tombstone eviction, readback reports unavailable historical detail rather than invented completion evidence.
Receipts retain bounded observation claims and cannot authorize resurrection.

## Sibling overlap and order

F09 owns resume for a still-active blocked occurrence and preserves its active slot.
F10 owns shared Ready admission. Establish those semantics before adapting occurrence keys and retention accounting, where practical.
F11 owns identity migration and terminal reclamation. Neither F09 nor F10 requires F11 completion to define its local invariant.
F12 is independent. There are no circular blocking dependencies.

## Validation and owner

Molten fabric-time maintainers own the durable state-machine and adapter tests.
Before edits, run existing scheduler, capacity, and shell tests.
Move the F11 sequence into normal repository tests with named repetition and retention bounds.
Cover Complete, Cancel, cleanup, repeated reuse attempts, stale callbacks after eviction, sequence exhaustion, and rejected-state equality.
Replay tests must distinguish old and fresh occurrences with the same logical name.
Run positive and negative serialization and live/simulation callback tests, then required Octet, Clippy, workspace, Nix, and Cairn gates.
The audit proves record accumulation only. New fences and capacity claims require fresh execution evidence.
No test count proves global liveness, host memory stability, or measured performance.
