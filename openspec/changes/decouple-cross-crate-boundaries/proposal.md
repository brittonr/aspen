## Why

Aspen has strong distributed primitives, but several top-level boundaries are too coupled:

- `crates/aspen-client-api/src/messages/mod.rs` centralizes hundreds of RPC variants in one wire file.
- `crates/aspen-rpc-core/src/context.rs` gives every handler a god-context with most subsystems.
- `src/bin/aspen_node/setup/client.rs` wires many domains in one imperative shell.
- `crates/aspen-cluster/src/lib.rs` and `src/node/mod.rs` gate bootstrap behind large feature conjunctions.
- `src/node/mod.rs` still needs an `unsafe` transmute between Raft and transport type configs.
- feature bundles such as `ci`, `hooks`, and bootstrap composition pull in unrelated subsystems.

Those seams raise rebuild blast radius, force cross-domain edits for local changes, and make Tiger Style boundaries weaker than they should be.

## What Changes

- Add a new architecture modularity spec that defines narrow, explicit composition seams.
- Plan an incremental refactor that cuts coupling at five hotspots:
  - client RPC protocol ownership
  - handler dependency injection
  - node bootstrap / service composition
  - shared Raft / transport type ownership
  - feature-bundle dependency shape
- Require verification that each refactor reduces central edit hotspots and preserves wire / bootstrap behavior.

## Capabilities

### New Capabilities

- `architecture-modularity`: Aspen has explicit requirements for modular composition, narrow RPC seams, and bounded feature coupling.
- `domain-owned-rpc-contracts`: app RPC definitions and metadata move toward domain-owned contracts instead of one mega edit point.
- `capability-scoped-handler-context`: handlers receive only declared capabilities instead of a shared global dependency bag.
- `phase-composed-bootstrap`: node startup composes bounded phases that can be enabled independently.
- `leaf-shared-protocol-types`: transport and consensus share one leaf type source without `unsafe` bridging.
- `bounded-feature-bundles`: feature flags and bundles expose direct prerequisites instead of silently pulling unrelated apps.

## Impact

- **Files**: `openspec/changes/decouple-cross-crate-boundaries/`, `crates/aspen-client-api/`, `crates/aspen-rpc-core/`, `crates/aspen-rpc-handlers/`, `crates/aspen-cluster/`, `src/bin/aspen_node/setup/client.rs`, `src/node/mod.rs`, workspace `Cargo.toml` files
- **APIs**: internal architecture and crate boundaries change; external user workflows should remain stable during incremental migration
- **Dependencies**: likely new leaf crates or narrower existing crate roles; no new runtime service required
- **Testing**: targeted cargo checks per feature slice, wire-format regression tests, bootstrap compile-slice tests, handler-registration tests, and crate-graph evidence
