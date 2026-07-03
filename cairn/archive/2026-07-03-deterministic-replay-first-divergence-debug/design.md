# Design: deterministic replay first-divergence debug

## Scope

This change covers generic deterministic replay fixture verification and replay debug evidence. It does not change production execution, authority admission, transport, or release promotion semantics.

## Proof checklist

- **Proof claim**: replay verification passes only for unchanged recorded deterministic evidence and denies at the first semantic divergence with a canonical first-divergence ref.
- **Out of scope**: broad distributed replay, live transport correctness, authority/provenance/policy/resource trust, and using debug evidence as pass evidence.
- **Trusted assumptions**: canonical Preserves rendering and BLAKE3 refs are stable for the compared replay values.
- **Positive evidence**: unchanged fixture verification emits `decision=pass`, `divergence=none`, and no first-divergence value.
- **Negative evidence**: tampered identity, scheduler, input, effect request, effect response, policy decision, action, receipt, output, final-state, and missing-recorded-effect cases deny with the expected divergence kind.
- **Canonical refs**: replay verify receipt ref, first-divergence ref, expected/actual identity refs, output refs, final-state refs, effect-log refs, and manifest refs.
- **Regeneration command**: `cargo test replay -- --nocapture`.

## Functional core

The replay comparison remains pure over already materialized deterministic run parts. The CLI reads fixture files, writes optional receipts, and renders summaries; it does not decide replay pass/deny behavior.

## Non-goals

- No replay catalog or MCP expansion beyond existing classifications.
- No release evidence regeneration in this change.
- No live effect execution during replay.
