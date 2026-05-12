## Why

The Redb Raft KV type, storage, and facade layers are extraction-ready, but the Iroh/IRPC adapter (`aspen-raft-network`) remains `workspace-internal` because transitive paths through `aspen-transport` and `aspen-sharding` still reach app concerns. A bounded adapter-boundary change can unlock the final reusable networking layer without touching the app compatibility shell.

## What Changes

- Inventory current `aspen-raft-network` default/no-default feature graph leaks.
- Feature-gate concrete transport, sharding, and app-adjacent paths behind named adapter features.
- Add negative boundary checks and compatibility evidence for Aspen runtime consumers.

## Capabilities

### Modified Capabilities
- `redb-raft-kv-extraction`: Adapter readiness must be proven independently from the Aspen app compatibility shell.

## Impact

- **Files**: `crates/aspen-raft-network`, `crates/aspen-transport`, `crates/aspen-sharding` feature wiring if needed, extraction docs/evidence.
- **APIs**: Adapter feature names may become more explicit; runtime compatibility bundles should keep existing Aspen users compiling.
- **Dependencies**: Default adapter graph must not pull root app, handlers, cluster bootstrap, or unrelated runtime shells.
- **Testing**: `cargo check -p aspen-raft-network --no-default-features`, feature checks, cargo tree negative scans, readiness checker.
