## Phase 1: Implementation

- [x] [serial] Add `examples/wasm-optimization-profiles.rs` measuring the four reviewed profiles on the `loop` and `short-lived` workloads, with pure, tested aggregation and no capability or runtime registration. r[wasm_profile_measurement.tool] r[wasm_profile_measurement.reporting] r[wasm_profile_measurement.aggregation]
- [x] [serial] Document the command in `docs/development.md` beside the other diagnostic examples. r[wasm_profile_measurement.tool]

## Phase 2: Verification

- [x] [serial] Run `cargo test --release --example wasm-optimization-profiles` for the aggregation and `cargo run --release --example wasm-optimization-profiles -- WORKLOAD SAMPLES` for both workloads, and record the commit, image, sample counts and raw numbers under `evidence/`. r[wasm_profile_measurement.evidence]
  - Executed on 2026-11-18 at `c92e9f883` on this workstation, image `7decd25535937724`, three processes per workload: `short-lived` shows `pooling` 3.25x faster than `baseline` while `cow` (0.97x) and `instance-pre` (0.99x) match it, and `loop` stays flat (5.9% best-to-worst). Recorded in [the measurement record](evidence/profile-measurement-2026-11-18.json). It supports a static per-shape mapping and not a learned selector; it is not benchmark or release evidence.
- [x] [serial] Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, the Octet source gate, strict Cairn validation and the proposal, design and tasks gates, then sync and archive only with this evidence. r[wasm_profile_measurement.evidence] r[wasm_profile_measurement.tool]
  - Executed at `6185f04ee`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, the Octet source gate through the repository hooks, strict Cairn validation (0 issues) and the proposal, design and tasks gates (0 issues each) all passed.

## Non-goals

No selector, trained artifact, profile-admission change or runtime backend. The
example is diagnostic and stays out of gate logic.
