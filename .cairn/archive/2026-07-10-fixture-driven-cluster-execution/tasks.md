# Tasks: fixture-driven-cluster-execution

## Phase 1: Fixture-derived plan core

- [x] [serial] r[molten.testing.fixture_driven_cluster_execution.fixture_source_of_truth] Define a pure fixture-derived cluster/VM execution plan model from checked scenario metadata.
- [x] [parallel] r[molten.testing.fixture_driven_cluster_execution.observation_gate] Add observation comparison diagnostics for topology, command surface, artifact kind, child ref, unavailable policy, and caveat drift.

## Phase 2: Harness integration

- [x] [serial] r[molten.testing.fixture_driven_cluster_execution.fixture_source_of_truth] Wire local cluster or VM shard planning to consume fixture-derived metadata instead of handwritten scenario shape where practical.
- [x] [parallel] r[molten.testing.fixture_driven_cluster_execution.observation_gate] Add negative fixtures for wrong topology, wrong command surface, missing expected artifact kind, unsupported pass claim, and log-only success.

## Phase 3: Documentation and validation

- [x] [parallel] r[molten.testing.fixture_driven_cluster_execution.fixture_source_of_truth] Document how fixture authors add cluster/VM scenarios and how run observations are gated.
- [x] [serial] r[molten.testing.fixture_driven_cluster_execution.observation_gate] Run focused multinode fixture tests, cluster harness tests, and traceability coverage updates.
