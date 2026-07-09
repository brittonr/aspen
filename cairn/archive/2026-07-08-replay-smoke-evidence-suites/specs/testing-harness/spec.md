# Testing Harness Delta: Replay Smoke for Evidence Suites

## ADDED Requirements

### Requirement: Evidence suites have replay smoke coverage
r[molten.testing.replay_smoke.all_evidence_suites] Evidence-bearing deterministic harness suites SHOULD have replay smoke coverage that runs the suite, replays it from recorded effects when applicable, and reruns it from fresh declared fixtures.

#### Scenario: Deterministic suite smoke passes
- GIVEN an evidence-bearing deterministic suite with declared fixtures and effect records
- WHEN replay smoke executes run, replay, and fresh rerun
- THEN the canonical report refs, final-state refs, effect-log refs, and required trace or receipt refs match the declared replay identity

### Requirement: Fresh reruns compare canonical refs
r[molten.testing.replay_smoke.fresh_rerun] Replay smoke comparisons MUST use canonical refs and receipts rather than rendered logs, wall-clock timing, temporary paths, or process ids.

#### Scenario: Temporary path variance is ignored only when declared
- GIVEN a fresh rerun produces a different temporary diagnostic path but the same semantic report and final-state refs
- WHEN replay smoke compares the runs
- THEN the semantic refs match and the path variance is ignored only if an explicit variance declaration exists

### Requirement: Non-replayable suites are excluded visibly
r[molten.testing.replay_smoke.non_replayable_excluded] Suites marked exploratory, live-only, unavailable, or non-replayable MUST be excluded from deterministic pass evidence and SHOULD emit a visible replay-smoke diagnostic.

#### Scenario: Live-only run cannot satisfy deterministic gate
- GIVEN a live-only diagnostic run without a recorded effect log
- WHEN replay smoke evaluates it for deterministic evidence
- THEN the run is excluded with a non-replayable diagnostic even if its rendered status is pass
