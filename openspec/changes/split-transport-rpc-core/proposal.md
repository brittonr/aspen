## Why

`aspen-transport` and `aspen-rpc-core` are valuable but currently too entangled to be cleanly reused: transport pulls auth, sharding, trust, OpenRaft, core-shell, and iroh concerns together; RPC core pulls concrete Raft, transport, sharding, coordination, optional jobs/CI/Forge, and runtime testing concerns. This makes every consumer of basic ALPN/protocol handling or handler-registry abstractions inherit Aspen node assumptions.

This change defines a staged split so the generic iroh transport and RPC registry pieces can become independent reusable crates while Aspen-specific node contexts remain explicit runtime adapters.

## What Changes

- Create an extraction manifest and readiness policy for the transport/RPC family.
- Split or feature-gate `aspen-transport` so the reusable default surface contains protocol identifiers, connection/stream helper traits, and iroh transport adapter pieces without pulling trust, sharding, auth runtime, root Aspen, or Raft compatibility unless named features opt in.
- Split or feature-gate `aspen-rpc-core` so the reusable default surface contains handler registry/dispatch abstractions and metrics/error plumbing without concrete `RaftNode`, cluster bootstrap, domain services, job/Forge/CI contexts, or concrete transport unless named features opt in.
- Preserve current Aspen node/handler behavior through explicit compatibility features or shell crates.
- Add downstream fixtures for generic iroh transport helpers and handler registry usage without root Aspen app/runtime shells.
- Save dependency-boundary, negative source-audit, and representative consumer evidence.

## Capabilities

### New Capabilities

- `transport-rpc-extraction`: Staged extraction boundary and verification rails for generic iroh transport helpers and RPC handler registry/core abstractions.

### Modified Capabilities

- `architecture-modularity`: Updates crate extraction inventory and policy with transport/RPC readiness state and blocked next steps.

## Impact

- **Crates modified**: `aspen-transport`, `aspen-rpc-core`, possibly new adapter crates if the split is cleaner than feature-gating.
- **Docs modified**: `docs/crate-extraction.md`, `docs/crate-extraction/policy.ncl`, new `docs/crate-extraction/transport-rpc.md`.
- **Consumers verified**: `aspen-raft-network`, `aspen-raft`, `aspen-cluster`, `aspen-client`, `aspen-rpc-handlers`, root `aspen` node runtime, and downstream fixtures.
- **APIs**: Generic transport/RPC APIs become canonical reusable surfaces. Existing Aspen context APIs remain available through explicit runtime features or compatibility paths.
- **Dependencies**: Iroh/irpc are allowed for transport adapter surfaces. Trust, sharding, auth runtime, Raft compatibility, concrete domain services, and cluster bootstrap must be explicit opt-ins, not default reusable requirements.
