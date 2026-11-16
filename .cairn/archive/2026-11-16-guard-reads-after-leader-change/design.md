## Context

`become_leader` appends no entry, and `begin_linearizable_read` records `required_index = commit_index`, completing after a quorum acknowledges the probe. Commit advancement counts only current-term entries (Raft §5.4.2), so a new leader's commit index can lag an index its predecessor already committed and acknowledged.

## Decisions

### Decision: Guard read-index reads on a commit index that covers earlier terms

**Choice:** In `handle_read`, a linearizable read on a leader for which `support::is_commit_index_current` is false returns `ReadDisposition::Retryable` without registering a pending read or sending probes. The predicate holds when `term_at(commit_index)` is the current term, or when `commit_index` reaches the leader's last log index.

**Rationale:** Every entry an earlier leader committed is in the new leader's log (leader completeness). Either condition puts all of them at or below the commit index: a committed current-term entry follows them (Raft dissertation §6.4), and a commit index at the last log index covers the whole log. The read probe then confirms leadership as before.

The issue's model passes with the current-term guard alone (`GuardOnly`) and fails without it even when a leader no-op is added (`NoopOnly`). The committed-log condition is not in that model, so it was checked separately against a copy of the same model (`evidence/tla-2026-11-16/`):

| Configuration | Bounds | Result |
|---|---|---|
| `GuardOnly` (condition off) | MaxTerm 3, MaxLogLen 2 | pass, 4,319,902 distinct states; equals the published count |
| `GuardException` | MaxTerm 3, MaxLogLen 2 | pass, 11,153,290 distinct states |
| `GuardException_Log3` | MaxTerm 2, MaxLogLen 3 | pass, 3,551,442 distinct states |
| `GuardException_Witness` | MaxTerm 3, MaxLogLen 2 | `NoExceptionPostElectionRead` violated: the condition admits post-election reads that must observe earlier-term writes |
| `GuardExceptionMutant` (bound loosened by one index) | MaxTerm 3, MaxLogLen 2 | `ReadLinearizable` violated |

TLC 1.7.4 ran each configuration once. These are bounded model results with atomic, lossless messages, not a proof of the Rust engine.

**Alternative rejected:** the current-term guard alone. It made every newly elected leader, including the first leader of an empty cluster, answer reads retryable until a client write committed. The collaboration-scope flow reads before it can stage its first write, so its three-process test never left its first phase, and the canonical envelope test could not observe a read probe.

**Alternative deferred:** a leader barrier (no-op) entry. Collaboration scope mirrors the Raft index in `currentness.committed_index`, advancing it once per applied command, and binds a read only when `observed_index == last_applied`. A barrier that only the Raft core and application port knew about would leave that mirror one index behind and make every later collaboration command stale. A barrier therefore needs an application-visible index-advance contract for every committed-batch handler.

## Risks / Trade-offs

- Liveness: a new leader that holds entries above its commit index answers linearizable reads `retryable` until a current-term entry commits. The reliability service maps that to `info`, and its clients can still write. Collaboration scope stages every mutation from a current read, so in that state it cannot stage a command until a current-term entry commits. A fresh cluster, and a leader that learned the commit index of its whole log before the leader change, are not affected.
- Closing the remaining gap needs the deferred leader barrier and its application contract. Reopen when a leader-change workload must keep collaboration-scope writes available while its new leader holds uncommitted earlier-term entries.
- No lease reads, leadership transfer or membership change semantics change.
