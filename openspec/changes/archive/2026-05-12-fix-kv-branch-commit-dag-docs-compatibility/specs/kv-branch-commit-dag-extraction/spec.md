## MODIFIED Requirements

### Requirement: KV Branch / Commit DAG representative consumers stay compatible

The `kv-branch-commit-dag` family MUST retain fresh compatibility evidence for representative Aspen consumers, including `aspen-docs --features commit-dag-federation`, before its readiness state is raised from `workspace-internal`.

ID: kv-branch-commit-dag-extraction.docs-federation-compatibility

#### Scenario: Docs federation feature compiles

- GIVEN the `aspen-docs` crate with `commit-dag-federation` enabled
- WHEN `cargo check -p aspen-docs --features commit-dag-federation` runs
- THEN the command MUST pass and the transcript MUST be stored as readiness evidence.

#### Scenario: Branch/DAG reusable graphs remain clean

- GIVEN a fix for docs feature compatibility
- WHEN branch/DAG default and `commit-dag` feature graph checks run
- THEN the evidence MUST show no root app, handler, concrete transport, or `aspen-raft` dependency leaks into reusable branch/DAG graphs.
