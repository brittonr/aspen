## Why

Issue #44: a newly elected leader could complete a linearizable read at its pre-election commit index. The previous leader may already have acknowledged a later write, so the read could miss an acknowledged write. The issue's TLA+ model (`RaftReadIndex.tla`, configurations `Current` and `NoopOnly`) reproduces the violation; the read guard is the safety fix.

## What Changes

- A leader answers a linearizable read with `retryable` until its commit index covers every entry an earlier leader may have committed: the entry at its commit index belongs to its current term, or its commit index already reaches the last entry in its log. Only then does it take a read index.
- A regression test reproduces the issue's interleaving: the old leader commits and acknowledges a write, then votes for a challenger whose commit index is still behind. The challenger reports the read retryable; after a current-term entry commits, its read index covers the acknowledged write.
- A second test elects a challenger that already learned the commit index of every entry it holds; it serves the read without a current-term commit. The existing fresh-leader read test is unchanged, because an empty log is fully committed.

## Impact

- **Files**: `src/fabric_consistency/raft/transition/{client,support}.rs` and the live Raft transition tests.
- **Testing**: the two leader-change read tests, the existing Raft, reliability and collaboration-scope suites that issue linearizable reads, and bounded TLC runs of the issue's model extended with the committed-log condition and a loosened mutant.
- **Owner / consumer**: the replicated Raft engine; the reliability and collaboration-scope services issue linearizable reads through it and already treat `retryable` as an uncertain (`info`) client result.
