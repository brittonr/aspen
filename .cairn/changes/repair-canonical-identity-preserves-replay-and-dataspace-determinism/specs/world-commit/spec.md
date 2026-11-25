# World commit deltas

## ADDED Requirements

### Requirement: World replay validates observation binding before consumption, binds trace bytes, and compares full step roots
r[molten.world_commit.replay_sealed_observation_and_full_comparison] `SealedReplayObservations::observe` MUST validate `(source_id, operation_id, request_ref)` before removing the pending observation and record any mismatch as a violation that fails `finish_step`; `world_replay_trace_member` MUST verify `record.bytes` matches `trace` and `trace.trace_ref`; and `compare_step` MUST verify `roots` and `field_differences` even when `commit_ref` matches.

#### Scenario: Mismatched observation demand cannot be swallowed before finish_step
- GIVEN a replay transition makes an observation demand with a mismatched `request_ref` and ignores the returned `Err`
- WHEN `finish_step` executes
- THEN the recorded mismatch violation MUST fail `finish_step`.

#### Scenario: Matching commit_ref with differing roots or field_differences flags divergence
- GIVEN `observation.actual.commit_ref == step.expected_successor` while `observation.actual.roots` differs from `expected_commit.roots`
- WHEN `compare_step` runs
- THEN it MUST report divergence rather than returning `Ok(None)`.
