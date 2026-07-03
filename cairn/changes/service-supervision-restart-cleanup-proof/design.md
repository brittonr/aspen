# Design: service supervision restart and cleanup proof

## Scope

This change proves service runtime and supervision state-machine behavior. It covers demand evaluation, dependency readiness, lifecycle receipts, monitor notifications, bounded restart policy, cleanup receipts, owned assertion/resource cleanup, and replay identity.

## Proof checklist

- **Proof claim**: services start only after dependency and evidence gates pass; dependency waits perform no start side effects; restart decisions are bounded by explicit policy; cleanup is idempotent and removes only service-owned state.
- **Out of scope**: OS process parentage, scheduler fairness, external adapter recovery, and live service SLOs.
- **Trusted assumptions**: lifecycle and service record refs are canonical evidence and authority/resource checks are supplied by their own gates.
- **Positive evidence**: ready dependencies produce passing lifecycle receipts and readiness assertions; restart within budget emits deterministic monitor and lifecycle refs; cleanup retracts owned state.
- **Negative evidence**: missing dependency, missing authority/resource evidence, exhausted restart budget, stale readiness ref, and repeated cleanup deny or no-op without unintended mutation.
- **Canonical refs**: proof traces bind demand refs, manifest refs, dependency refs, lifecycle refs, monitor refs, cleanup refs, and owned assertion/resource refs.
- **Regeneration command**: `cargo test service`.

## Functional core

Restart evaluation and cleanup planning should remain pure over service records and runtime snapshots. Shell code may start or stop adapters only after receipts prove the transition is admitted.

## Non-goals

- No guarantee that a faulty adapter becomes healthy.
- No compatibility claim for OTP supervision semantics.
