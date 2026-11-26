# Production Raft model check

`verification/tla/AspenRaft.tla` is a bounded TLA+ design model of Aspen's
production replicated Raft transition function,
`fabric_consistency::raft::apply_replica_event`
(`src/fabric_consistency/raft/transition.rs` and `transition/`). The
`production-raft-model` flake check model-checks it with the pinned TLC
**2.19 / TLA+ 1.7.4**. The check exists only for `x86_64-linux`, so it runs in
`nix flake check` on x86_64 Linux only. It is finite-model evidence
about the design, **not** a proof of the Rust code. The internal crate and
requirement identifiers keep the name `molten`.

r[impl molten.consensus.production_raft_formal_model]

## What is modeled

The model starts from the **empty durable state** of the fixed three-voter
group (`STATIC_VOTER_COUNT`, `STATIC_QUORUM_COUNT = 2`): term 0, no vote, empty
logs, nobody leading. Nothing about leaders, logs or commitment is assumed in
`Init`; every safety property below is a checked invariant.

| Rust transition | Model action | Notes |
| --- | --- | --- |
| `election::handle_timeout` | `Timeout`, `SendRequestVote` | Candidate in the next term, votes for itself; RequestVote carries last index/term. |
| `election::handle_request_vote` | `HandleRequestVote` | Grants only with no or the same vote and an up-to-date log (`candidate_log_is_up_to_date`). |
| `election::handle_vote_response`, `become_leader` | `HandleVoteResponse` | Leader once `STATIC_QUORUM_COUNT` votes; `nextIndex` = last + 1. No no-op entry is appended, as in the code. |
| `client::handle_proposal` | `ClientRequest` | Leader appends a current-term entry. |
| `support::append_effect_for` | `SendAppend` | InstallSnapshot when `nextIndex` is at or below the snapshot boundary, else AppendEntries with `prev_log_term = term_at(...).unwrap_or(0)`. |
| `replication::handle_append_entries`, `merge_entries`, `apply_leader_commit` | `HandleAppendEntries` | Steps down; rejects a mismatched previous entry; truncates the conflicting suffix; an overwrite at or below the commit index is an `Err` (the transition is rejected); commit = min(leader commit, last new index). |
| `replication::handle_append_response`, `highest_quorum_index`, `apply_failed_response` | `HandleAppendResponse` | Monotonic `matchIndex`; commits only **current-term** entries replicated on a quorum; backoff `clamp(conflict_index, 1, next - 1)`. |
| `client::handle_create_snapshot` | `CreateSnapshot` | Compacts through the applied (= committed) index. Compacted entries are absent to `term_at` except the boundary. |
| `snapshot::handle_install`, `handle_response` | `HandleInstallSnapshot`, `HandleSnapshotResponse` | Installs only above the commit index; retains the suffix only when the boundary term matches. |
| `client::handle_read` (Linearizable), `begin_linearizable_read` | `ReadStart`, `SendReadProbe` | Requires `is_commit_index_current` (current-term commit, or commit at the last log index); `required_index` = commit index. |
| `read::handle_probe`, `handle_acknowledgement` | `HandleReadProbe`, `HandleReadAck` | Follower acknowledges in its term; the read completes `Current` at `required_index` after a same-term quorum. |
| `observe_higher_term` | every receipt | A higher term resets role, vote, leader and pending reads before dispatch. |
| durable replay (`live_process` restart) | `Restart` | Term, vote, log, snapshot and commit survive; every volatile field resets. |

The network is a set of messages that may be **lost** (`DropMessage`),
**delayed** across any number of local steps, **duplicated** (`Duplicates`),
and, with two messages in flight, **reordered**. An `Err` from the Rust handler
rejects the whole transition, so such a receipt is disabled (the message can
only be dropped).

## Checked invariants

| Invariant | Statement |
| --- | --- |
| `ElectionSafety` | No two servers lead the same term at the same time. |
| `LogMatching` | Logs that agree at an index agree on the whole prefix through it. |
| `LeaderCompleteness` | Every entry committed in a term no later than a leader's term is in that leader's log at the same index (history-based ghost `committed`). |
| `StateMachineSafety` | No two state machines (across servers, restarts, snapshot restores and time) are handed different entries at one index (ghost `applied`). |
| `ReadIndexLinearizable` | A completed ReadIndex read observes every index committed anywhere before the read started. |
| `CommitWithinLog`, `TypeOK` | Structural (`validate_replica_state`). |

## Configurations and seeded bugs

Each mutant removes exactly **one** guard of the Rust transition function
(`Mutation` constant) or the quorum size, at the **same bounds** as a baseline
that passes exhaustively, so a passing baseline is not vacuous. All configs use
three servers, one entry per message and symmetry reduction over servers.

