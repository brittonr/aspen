## Phase 1: Implementation

- [x] [serial] Reproduce the issue's interleaving as a regression test that observes the stale read before the guard. r[molten.consensus.linearizable_reads]
- [x] [serial] Return `retryable` for linearizable reads until the leader's commit index covers earlier terms: a current-term entry at the commit index, or a commit index at the last log entry. r[molten.consensus.linearizable_reads]
- [x] [serial] Check the committed-log condition against the issue's TLA+ read-index model with a reachability witness and a loosened mutant, and retain the model, configurations and TLC output. r[molten.consensus.linearizable_reads]
- [x] [serial] Prove that a read after a current-term commit covers the acknowledged write, and that a new leader whose commit index reaches its log serves the read; record paired mutant observations for both conditions. r[molten.consensus.linearizable_reads]
- [x] [serial] Run the focused Raft, reliability and collaboration-scope tests, formatting, Clippy and source/lifecycle gates before review, sync and archive. r[molten.consensus.linearizable_reads]

## Paired observation (2026-11-16)

With only the regression test applied to the integration tree, `cargo test -p molten --lib live_does_not_complete_stale_read_after_leader_change` failed (0 passed, 1 failed): the new leader began a read-index read instead of reporting the read retryable (assertion at `src/fabric_consistency/raft/parts/tests/p002/body.rs:135`). With the guard applied, the same test passes. This is a native unit-test observation, not a live-cluster qualification.

The current-term guard alone failed the integration run: `vote_and_append_envelopes_roundtrip_exactly` found no read probe after a fresh election, and `three_participant_conflict_partition_restart_snapshot_and_currentness_recovery` never left its first phase (`scope barrier fresh/done`), because collaboration scope reads before it can write. With the committed-log condition both pass.

Mutants of `support::is_commit_index_current`, each run against the final tests and then restored byte-identically:

| Mutant | Test | Result |
|---|---|---|
| committed-log condition removed | `live_new_leader_whose_commit_index_reaches_its_log_serves_read_index_reads` | failed at `p002/body.rs:195` (no pending read) |
| committed-log condition removed | `vote_and_append_envelopes_roundtrip_exactly` | failed at `p000/body.rs:279` (no read probe) |
| condition loosened to `commit_index + 1 >= last_log_index` | `live_does_not_complete_stale_read_after_leader_change` | failed at `p002/body.rs:120` (read not retryable) |
| restored | `fabric_consistency::raft::tests::live_` | 14 passed |

## Verification (2026-11-16)

Integration tree with the #43, #44, #57 and #58 changes applied: `cargo fmt --check`; `cargo test -p molten --lib fabric_consistency` (147 passed, including the three-process collaboration recovery test); `cargo test -p molten --lib collaboration_scope` (36 passed); `cargo test -p molten --lib reliability` (56 passed); `cargo clippy -p molten --lib --bins --tests -- -D warnings` passed. TLC results are in `evidence/tla-2026-11-16/` and summarised in the design. These are native test and bounded-model observations, not live-cluster qualification.
