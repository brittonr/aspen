## Why

Dogfood receipts now prove that a full self-hosting run completed and can publish evidence into Aspen KV, but skeptical operators still have to start from the dogfood wrapper to understand the underlying CI/deploy evidence. The next trust boundary is native CI run evidence: a run should expose a stable, receipt-shaped summary directly from Aspen's CI domain.

## What Changes

- Add a native CI run receipt RPC and CLI command.
- Project existing persisted `PipelineRun` state into a schema-versioned, operator-facing receipt JSON.
- Include run identity, repo/ref/commit, created/completed times, stages/jobs, terminal status, and error context.
- Document this as the first native CI/deploy receipt slice; dogfood can continue to reference run IDs but operators can inspect CI evidence directly.

## Non-Goals

- No new storage schema for separate receipt rows in this slice; the receipt is generated from the existing Raft-backed pipeline run KV record.
- No artifact download or log embedding. Job IDs remain handles for `ci logs`/`ci output`.
- No broad deploy executor redesign. Deploy stages already recorded in `PipelineRun.stages` are included.

## Impact

- Files: `crates/aspen-client-api`, `crates/aspen-ci-handler`, `crates/aspen-cli`, docs/OpenSpec.
- APIs: append-only client RPC request/response variants for CI receipt readback.
- Testing: targeted cargo checks/tests, CLI help smoke, OpenSpec validation, diff check.
