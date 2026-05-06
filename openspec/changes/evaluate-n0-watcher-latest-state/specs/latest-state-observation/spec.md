## ADDED Requirements

### Requirement: Latest-State Watcher Evaluation [r[latest-state-observation.evaluation]]

Aspen MUST evaluate `n0-watcher` only through a targeted local prototype or explicit no-adoption comparison before accepting it as a dependency.

#### Scenario: Candidate seam selected [r[latest-state-observation.evaluation.candidate-selected]]

- GIVEN existing Aspen code that propagates changing local state to observers
- WHEN the seam is selected for evaluation
- THEN the evaluation SHALL document why latest-value semantics are correct for that seam
- AND it SHALL identify why missed intermediate values do not violate correctness

#### Scenario: No blanket adoption [r[latest-state-observation.evaluation.no-blanket-adoption]]

- GIVEN `n0-watcher` has not yet been proven in Aspen code
- WHEN dependency changes are made
- THEN the dependency SHALL be added only to the selected crate or not added at all
- AND workspace-wide dependency bundles SHALL remain unchanged unless later evidence justifies expansion

### Requirement: Latest-State Semantics Boundary [r[latest-state-observation.semantic-boundary]]

Aspen MUST use latest-state watchers only for local state where observers require the newest value and do not require every intermediate transition.

#### Scenario: Slow observer skips values [r[latest-state-observation.semantic-boundary.slow-observer]]

- GIVEN a watcher observes a value that changes repeatedly
- WHEN the watcher is slower than the writer
- THEN tests SHALL allow the watcher to skip intermediate values
- AND tests SHALL require the watcher to observe or converge on the latest value according to the seam contract

#### Scenario: Durable stream rejected [r[latest-state-observation.semantic-boundary.durable-stream-rejected]]

- GIVEN a Raft log, audit stream, CI/job log, Forge event stream, hook stream, or other ordered durable event source
- WHEN `n0-watcher` is considered for that source
- THEN the design SHALL reject the use unless a separate spec proves that skipped intermediate values are harmless

### Requirement: Dependency Boundary Evidence [r[latest-state-observation.dependency-boundary]]

Aspen MUST capture dependency-boundary evidence before keeping `n0-watcher` in any implementation crate.

#### Scenario: Core boundary protected [r[latest-state-observation.dependency-boundary.core-protected]]

- GIVEN Aspen has alloc-only and foundational core crate boundaries
- WHEN `n0-watcher` is added to a candidate crate
- THEN dependency-tree evidence SHALL show that the dependency does not leak into `aspen-core --no-default-features` or other protected alloc-only paths

#### Scenario: Tokio comparison recorded [r[latest-state-observation.dependency-boundary.tokio-comparison]]

- GIVEN `tokio::sync::watch` can provide similar latest-value behavior
- WHEN the prototype is reviewed
- THEN the acceptance evidence SHALL state whether `n0-watcher` materially improves ergonomics, resource-bound clarity, or code simplicity over the existing Tokio primitive
