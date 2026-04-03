## Why

The job orchestration system has 22K lines of code across subsystems (dependency DAGs, distributed worker pools, cron scheduling, dead letter queues, saga executor, workflow engine, job affinity, replay) but only the happy-path CI pipeline and a bundled durable-workflow test have NixOS VM integration tests. The lower-level primitives that CI depends on are tested only at the unit level. A regression in DAG resolution or DLQ processing would pass `cargo nextest` but fail in production where real async, real disk, and real process boundaries exist.

## What Changes

- Add NixOS VM integration tests for job orchestration subsystems that currently lack them:
  - **Job dependency tracker**: submit a DAG of jobs, verify execution respects ordering and handles upstream failures
  - **Distributed worker pool**: multi-node cluster where jobs route to workers on different nodes
  - **Scheduler**: cron and delayed job scheduling with time progression
  - **Dead letter queue**: jobs that exceed retry limits land in DLQ, can be inspected and retried
  - **Saga executor**: multi-step saga with intentional failure triggers compensation in LIFO order (standalone, not bundled in durable-workflow)
  - **Workflow engine**: multi-step workflow definitions with conditional transitions
  - **Job affinity**: jobs with affinity constraints route to matching workers
  - **Job replay**: record execution, replay deterministically, verify identical output
- Add a test binary crate (`aspen-job-orchestration-test`) that exercises each subsystem against a live single-node or multi-node cluster
- Wire the new tests into `nix/tests/` and the flake checks

## Capabilities

### New Capabilities

- `job-orchestration-vm-tests`: NixOS VM integration tests covering job dependency tracking, distributed worker pools, scheduling, DLQ, saga, workflow engine, affinity, and replay

### Modified Capabilities

## Impact

- `nix/tests/`: 4-6 new `.nix` test files
- New test binary crate under `crates/` or `tests/`
- `flake.nix`: new check entries
- No changes to production code — tests only
