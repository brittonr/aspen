## Phase 1: Implementation

- [x] [serial] Add `examples/wasm-optimization-profiles.rs` measuring the four reviewed profiles on the `loop` and `short-lived` workloads, with pure, tested aggregation and no capability or runtime registration. r[wasm_profile_measurement.tool] r[wasm_profile_measurement.reporting] r[wasm_profile_measurement.aggregation]
- [x] [serial] Document the command in `docs/development.md` beside the other diagnostic examples. r[wasm_profile_measurement.tool]

## Phase 2: Verification

- [ ] [serial] Run `cargo test --release --example wasm-optimization-profiles` for the aggregation and `cargo run --release --example wasm-optimization-profiles -- WORKLOAD SAMPLES` for both workloads, and record the commit, image, sample counts and raw numbers under `evidence/`. r[wasm_profile_measurement.evidence]
- [ ] [serial] Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, the Octet source gate, strict Cairn validation and the proposal, design and tasks gates, then sync and archive only with this evidence.

## Non-goals

No selector, trained artifact, profile-admission change or runtime backend. The
example is diagnostic and stays out of gate logic.
