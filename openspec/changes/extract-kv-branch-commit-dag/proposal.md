## Why

`aspen-kv-branch` and `aspen-commit-dag` are the next reusable layer above the newly extracted Raft KV stack: copy-on-write KV overlays, speculative branch commits, chain-hashed commit history, fork/diff support, and GC. Aspen already uses this family from jobs, deploy, FUSE, docs, and CLI paths, but the family is not extraction-ready because `aspen-commit-dag` still imports hash helpers through `aspen-raft`, and the combined default/feature graph has not been proven independent from Aspen app/runtime shells.

This split gives Aspen an independent branch-and-history library over any `KeyValueStore`, while Aspen continues to consume it for speculative execution, deploy state, and user-facing branch workflows.

## What Changes

- Remove `aspen-commit-dag`'s dependency on `aspen-raft` by moving the chain hash helper surface (`ChainHash`, genesis hash, hex conversion, constant-time comparison, and commit-hash pure helpers) into `aspen-commit-dag` or a leaf helper owned by the family.
- Keep `aspen-kv-branch` default features independent from `aspen-commit-dag`, with the existing `commit-dag` integration remaining a named opt-in feature.
- Add extraction manifests and readiness-policy entries for `aspen-kv-branch` and `aspen-commit-dag`.
- Add a downstream-style consumer fixture that uses the branch overlay and commit DAG over only `aspen-traits` / `aspen-kv-types` style contracts, without root `aspen`, `aspen-raft`, handlers, binaries, or concrete transport.
- Save compile, dependency-boundary, source-audit, consumer-compatibility, and test evidence for both default and `commit-dag` feature sets.

## Capabilities

### New Capabilities

- `kv-branch-commit-dag-extraction`: Extraction boundary, dependency contract, and standalone verification rails for the branch overlay and commit DAG family as reusable libraries independent of Aspen's Raft compatibility shell.

### Modified Capabilities

- `architecture-modularity`: Updates the crate extraction inventory with the branch/DAG family readiness state, manifest links, owner status, and next action.

## Impact

- **Crates modified**: `crates/aspen-commit-dag`, `crates/aspen-kv-branch`, optional helper crate if a leaf hash crate is chosen.
- **Docs modified**: `docs/crate-extraction.md`, `docs/crate-extraction/policy.ncl`, new or expanded manifest under `docs/crate-extraction/`.
- **Consumers verified**: `aspen-jobs` with `kv-branch`, `aspen-ci-executor-shell` with `kv-branch`, `aspen-deploy` with `kv-branch`, `aspen-fuse` with `kv-branch`, `aspen-docs` with `commit-dag-federation`, `aspen-cli` with `commit-dag`, and a downstream fixture.
- **APIs**: Public branch overlay and commit DAG APIs remain stable. Hash helper paths may move from `aspen_raft::verified::*` to the branch/DAG family surface for this crate's internal use.
- **Dependencies**: `aspen-commit-dag` loses the normal dependency on `aspen-raft`; no root app or concrete transport dependencies are introduced.
