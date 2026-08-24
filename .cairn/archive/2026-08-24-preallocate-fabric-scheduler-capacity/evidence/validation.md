# Validation evidence

## Scope

This change binds physical scheduler reservations to one admitted profile and generation.
It does not change scheduler ordering, fairness, replay, cleanup, or overload policy.

Base source commit: `6dcddb4564d3b48c4913e323397e520c8eb82577`.

## Baseline

`nix develop -c cargo test -p molten-core` passed before implementation.
The baseline ran 186 tests with no failures.

The existing scheduler ordering, overload, fairness, replay, generation, and cleanup tests passed.
The proposal, design, and tasks gates passed before implementation.

## Functional core

`crates/molten-core/src/fabric_time/scheduler/capacity/` owns deterministic planning and accounting.
A plan binds the admitted profile reference, active generation, runnable slots, queue slots, concurrency slots, and checked total.

Planning rejects zero generation, zero limits, relation errors, hard-cap errors, count-conversion errors, and arithmetic overflow.
Runtime accounting rejects stale generations, wrong profiles, wrong plans, released state, underflow, overflow, and exhaustion.

Plan and observation identities use domain-separated BLAKE3 framing.
Observations record bounded usage, high-water, exhaustion, release, profile, and generation facts.

## Imperative shell

`src/fabric_time/capacity/` owns fallible allocation and reservation release.
Activation reserves concrete `RunnableState` and `RunnableKey` slots before it returns a runtime.
Any reservation error denies activation without a smaller fallback.

The shell does not widen either reservation after activation.
Release drops both reservations and marks the accounting state as released.

## Positive and negative validation

The focused matrix covers these cases:

- valid and stable plan identity
- zero generation
- one-past-hard-cap limits
- queue and concurrency relation errors
- count conversion and arithmetic overflow
- complete activation
- forced queue-allocation failure
- stale generation and wrong profile
- exhaustion, underflow, release, and restart fencing
- unchanged FIFO selection
- scoped non-claim observations

`nix develop -c cargo clippy -p molten-core -p molten --all-targets -- -D warnings` passed.
`nix develop -c cargo fmt --check` passed.

The focused Octet runs reported existing workspace findings.
They reported no finding for the new core or shell files.
These warning-only runs are not strict Octet acceptance evidence.

## Nix

`nix build .#checks.x86_64-linux.molten --no-link -L` passed with local builders.
The Nix nextest rail ran 1,378 tests with no failures or skips.
Its CI receipt is `blake3:90eebe1267ff6c2bbf11f8024e4326656a9070b0047173e3236a6d28b9725fb5`.

## Cairn

Strict Cairn validation passed before sync.
The result covered 77 accepted specifications and had no issues.

Final gate receipts before sync:

- proposal: `5e9313996e7528fdf449285830b4d54ceb5944894972f20dcfcd6c7322eae547`
- design: `a218988bbd98877c8e70ad1ec247d8adf8c5a951436775c0dd3fbf408b15e8c9`
- tasks: `991e7d0d5cbca7403c12891eeb079e1b6b1050050df4562bb671bc7e290729aa`

The sync dry-run passed with plan `dcfe784c3ab7656244bd8f4e580a922f2c839523dfeae39a4b30d8393168e78f`.
The executed sync added all six requirements to `fabric-time-scheduling`.
Strict validation passed after sync.

The archive dry-run passed with plan `3823e987135775656b5a845defabc5d3bbaf8da5452ad35d63d4fcec6cb12cb8`.
Archive execution moved the package to `2026-08-24-preallocate-fabric-scheduler-capacity`.
The archive receipt is `acf0ac8b0753b4e1f22ab1bf0d1d1f22fa0defd39207a9bb97f36fbecda30805`.
Strict validation passed after archive execution.

## Non-claims

Capacity evidence describes one scheduler instance and one exact implementation cohort.
It does not prove global latency, fairness, liveness, host memory stability, or whole-runtime zero allocation.
It does not make Rust memory layout canonical authority.
