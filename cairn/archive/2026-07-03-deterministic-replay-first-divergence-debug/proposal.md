## Why

Replay denial is strongest when it identifies the first semantic divergence instead of only reporting that replay failed. The current replay-fixture work adds first-divergence evidence for tampered deterministic runs and explicitly denies live external effects when recorded effect responses are missing. This should be captured as a Cairn change before it is committed.

## What Changes

- Extend deterministic replay verification receipts to bind expected and actual refs for replay identity, scheduler, input, effect request, effect response, policy decision, action, receipt, output, and final-state comparisons.
- Emit a canonical first-divergence record and ref for deny receipts.
- Add fixture tampering support and CLI coverage for pass and tamper-denial replay workflows.
- Preserve recorded-effects-only replay semantics and classify live-effect attempts as denial evidence.

## Impact

- **Files**: deterministic replay core, replay-fixture CLI, replay tests, and CLI harness tests.
- **Testing**: focused replay tests, full `cargo test`, formatting, clippy after the clippy cleanup package lands, and release dogfood after commit.
