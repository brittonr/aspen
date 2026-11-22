# Bounded live Raft read-index check

<!-- r[verify molten.testing.live_raft_trace.model] -->
<!-- r[verify molten.testing.live_raft_trace.checking] -->

## Observed inputs

Pueue task 77 ran the exact ignored test below on Linux x86-64, in a detached source snapshot based on `31794a3ca192415191d6b6ed294e619c22ef78a5`. It used Rust `1.98.0-nightly (31a9463c6 2026-05-25)`, LLVM `22.1.6`, and Cargo.lock BLAKE3 `f61823d53822090997fb5da2ab063846d1646f510a72ba783eb7de262e242912`. The only source overlays were the three captured fixture files; their BLAKE3 digests are:

| File under `src/fabric_consistency/raft/live_cluster/` | BLAKE3 |
| --- | --- |
| `mod.rs` | `9b8a94e89be91ab633c604fa905ea3bfc337c5e1d57906bb38b8335c9cb3d266` |
| `workflow.rs` | `f367d10630a10be739c2c90e5b5721e9cc316768c8652fc45f396519cd28f7d7` |
| `trace_capture.rs` | `efb27f2d0389dd991196b8b263d3b743c9901469ba5d18b8fdd5581cc8214e0f` |

```sh
CARGO_BUILD_JOBS=4 CARGO_PROFILE_TEST_DEBUG=0 timeout --kill-after=20s 2100s \
  cargo test --offline --locked -p molten --lib \
  fabric_consistency::raft::live_cluster::trace_capture::capture_live_raft_read_index_trace \
  -- --exact --ignored --nocapture
```

One test passed in 1.46 s and retained a single canonical [1,656-byte observation](clean/capture.preserves) from three independent Iroh endpoints and three capability-rooted local stores in one process. A acknowledged index 1 in term 1. Newly elected B had that log entry but commit/applied indexes 0. B returned `Retryable` at 0, then committed and applied its term-2 entry at 2. With its own acknowledgement and the delivered matching acknowledgement from A, B returned `Current` at index 2. The [mapped five states](clean/sample.json) retain those indexes and the distinct acknowledgers. Clean libtest BLAKE3 `63f1641114acbf03d52010da9d481b243ee4b287547fec421381d540b3513997`; diagnostic log `1c04ff770975a9564d8df509937566955f47a16c2963f0e74026403a81b03463`. The earlier shared-target attempts selected zero tests and supplied no trace evidence; the successful run used a detached checkout and separate target.

## Reached negative

Task 80 ran the same selection with a 1,900 s outer deadline in a separate disposable source snapshot. The retained [patch](guard-control/isolated-test.patch) changes the fixture to issue its read before B's next commit and compiles out only the current-term guard in that test build. The guard remains in Aspen's published implementation. The selected test ran and no state was hand-edited in the capture: the actual live service returned `Current` at index 0 with acknowledgers B and A, below the independently observed prior acknowledgement at index 1. The [1,099-byte capture](guard-control/capture.preserves) and [three-state projection](guard-control/sample.json) preserve this divergence. Negative binary BLAKE3 `6faba93b00f48fb727eb4fc880573f53e67df7b02fe06e8fdb5b68d32436207a`; patch `3c8a0a56f73d626e39694b15df3e13daf28b56bbda87ac5ad64941760946e9fc`; log `43d5aec7547fdf83b6a1bb54e1b3da2178316c8a277314c7b805b2b701a74dd9`.

## Independent checker

[Trace Lab](https://github.com/OnixResearch/trace-lab/tree/b7dba0b287ef7484128ae7b5e556599ee8ef9056) owns the mapper and driver. Its locked executable BLAKE3 was `1482917d42ccf38007f634ea5c1b05e12a9c4b0108089d52bb0cccd08e364290`. The pinned [Dive revision `0fc00914`](https://github.com/Chelis-Lang/dive/tree/0fc00914b7cd93d88e97e5d7d66aac805c06228b) was built with `cargo build --locked -p dive-cli --bin dive` from that exact source using `rustc 1.99.0 (b940084d7 2026-09-28)`. Dive Cargo.lock BLAKE3 `ca86b47daeb3bd2acbd0e6564b09c499c627029d5a61fdb007a3730004fdffe1`; Dive executable `2a39fc7ca7c05679c7521c1f390183dc8182c5f9e2e4b7eca2ce50bfe8e7f09c`. This is a measured local build, not an Octet admission or a signed reusable tool release.

The same model and config were used in both commands: `dive validate-trace RaftReadIndexWindow.tla --config RaftReadIndexWindow.cfg --trace sample.json --compat=tlc --sample --seed 0 --fp 0 --output json`. The driver independently selects input bytes and checker digest; it sets a 20-second process deadline and 65,536-byte limits for each stream.

| Input | Observed result |
| --- | --- |
| [Clean capture](clean/run.json) | Exit 0, schema 4, five states, [`valid`](clean/dive.stdout). |
| [Test-only guard control](guard-control/run.json) | Exit 1, schema 4, three states, [`invalid`](guard-control/dive.stdout). The transition into the pending state at zero-based index 1 does not satisfy `Next`: the new leader is not yet ready. The separately captured terminal row violates `ReadLinearizable`, but the checker stops at the earlier illegal edge; it does not report a named invariant verdict on the terminal row. |
| [Clean capture under the guard-control selector](wrong-arm/run.json) | Exit 3, `mapping-unsupported` at the mismatched schema label. No checker report was emitted. |

The existing live-cluster election/commit/read/catch-up test independently passed on the clean test executable. `cargo fmt --check` passed; task 79 passed `cargo clippy --offline --locked -p molten --lib --profile test -- -D warnings` (log BLAKE3 `e37f95ad8fccb0a28b64d3fc66f928f03e0dbee67fb2e445c15cf799e56718ba`). The finite TLC model run in [tasks.md](../tasks.md#model-only-verification) remains separate. Native Cairn proposal, design, tasks and strict validation passed. The publication guard accepted the copied evidence.

## Limits

This checks one explicitly selected local read-index window and one reached test-only guard control. It is not a check of arbitrary elections, storage, restart, application values, multi-process/guest network faults, liveness, replay, or production/release readiness. It changes neither the shipped Raft path nor ConsensusFastPath and Quint defaults.
