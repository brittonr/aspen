## Why

Dogfood receipts currently prove local orchestration because they are persisted beside the local cluster directory. The next self-hosting frontier is proving Aspen can also carry its own dogfood evidence while the cluster is alive, so operators do not have to trust only `/tmp` files or scrollback.

## What Changes

- Add read-only/write-on-demand receipt publication commands under `aspen-dogfood receipts`.
- Publish validated canonical receipt JSON into the running Aspen cluster KV store under a deterministic dogfood evidence key.
- Retrieve a published receipt by run id from the running cluster and render it through the same validated show path.
- Keep local receipt files as the durable fallback; do not store tickets or credential files in the published value.

## Impact

- **Files**: `crates/aspen-dogfood/src/main.rs`, `docs/deploy.md`, `openspec/specs/dogfood-evidence/spec.md`.
- **APIs**: Adds `receipts publish <run-id-or-path>` and `receipts cluster-show <run-id> [--json]`.
- **Testing**: Dogfood package tests for key naming, publish/read response interpretation, and existing receipt checks; CLI smoke where possible.
