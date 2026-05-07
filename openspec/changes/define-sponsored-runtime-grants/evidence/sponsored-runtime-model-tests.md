# Sponsored runtime grant pure model tests

- Change: `define-sponsored-runtime-grants`
- Task: pure model tests
- Started: `2026-05-07T01:36:02Z`
- Completed: `2026-05-07T01:37:06Z`

## Implemented

Added `aspen-runtime-core` tests for:

- bounded resource requests against grant limits;
- validity-window fields and workload/provider/isolation scope anchoring;
- settlement-reference opacity through `RedactedValue::OpaqueHandle`;
- usage receipt redaction and raw-token rejection through `contains_raw_secret`;
- quota arithmetic over reserved + consumed resources and active concurrency.

## Verification

- `rustfmt crates/aspen-runtime-core/src/lib.rs`
- `CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core --all-targets`

Result: 10 runtime-core tests passed.
