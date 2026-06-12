# Change: deterministic-replay-fixture

## Why

Molten's architecture states deterministic playback as a central law, and the existing `deterministic-test-playback` roadmap has local record/replay profiles and divergence tests. The next implementation slice needs a small, auditable Cairn package that turns the law into a fixture contract: record a bounded local run, replay it without ambient effects, and emit first-divergence evidence when any canonical boundary changes.

Without this slice, later executable transcripts, evaluation cache keys, remote artifact sync, and job DAG replay will each invent their own partial replay identity and diagnostics. A shared fixture establishes the receipt shapes and CLI contract before wiring broader subsystems.

## What

- Add a local deterministic replay fixture contract with canonical run identity, turn journal, effect log, replay verification, and first-divergence receipts.
- Require replay verification to compare scheduler/input/effect/action/receipt/output/after-state boundaries in order and stop at the first mismatch.
- Require replay profiles to inject recorded responses and deny live external effects.
- Add CLI-facing fixture expectations for record, verify, tamper/divergence, and show/readback flows.
- Keep this slice evidence-only: matching replay evidence does not grant authority, policy admission, transport trust, provenance, or release trust.

## Impact

This gives Molten one narrow replay spine that vat fixtures, effect handlers, executable transcripts, evaluation cache, and remote/job replay can reuse. It should be implemented as a local bounded fixture first, then promoted into broader runtime surfaces once the canonical evidence shapes are stable.
