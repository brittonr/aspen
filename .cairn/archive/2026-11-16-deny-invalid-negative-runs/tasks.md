## Phase 1: Implementation

- [x] [serial] Recompute run diagnostics when parsing receipts and reject receipts whose diagnostics differ. r[molten.testing.receipt_driven_traceability.coverage_derivation]
- [x] [serial] Keep run diagnostics through conversion, stop substituting the receipt hash for artifacts, and report invalid runs as stale evidence. r[molten.testing.receipt_driven_traceability.stale_receipt_denial]
- [x] [serial] Cover the issue's table through strict manifest admission and update the receipt contract article; run focused tests, formatting, Clippy, the Tracey debt guard and source/lifecycle gates before review, sync and archive. r[molten.testing.receipt_driven_traceability.stale_receipt_denial]

## Verification (2026-11-16)

Integration tree with the #43, #44, #57 and #58 changes applied: `cargo fmt --check`; `cargo test -p molten --lib traceability` (4 passed, including `invalid_negative_runs_never_qualify_as_negative_coverage` and `a_receipt_with_a_dropped_run_diagnostic_is_rejected`); `cargo clippy -p molten --lib --bins --tests -- -D warnings` passed. These are native test observations; they do not establish why a recorded command exited nonzero.
