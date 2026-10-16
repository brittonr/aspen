# F12 design

## Source and reproduced boundary

Finding F12 refers to source revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.
The accepted contract is `.cairn/specs/fabric-time-scheduling/spec.md`.
`docs/fabric-time-scheduler-runtime.md` assigns retry calculations to the pure core.

`plan_retry` converts the attempt to a shift and calls `base_delay_ticks.checked_shl(shift)`.
A valid shift can discard high bits without returning an error.
The executed case has base delay two, attempt 63, maximum delay 128, no jitter, and enough permitted attempts.
The actual delay is zero. The expected capped delay is 128.
These exact values are reproduction inputs, not new configuration defaults.

## Pure calculation

Compute the mathematical base delay multiplied by the power of two, capped at `maximum_delay_ticks`.
Use checked multiplication or a pre-shift range proof. Do not trust shift-count validity as an overflow proof.
Use the numeric type width as a named arithmetic boundary, not an unexplained literal.
An admitted large attempt must saturate without an attempt-sized loop or a narrowing-conversion error.
No benchmark or measured performance claim follows from this bounded calculation.

Retain the existing finite-attempt admission before calculation.
Preserve fixed delay, explicit supplied jitter validation, checked jitter addition, and the maximum-delay cap.
A checked deadline overflow remains an error. Saturation of the delay does not authorize wrap of the target instant.
Profile, domain, generation, subject identity, and uncertainty rules remain authoritative.
The core accepts explicit time and jitter values. It reads no clock or entropy source.
No new port or dependency belongs around this arithmetic helper.

## Shell and adapter behavior

Shells obtain time and entropy through existing admitted boundaries.
They publish or schedule retry effects only after a valid plan.
On invalid jitter, exhausted attempts, or deadline overflow, they preserve consumer state and emit no retry timer.
Adapter tests must cover accepted plans and error translation without replacing core arithmetic tests.

## Compatibility and replay

Valid fixed-delay outputs remain unchanged, including the current coordination-delivery profile.
Non-overflow exponential outputs remain unchanged.
Overflowing exponential cases intentionally change from truncated values to the maximum delay.
Canonical retry receipts must record the actual corrected delay and deadline.
Legacy traces that depend on wrapped delay require visible divergence or explicit cohort rejection, not silent receipt rewriting.
The parent must review whether this semantic correction requires a replay/profile cohort version change.

## Evidence, owner, and sibling order

Molten fabric-time maintainers own the durable boundary tests, fixtures, and docs.
The audit executed only the generic arithmetic counterexample, not a live retry or delivery failure.
Current coordination delivery uses fixed delay. Exposure through that profile is not a claim of F12.
F09 and F10 change scheduler admission. F11 changes retention and occurrence identity.
F12 has no blocking dependency on those packages. Shared conformance fixtures require coordination only.

## Validation approach

Before core edits, run `retry_plans_are_bounded_and_jitter_explicit` and existing fabric-time retry tests.
Move the audit case into normal repository tests with named base, attempt, and maximum-delay inputs.
Pair ordinary exponential growth with high-bit loss, type-width boundaries, very large admitted attempts, and exhausted attempt budgets.
Cover fixed-delay equivalence, invalid jitter, mixed domains, stale generation, and deadline overflow without effects or state mutation.
Run adapter and replay fixtures, then required Octet, Clippy, workspace, relevant Nix, and Cairn gates without weakening checks.
No local result proves safe application retries, global liveness, or measured performance.
