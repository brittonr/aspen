# Enforce scheduler queue bounds on yield

## Why

F10 is an executed defect at source revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.
Set `max_scheduler_queue_depth` to one with enough active capacity for A and B.
Wake A, select A, Wake B, then Yield A.
The core returns two Ready entries under a queue bound of one.
Expected behavior is the admitted overload result with A still Running and B still Ready.

The source is `crates/molten-core/src/fabric_time/scheduler/mod.rs`.
Yield calls `transition_phase` without queue admission.
The current consumer is the fabric-time scheduler through `ExtensionTimeContext` and live/simulation shells.

## Proposed change

Use one pure ready-admission contract for new Wake, blocked Wake, and Yield.
Check the named profile queue bound before phase, sequence, or capacity mutation.
Preserve the complete state under Reject, Backpressure, invalid phase, and arithmetic error.
Keep adapter effects downstream of accepted decisions.

## Ownership and durable capability

Molten fabric-time maintainers own this change and its normal repository regression tests.
The immediate outcome is a queue invariant across all Ready transitions.
The durable capability is a repeatable transition matrix across core, capacity accounting, and adapters.
The existing scheduler service adopts the correction without a new dependency.

## Evidence and non-claims

The audit executed `audit_yield_does_not_exceed_queue_bound` and observed its expected-bound assertion fail.
The audit core baseline reports 359 passing tests. Its harness reports 41 passing controls and eight failing regression assertions.
Eight audit findings have executed counterexamples. Six other findings have static evidence only.
The trigger and result here are self-contained. Ignored scratch logs are not lifecycle acceptance receipts.
This planning pass ran no tests or commands and grants no implementation permission.

This correction does not prove fairness, global liveness, measured performance, callback completion, or release readiness.
