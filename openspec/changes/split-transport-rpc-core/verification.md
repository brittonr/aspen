# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.
Do not rely on chat-only summaries, `/tmp` logs, or memory.

## Implementation Evidence

- Changed file: `openspec/changes/split-transport-rpc-core/evidence/r1-capture-baseline.sh`
- Changed file: `openspec/changes/split-transport-rpc-core/evidence/r1-baseline.md`
- Changed file: `openspec/changes/split-transport-rpc-core/verification.md`
- Changed file: `openspec/changes/split-transport-rpc-core/tasks.md`

## Task Coverage

- [x] R1 Capture baseline compile, `cargo tree`, feature, and source-import evidence for `aspen-transport`, `aspen-rpc-core`, and representative consumers, classifying each dependency as generic transport, RPC core, domain context, adapter/runtime, test-only, or forbidden. [covers=transport-rpc-extraction.rpc-core-default-avoids-service-graph,transport-rpc-extraction.transport-default-avoids-runtime-shells,transport-rpc-extraction.transport-default-avoids-runtime-shells.iroh-adapter-is-documented-exception,transport-rpc-extraction.transport-default-avoids-runtime-shells.runtime-concerns-feature-gated,transport-rpc-extraction.rpc-core-default-avoids-service-graph.registry-compiles-without-concrete-contexts]
  - Evidence: `openspec/changes/split-transport-rpc-core/evidence/r1-capture-baseline.sh`, `openspec/changes/split-transport-rpc-core/evidence/r1-baseline.md`, `openspec/changes/split-transport-rpc-core/evidence/r1-baseline-logs/transport-cargo-check.txt`, `openspec/changes/split-transport-rpc-core/evidence/r1-baseline-logs/rpc-core-cargo-check.txt`, `openspec/changes/split-transport-rpc-core/evidence/r1-baseline-logs/transport-cargo-tree.txt`, `openspec/changes/split-transport-rpc-core/evidence/r1-baseline-logs/rpc-core-cargo-tree.txt`, `openspec/changes/split-transport-rpc-core/evidence/r1-baseline-logs/transport-source-imports.txt`, `openspec/changes/split-transport-rpc-core/evidence/r1-baseline-logs/rpc-core-source-imports.txt`

## Review Scope Snapshot

Pending broader implementation diff.

## Verification Commands

### `openspec/changes/split-transport-rpc-core/evidence/r1-capture-baseline.sh`

- Status: pass
- Artifact: `openspec/changes/split-transport-rpc-core/evidence/r1-baseline.md`

### `scripts/openspec-preflight.sh split-transport-rpc-core`

- Status: pass
- Artifact: `openspec/changes/split-transport-rpc-core/evidence/r1-openspec-preflight.txt`

## Notes

- Baseline captured before transport/RPC split implementation.
