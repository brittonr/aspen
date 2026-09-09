# Saturate exponential retry arithmetic

## Why

F12 is an executed defect at source revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.
In the generic retry planner, use base delay two, attempt 63, maximum delay 128, no jitter, and an admitted attempt budget.
The exponential calculation returns zero instead of the expected maximum delay 128.
`checked_shl` checks the shift amount, not loss of high bits.
The source is `crates/molten-core/src/fabric_time/lease.rs`, in `plan_retry`.

## Proposed change

Compute capped exponential growth through checked multiplication or an explicit pre-shift overflow check.
Keep the named maximum delay as the saturation bound.
Preserve fixed-delay behavior, generation and domain admission, finite attempts, jitter checks, and deadline overflow errors.

## Ownership and durable capability

Molten fabric-time maintainers own the generic retry planner and regression tests.
The current consumer is the public fabric-time retry API and its conformance fixture.
Coordination delivery consumes the API through a fixed-delay profile. This package does not claim that profile exposes exponential wrap.
The immediate outcome is correct capped delay arithmetic.
The durable capability is a repeatable boundary matrix in normal repository tests without a new dependency.

## Evidence and non-claims

The audit executed `audit_exponential_retry_saturates_instead_of_wrapping` and observed actual delay zero against expected delay 128.
The core baseline reports 359 passing tests. The harness reports 41 passing controls and eight failing regression assertions.
Eight audit findings have executed counterexamples. Six other findings have static evidence only.
This tracked description preserves the trigger and result without reliance on ignored scratch evidence.
This planning pass ran no tests or commands and grants no implementation permission.

Correct retry arithmetic does not prove safe retries, global liveness, measured performance, remote deadline agreement, or production exposure.
