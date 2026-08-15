# Design: runtime turn commit/rollback proof

## Scope

This change proves the runtime dataspace turn state machine. It covers pending turn construction, commit, rollback, predicate receipts, before/after snapshot refs, assertion/message visibility, and recorded effect-response application.

## Proof checklist

- **Proof claim**: committed turns make exactly the predicate-approved delta visible, while denied or failed turns leave committed runtime state unchanged and produce denial or rollback evidence.
- **Out of scope**: external adapter execution, wall-clock scheduling, network delivery, and service-specific policy decisions.
- **Trusted assumptions**: `RuntimeSnapshot` refs are canonical and the runtime predicate receipt builder correctly hashes its input value.
- **Positive evidence**: generated valid turns commit assertions, retractions, messages, and effect responses with after-state refs matching replay.
- **Negative evidence**: denied turns, stale commits, failed predicate checks, and rollback paths preserve the before-state ref and do not publish pending effects.
- **Canonical refs**: proof traces bind before snapshot refs, pending turn refs or action refs, after snapshot refs, predicate receipt refs, decisions, and diagnostics.
- **Regeneration command**: `cargo test runtime` or the smallest focused runtime/dataspace predicate test command available.

## Functional core

The proof should keep turn-delta logic pure and deterministic. Tests should generate in-memory steps, compute before/after snapshots, and assert receipt decisions without filesystem or network dependencies.

## Non-goals

- No claim that external effects are exactly-once.
- No new actor scheduling semantics.
