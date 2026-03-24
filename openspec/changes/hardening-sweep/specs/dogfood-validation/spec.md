## ADDED Requirements

### Requirement: Dogfood full-loop completes after hardening sprint

The dogfood pipeline (`nix run .#dogfood-local -- full-loop`) SHALL complete all phases (start → push → build → deploy → verify) without manual intervention on the current `main` branch.

#### Scenario: Single-node full-loop passes

- **WHEN** `nix run .#dogfood-local -- full-loop` is executed with `ASPEN_NODE_COUNT=1`
- **THEN** the pipeline completes with exit code 0 and the verify phase confirms the CI-built binary matches the locally-built binary

### Requirement: Dogfood phases are individually runnable

Each dogfood phase (start, push, build, deploy, verify, stop) SHALL be independently executable against a running cluster, allowing incremental debugging when the full-loop fails.

#### Scenario: Phase isolation after failure

- **WHEN** the full-loop fails during the `build` phase
- **THEN** the operator can re-run `nix run .#dogfood-local -- build` without restarting the cluster
