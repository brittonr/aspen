# F09 design

## Source and current behavior

Reviewed source revision: `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.
Finding: F09. Accepted contract: `.cairn/specs/fabric-time-scheduling/spec.md`.
Architecture: `docs/fabric-time-scheduler-runtime.md`.

`wake` rejects every existing key before phase or capacity admission.
`block` accepts Ready and Running, but no existing transition returns Blocked to Ready.
The executed sequence is `Wake(A) -> Block(A) -> Wake(A)` within one generation.
The final result is `DuplicateRunnable(A)`, not Ready.

## Core decision

Classify Wake as new occurrence, blocked resume, duplicate, or stale before capacity mutation.
A blocked resume keeps the key and record and adds no active slot.
Ready admission checks `max_scheduler_queue_depth` for both new Wake and resume.
Use the F10 shared admission rule, not a second queue policy.
On success, apply the supplied priority, reset ready wait accounting, and assign a fresh enqueue sequence through checked arithmetic.
On rejection or overflow, preserve the complete input state and all sequence positions.
Other existing phases retain duplicate rejection. F11 defines identity protection after terminal reclamation.

The core receives profile, policy, generation, and command as explicit values.
No clock, callback, receipt write, or allocation effect belongs in this decision.
An internal helper needs no new port.

## Shell and adapters

`ExtensionTimeContext::apply_scheduler_command` currently checks every Wake against `self.max_runnables`.
Replace that unconditional charge with the core decision about new versus existing work.
The extension envelope remains binding for new work. Resume at a full active limit remains valid with queue capacity.
The shell retains service, profile, and generation admission and executes only admitted wake effects.
Live and simulation adapters must return the same action and state for the same explicit inputs.

## Compatibility, replay, and evidence

Wake on Blocked intentionally changes from an error to `Woken`.
Keep existing command shape and canonical action names unless identity work requires an explicit version change.
Document supplied-priority and fresh FIFO-position semantics.
Existing successful schedules retain ordering. A legacy trace expecting the defect must diverge visibly, not receive silent reinterpretation.
Receipts bind the actual admitted transition and choice position, not an assumed callback completion.

The audit counterexample is executed evidence for the core defect only.
The shell precheck is static evidence. New adapter claims require executed conformance tests.
No global liveness or measured performance claim follows from resumed work.

## Overlap and order

F10 owns shared ready-queue admission. Prefer its helper before the F09 integration, or land both coherently in one review sequence.
F09 owns blocked-resume classification and extension slot charging.
F11 owns terminal retention and fresh occurrence identity. It must preserve F09 resume identity for a still-active occurrence.
F12 owns retry arithmetic and has no scheduler dependency.
These are integration order notes, not circular lifecycle blockers.

## Validation approach

Before edits, run the smallest existing scheduler tests in `molten-core` and the shell Wake test.
Move the audit reproduction into normal core tests without absolute paths or ignored harness dependencies.
Cover full active capacity with queue space, full ready queue, duplicate phases, stale keys, sequence overflow, and denied-state equality.
Pair live and simulation tests with positive and denied wake-effect observations.
Retain capacity, replay, Octet, Clippy, workspace, Nix, and Cairn gates.
Molten fabric-time maintainers own the durable tests and compatibility documentation.
