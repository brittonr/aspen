## Implementation

- [x] I1 Add compatible `elapsed_ms` receipt field and serialization/legacy parsing tests. Evidence: `cargo test -p aspen-dogfood receipt -- --nocapture` passed 2026-05-05. [covers=dogfood-evidence.stage-receipts.elapsed-ms.recorded]
- [x] I2 Populate elapsed milliseconds for all dogfood full-run stages, including publish and stop/failure stages. Evidence: `cargo test -p aspen-dogfood receipt -- --nocapture` passed 2026-05-05. [covers=dogfood-evidence.stage-receipts.elapsed-ms.recorded]
- [x] I3 Render elapsed duration in `receipts show` while preserving fallback for legacy receipts. Evidence: `cargo test -p aspen-dogfood receipt -- --nocapture` passed 2026-05-05. [covers=dogfood-evidence.stage-receipts.elapsed-ms.legacy-compatible]

## Verification

- [x] V1 Run focused dogfood receipt tests. Evidence: `cargo test -p aspen-dogfood receipt -- --nocapture` passed 27 tests, 38 filtered out. [covers=dogfood-evidence.stage-receipts.elapsed-ms.recorded]
- [x] V2 Run full package tests for the affected dogfood binary. Evidence: `cargo test -p aspen-dogfood` passed 65 tests. [covers=dogfood-evidence.stage-receipts.elapsed-ms.recorded]
- [x] V3 Validate the OpenSpec change before archive and affected canonical dogfood evidence spec after archive. Evidence: `openspec validate record-dogfood-stage-duration --strict --json` passed before archive; `openspec validate dogfood-evidence --strict --json` passed after archive. [covers=dogfood-evidence.receipt-inspection.show.run-id]
- [x] V4 Run whitespace/diff checks before commit/archive. Evidence: `git diff --check` passed after archive. [covers=dogfood-evidence.stage-receipts.elapsed-ms.legacy-compatible]
