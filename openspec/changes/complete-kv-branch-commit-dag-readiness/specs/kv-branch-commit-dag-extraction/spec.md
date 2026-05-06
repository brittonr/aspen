# Complete KV branch and commit DAG readiness evidence Delta

## ADDED Requirements

### Requirement: Commit DAG reusable graph avoids Raft [r[kv-branch-commit-dag-extraction.commit-dag-avoids-raft]]
The commit DAG reusable graph MUST avoid normal dependencies on `aspen-raft`, node bootstrap, runtime handlers, and app shells.

#### Scenario: Commit DAG reusable graph avoids Raft evidence [r[kv-branch-commit-dag-extraction.commit-dag-avoids-raft.evidence]]
- GIVEN the default and minimal feature graphs for `aspen-commit-dag` are checked
- WHEN cargo dependency evidence is captured
- THEN normal dependencies SHALL exclude `aspen-raft` and runtime shells while preserving hash and serialization contracts.

### Requirement: KV branch boundaries are feature-gated [r[kv-branch-commit-dag-extraction.kv-branch-boundaries-feature-gated]]
KV branch overlay behavior MUST keep branch/DAG integration behind documented feature or adapter boundaries with downstream fixture evidence.

#### Scenario: KV branch boundaries are feature-gated evidence [r[kv-branch-commit-dag-extraction.kv-branch-boundaries-feature-gated.evidence]]
- GIVEN downstream fixtures exercise branch overlay and commit DAG APIs
- WHEN default, no-default, and feature-enabled checks run
- THEN each runtime or Raft dependency SHALL be absent by default or tied to an explicit documented feature with compatibility evidence.
