# R1 Baseline

Baseline commands captured under `r1-baseline-logs/`.

Expected current blocker before implementation:
- `aspen-commit-dag` has a normal `aspen-raft` dependency and source imports from `aspen_raft::verified`.
- `aspen-kv-branch` keeps `commit-dag` behind its named feature.
