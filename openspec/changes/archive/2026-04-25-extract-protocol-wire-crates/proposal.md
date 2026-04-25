## Why

Aspen's wire crates are the public compatibility surface for clients, Forge, jobs, and coordination. They should be independently usable by clients, tools, tests, and protocol gateways without pulling handler registries, root `aspen`, node bootstrap, concrete transport, UI/web binaries, or runtime auth shells. The no-std foundation work reduced several leaks, and coordination protocol is already clean, but the family still lacks a first-class extraction manifest, append-only wire compatibility evidence, and deterministic boundary checks across all protocol crates.

This change makes the wire layer a reusable contract family rather than an accidental byproduct of Aspen's runtime crates.

## What Changes

- Create a protocol/wire extraction manifest covering `aspen-client-api`, `aspen-forge-protocol`, `aspen-jobs-protocol`, and `aspen-coordination-protocol`.
- Register the protocol/wire family in `docs/crate-extraction/policy.ncl` with explicit forbidden runtime/app dependencies and named feature sets.
- Preserve postcard discriminants and serialized compatibility through golden/snapshot tests for public request/response enums and domain protocol types.
- Keep portable auth/ticket references pointed at leaf crates (`aspen-auth-core`, `aspen-hooks-ticket`, protocol crates) instead of runtime shells.
- Compile default, no-default, and wasm-friendly feature sets where supported, and save dependency-boundary evidence.

## Capabilities

### New Capabilities

- `protocol-wire-extraction`: Extraction boundary, compatibility contract, and standalone verification rails for Aspen's protocol/wire crates.

### Modified Capabilities

- `architecture-modularity`: Updates crate extraction inventory and policy for the protocol/wire family.

## Impact

- **Crates modified**: `aspen-client-api`, `aspen-forge-protocol`, `aspen-jobs-protocol`, `aspen-coordination-protocol` if dependency cleanup or compatibility tests require changes.
- **Docs modified**: `docs/crate-extraction.md`, `docs/crate-extraction/policy.ncl`, `docs/crate-extraction/protocol-wire.md`.
- **Consumers verified**: `aspen-client`, `aspen-cli`, `aspen-rpc-core`, `aspen-rpc-handlers`, Forge handlers, jobs handlers, coordination service, and downstream serialization fixture.
- **APIs**: Existing wire enums and protocol types stay compatible. New enum variants remain append-only.
- **Dependencies**: Handler, runtime auth, node bootstrap, concrete transport, and UI/web crates are forbidden from default reusable wire graphs.
