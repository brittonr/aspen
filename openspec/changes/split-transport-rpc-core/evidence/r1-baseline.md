# R1 Baseline: split transport/RPC core

- Target dir: `target`
- Transport default compile: see `evidence/r1-baseline-logs/transport-cargo-check.txt`
- RPC core default compile: see `evidence/r1-baseline-logs/rpc-core-cargo-check.txt`
- Representative consumer compiles: raft-network, raft, cluster bootstrap, client, rpc-handlers logs under `evidence/r1-baseline-logs/`
- Dependency trees: `transport-cargo-tree.txt`, `rpc-core-cargo-tree.txt`
- Source import audits: `transport-source-imports.txt`, `rpc-core-source-imports.txt`

## Baseline classification

- `aspen-transport` default currently includes generic Iroh stream helpers and runtime Raft/log-subscriber/trust/sharding/auth integrations in one crate manifest.
- `aspen-rpc-core` default currently includes handler traits plus concrete `ClientProtocolContext` service bundles that reference Raft, transport, sharding, coordination, hooks, auth, metrics, and optional domain crates.
- Representative consumers compile against implicit normal dependencies before the split; the implementation must preserve runtime behavior by adding explicit feature opt-ins.
