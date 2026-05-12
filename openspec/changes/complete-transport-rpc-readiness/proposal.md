## Why

`transport-rpc` is the next high-value extraction seam after the ready-family sweep. The manifest already identifies the reusable default surfaces for `aspen-transport` and `aspen-rpc-core`, but readiness remains `workspace-internal` until downstream fixtures, negative dependency checks, and runtime compatibility evidence are captured together.

## What Changes

- Add downstream-style fixtures for default `aspen-transport` and `aspen-rpc-core` use.
- Capture cargo metadata, forbidden dependency greps, default graph cargo trees, and representative runtime consumer compatibility transcripts.
- Update the extraction manifest, policy inventory, and readiness evidence only when the rails pass.

## Capabilities

### New Capabilities
- `transport-rpc-extraction`: Evidence-backed extraction readiness for Aspen's Iroh transport helpers and RPC core dispatch library.

### Modified Capabilities
- `architecture-modularity`: Transport/RPC inventory and policy rows may advance once evidence is durable.

## Impact

- **Files**: `docs/crate-extraction/transport-rpc.md`, `docs/crate-extraction.md`, `docs/crate-extraction/policy.ncl`, `openspec/changes/complete-transport-rpc-readiness/`.
- **APIs**: No behavior change expected; fixes are limited to feature/default graph boundaries if evidence exposes leaks.
- **Dependencies**: Default reusable graphs must avoid root app/runtime shells and handler bundles.
- **Testing**: Fixture `cargo metadata/check`, `cargo tree`, readiness checker, representative consumer `cargo check`.
