## ADDED Requirements

### Requirement: Replay coverage matrix summarizes subsystem readiness
r[molten.determinism.replay_coverage.matrix] Molten SHOULD emit a canonical replay coverage matrix that records subsystem, workflow, replay eligibility, positive replay evidence refs, negative evidence refs, replay index refs when available, and caveat refs.

#### Scenario: Complete matrix passes readiness
- GIVEN every required replay coverage row has replay eligibility, positive replay evidence, negative evidence, and valid refs
- WHEN the replay coverage matrix is generated
- THEN the matrix decision is `pass`
- AND the matrix records each subsystem/workflow exactly once.

#### Scenario: Missing negative evidence denies readiness
- GIVEN a replay coverage row has positive replay evidence but no required negative tamper or exclusion evidence
- WHEN the replay coverage matrix is generated
- THEN the matrix decision is `deny`
- AND diagnostics identify the subsystem and missing evidence class.

### Requirement: Replay smoke suites cover representative subsystems
r[molten.determinism.replay_coverage.subsystem_smoke] Molten SHOULD provide replay smoke evidence for representative harness, node-control, job worker, coordination, remote dataspace, vat, retention, and dogfood release workflows.

#### Scenario: Node-control workflow has replay row
- GIVEN a node-control workflow bundle replay smoke case emits replay verification evidence
- WHEN the coverage matrix is generated
- THEN the node-control row records the workflow, replay verify ref, negative evidence ref, and any caveats.

#### Scenario: Diagnostic-only subsystem is excluded from pass evidence
- GIVEN a subsystem emits live-only or diagnostic-only evidence without deterministic replay support
- WHEN the coverage matrix is generated
- THEN the subsystem row records diagnostic-only or non-replayable eligibility
- AND the row cannot satisfy deterministic replay readiness.

### Requirement: Replay readiness summaries remain evidence-only
r[molten.determinism.replay_coverage.release_readiness_summary] Replay readiness summaries MUST NOT replace individual replay verification, replay rollup, replay index, subsystem gate, source-gate, policy, provenance, authority, transport, resource, release, or retention evidence.

#### Scenario: Summary alone cannot satisfy gate
- GIVEN a replay coverage matrix with a passing summary
- WHEN a gate requires a replay verification receipt for a specific subsystem run
- THEN the summary alone is insufficient
- AND the gate still requires the referenced replay receipt or subsystem gate evidence.

### Requirement: Non-replayable evidence is explicitly classified
r[molten.determinism.replay_coverage.non_replayable_exclusions] Replay coverage rows MUST classify exploratory, live-only, or ambient-state-dependent runs as diagnostic-only or non-replayable and exclude them from deterministic readiness counts.

#### Scenario: Exploratory pass is excluded
- GIVEN an exploratory run has rendered status `pass` but lacks deterministic replay evidence
- WHEN coverage readiness is computed
- THEN the run is classified as non-replayable or diagnostic-only
- AND it is not counted as positive deterministic replay evidence.

### Requirement: Replay coverage behavior is tested
r[molten.determinism.replay_coverage.tests] Molten SHOULD test complete coverage, missing positive evidence, missing negative evidence, duplicate rows, stale refs, diagnostic-only exclusion, and catalog/readiness readback behavior.

#### Scenario: Stale matrix ref denies
- GIVEN a replay coverage row references evidence whose supplied value hashes to a different ref
- WHEN matrix validation runs
- THEN readiness denies
- AND diagnostics include expected and actual refs.
