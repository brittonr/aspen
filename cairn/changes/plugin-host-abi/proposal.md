## Why

Molten will run Wasm, Steel, and native actor/service artifacts behind effect handlers. Aspen's plugin host ABI discipline is useful prior art: host functions, permissions, lifecycle exports, version history, namespace isolation, result encoding, resource limits, and health checks must be documented and testable. Molten needs the same discipline, but mapped to Preserves envelopes and effect manifests.

## What Changes

- Define a versioned Molten host ABI contract for sandboxed/external execution adapters.
- Map hostcalls to declared effects and admitted handler bindings, not ambient runtime access.
- Require explicit result/error encoding, lifecycle callbacks, health checks, shutdown, and optional event/timer surfaces.
- Enforce namespace/resource/capability isolation per artifact and execution.
- Record ABI version, effect manifest, permissions, lifecycle events, and hostcall receipts.
- Keep Aspen as prior art; do not copy its JSON/RPC ABI shape into Molten.

## Impact

This gives Molten a stable adapter boundary for Wasm and future plugins. The first milestone can define a minimal ABI for send/observe/blob/storage/trace/clock/random through Preserves-encoded effect requests and lifecycle callbacks.
