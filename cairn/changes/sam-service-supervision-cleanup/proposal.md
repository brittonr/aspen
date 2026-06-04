## Why

Demand-driven service startup is not sufficient for Aspen 2.0. Molten services need deterministic logical supervision: failures must notify monitors, restart decisions must be bounded and replayable, and termination or revocation must clean up service-owned assertions, observers, live refs, and pending effects.

## What Changes

- Add logical service links, monitors, failure, restart, stop, and cleanup records.
- Implement bounded deterministic restart decisions with pass/deny/backoff receipts.
- Auto-retract service-owned assertions, observers, live refs, and pending effect intents during stop, failure, revocation, or cleanup.
- Enforce resource bounds for restart rate, mailbox/assertion counts, turn count, trace bytes, and cleanup work.
- Add failure/restart/revocation tests and Hegel properties for cleanup completeness.

## Impact

This completes the local deterministic supervision loop after service demand startup. It remains logical and receipt-based; it does not claim compatibility with BEAM OTP, systemd, Kubernetes, or OS process supervision.