| Config | Bounds | Expected |
| --- | --- | --- |
| `AspenRaft.cfg` | 4 terms, log length 1, 1 message in flight, restarts, duplicates | pass (exhaustive) |
| `AspenRaftCommitPreviousTerm.cfg` | as above; `highest_quorum_index` without the current-term check | `LeaderCompleteness` violated (Raft §5.4.2 / Figure 8) |
| `AspenRaftVoteWithoutUpToDate.cfg` | as above; vote granted without `candidate_log_is_up_to_date` | `LeaderCompleteness` violated |
| `AspenRaftReplication.cfg` | 3 terms, log length 2, 1 in flight, restarts, duplicates | pass (exhaustive) |
| `AspenRaftAppendWithoutPrevMatch.cfg` | as above; AppendEntries accepted without comparing the previous entry's term | `LogMatching` violated (needs three terms) |
| `AspenRaftReads.cfg` | 2 terms, log length 2, 1 in flight, 1 read, restarts, duplicates | pass (exhaustive) |
| `AspenRaftReadWithoutCurrentCommit.cfg` | as above; read without `is_commit_index_current` | `ReadIndexLinearizable` violated |
| `AspenRaftMinorityQuorum.cfg` | as above; `STATIC_QUORUM_COUNT = 1` | `ElectionSafety` violated |
| `AspenRaftReordering.cfg` | 2 terms, log length 1, **2** in flight, 1 read, restarts, duplicates | pass (exhaustive) |

`RaftReadIndexWindow.tla` is the frozen trace-mapping observation window used
by the live-trace diagnostic ([`r[molten.testing.live_raft_trace.model]`](../.cairn/specs/live-raft-read-trace/spec.md));
its bytes are pinned by archived evidence and are unchanged.
Its `Init` starts after a leader change and assumes the new leader holds every
acknowledged write, which is Leader Completeness. `AspenRaft.tla` checks
`LeaderCompleteness` and `ReadIndexLinearizable` from the empty state, at the
bounds listed here, rather than assuming them; the window model itself runs in
the same check.

`read::handle_probe` rejects a probe whose required index is beyond the
follower's log. The model does not encode that rejection as a guard: an `Err`
loses the probe, which the lossy network already allows, so the checked
behaviours cover the code both with and without that check.

## Running

```sh
nix build .#checks.x86_64-linux.production-raft-model --no-link --print-out-paths -L
```

`nix/production-raft-model.nix` first runs the synthetic reader tests in
`verification/tla/raft-campaign.test.mjs`, then `raft-campaign.mjs`, which runs
TLC on every configuration (baselines with all build cores, mutants with one
worker so the first reported violation is deterministic), parses each transcript
with the fast-path campaign's TLC reader, and fails closed unless every baseline
exhausts its graph without a violation and every mutant reports exactly its
expected invariant with a multi-state trace. The output holds `campaign.json`
(arguments, durations, state counts and verdicts), the model and configs, and
each run's raw stdout/stderr. TLC's state store is scratch, never evidence.

## Results

Observed on 2026-11-26 by building the check on an x86_64 Linux builder
(TLC 2.19, 642 s of checker time in total):

| Run | Workers | Generated | Distinct | Verdict |
| --- | --- | --- | --- | --- |
| `AspenRaft.cfg` | all cores | 12,094,701 | 1,465,811 | pass, empty queue, depth 41 (33 s) |
| `AspenRaftCommitPreviousTerm.cfg` | 1 | 14,600,870 | 1,910,923 | `LeaderCompleteness`, 29-state trace (260 s) |
| `AspenRaftVoteWithoutUpToDate.cfg` | 1 | 107,231 | 21,464 | `LeaderCompleteness`, 13-state trace |
| `AspenRaftReplication.cfg` | all cores | 44,361,336 | 5,360,916 | pass, empty queue, depth 51 (80 s) |
| `AspenRaftAppendWithoutPrevMatch.cfg` | 1 | 1,316,819 | 237,401 | `LogMatching`, 19-state trace |
| `AspenRaftReads.cfg` | all cores | 29,642,788 | 3,670,825 | pass, empty queue, depth 49 (51 s) |
| `AspenRaftReadWithoutCurrentCommit.cfg` | 1 | 466,976 | 90,410 | `ReadIndexLinearizable`, 17-state trace |
| `AspenRaftMinorityQuorum.cfg` | 1 | 4,225 | 1,129 | `ElectionSafety`, 9-state trace |
| `AspenRaftReordering.cfg` | all cores | 119,345,678 | 11,413,773 | pass, empty queue, depth 42 (163 s) |
| `RaftReadIndexWindow.cfg` | all cores | 81 | 69 | pass, empty queue |

Mutant counts are the prefix explored up to the first violation. The
previous-term mutant needs all four terms: two competing leaders each append
at index 1, the earlier entry is re-replicated and counted as committed by a
later leader, and the holder of the newer conflicting entry is then elected
without it. No baseline found a counterexample in the current transition
function at these bounds.

## Non-claims

- **Not the Rust code.** The model is hand-written from the transition
  function; no refinement mapping or trace validation ties it to the
  implementation. Bugs in effect execution, durability ordering, the executor,
  Iroh transport, timers or the application are out of scope. The live-trace
  diagnostic is the separate route to implementation conformance.
- **Bounded.** Small terms, logs, reads and network capacity; symmetry over
  servers. A larger instance can contain counterexamples these bounds miss.
- **Abstractions.** Entries are identified by term (commands, request refs and
  the duplicate window are not modeled); one pending read per leader; snapshots
  carry the compacted log prefix as a ghost; persistence is atomic and
  uncorrupted (no split durable writes or torn snapshots).
- **Not modeled.** Membership change, rejoin/recovery rounds and vote fencing
  (`ReplicaRejoin`), drain/stop lifecycle, lease and local-stale reads, fencing
  and config epochs, message batching beyond one entry, and liveness or timing.
- No release authority, production linearizability or fault-tolerance claim
  follows from a passing bounded check.
