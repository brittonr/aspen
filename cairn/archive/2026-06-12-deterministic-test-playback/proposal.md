## Why

Molten's architecture depends on policy-gated distributed execution, artifact sync, effect handlers, typed storage, transcripts, and evidence. Those systems are only trustworthy if Molten can reproduce what happened and identify exactly where a replay diverges. Deterministic test and playback should therefore be a central runtime law, not an optional testing feature.

Without a deterministic contract, actor scheduling, clocks, randomness, network timing, storage ordering, policy responses, or adapter behavior can make tests flaky and traces non-replayable. That would weaken Cairn receipts, upgrade-session validation, executable transcripts, distributed job debugging, and incident analysis.

## What Changes

- Establish Molten's deterministic playback law:

  ```text
  same artifacts
  + same dependency closure
  + same initial state
  + same policy refs
  + same handler profile
  + same seed or recorded effect log
  = same trace records, receipts, outputs, and final state hash
  ```

- Require all nondeterminism to enter through declared, admitted effect handlers.
- Add deterministic scheduler semantics for local actor turns and test playback.
- Add logical clock, seeded randomness, deterministic chaos, record, and replay handler profiles.
- Require canonical trace records for every committed turn and significant adapter event.
- Require snapshots and state hashes sufficient to replay from a known initial state.
- Add first-divergence diagnostics for replay: input hash, effect request, effect response, committed action, receipt, or state hash mismatch.
- Integrate deterministic playback with executable transcripts, evaluation cache, effect manifests, typed storage, remote sync, job DAGs, and upgrade sessions.

## Impact

This becomes a cross-cutting acceptance law for Molten. The first milestone can be local and in-process: two native actors, deterministic logical time/random handlers, a canonical turn journal, record/replay profiles, and tests proving the same seed and initial state reproduce identical traces and final state hashes.
