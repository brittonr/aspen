## Why

Molten will run Wasm, Steel, and native actor/service artifacts behind effect handlers. Aspen's plugin host ABI discipline is useful prior art: host functions, permissions, lifecycle exports, version history, namespace isolation, result encoding, resource limits, and health checks must be documented and testable. Molten needs the same discipline, but mapped to Preserves envelopes and effect manifests.

## What Changes

- Define a versioned Molten host ABI contract for sandboxed/external execution adapters.
- Map hostcalls to declared effects and admitted handler bindings, not ambient runtime access.
- Require explicit result/error encoding, declared lifecycle callbacks, health checks, shutdown/remove, upgrade compatibility, and cleanup receipts. Event/timer and request/turn callbacks remain future ABI extensions.
- Enforce namespace/resource/capability isolation per artifact and execution through permission, lifecycle, and hostcall receipts.
- Record ABI version through manifest-bound install, permission, lifecycle, health, removal, upgrade, and hostcall receipts.
- Keep Aspen as prior art; do not copy its JSON/RPC ABI shape into Molten.

## Impact

This gives Molten a stable adapter boundary for Wasm, Steel, native-adapter, and future plugins. The completed milestone defines a minimal Preserves-first ABI contract with artifact-backed manifests, canonical result/error values, declared lifecycle callbacks, storage-read hostcall coverage, ambient-network denial, permission/resource/supply-chain gates, health/cleanup receipts, and compatibility-gated upgrades. Broader send/observe/blob/storage/trace/clock/random hostcall wrappers require explicit future declarations and do not become ambient APIs.
