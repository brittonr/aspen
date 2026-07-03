# Design: delivery idempotency state proof

## Scope

This change proves the delivery/idempotency state machine for scoped operation ids, sequence windows, delivery receipts, duplicate suppression, retry decisions, and replayable delivery logs.

## Proof checklist

- **Proof claim**: a first admitted delivery commits exactly once; duplicate operation ids return prior receipt evidence; stale, gap, conflict, or malformed deliveries deny before side effects; replay logs reproduce committed events deterministically.
- **Out of scope**: network-level exactly-once delivery, peer reliability, and live transport ordering guarantees.
- **Trusted assumptions**: canonical operation ids and delivery receipt refs are stable.
- **Positive evidence**: first delivery commits, retry-before-side-effects emits retry evidence, and replayable logs produce expected runtime events.
- **Negative evidence**: duplicate, stale, gap, conflict, non-replayable log, and tampered idempotency receipt cases deny or suppress without applying side effects.
- **Canonical refs**: proof traces bind operation ids, window refs, prior receipt refs, delivery log refs, idempotency receipt refs, and runtime event refs.
- **Regeneration command**: `cargo test delivery` plus focused remote dataspace replay tests if separated.

## Functional core

The decision core should classify delivery attempts before any adapter or runtime side effect. Replay validation should consume recorded delivery logs and idempotency receipts, not live network state.

## Non-goals

- No claim that remote peers cannot resend messages.
- No replacement for control-plane admission or policy gates.
