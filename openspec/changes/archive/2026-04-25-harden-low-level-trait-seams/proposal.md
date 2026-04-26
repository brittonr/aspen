## Why

Aspen's extraction-ready crates now have clear package boundaries, but several low-level seams still expose overly broad traits or depend on root/runtime crates for narrow operations. Productizing the next crates requires contracts where reusable libraries depend on leaf traits and domain types, while runtime crates own concrete adapters.

## What Changes

- Define low-level trait seam requirements for KV capabilities, cache persistence, blob/KV offload, storage ports, branch/DAG persistence, time providers, and logical clocks.
- Require reusable low-level crates to depend on narrow contracts (`aspen-traits`, `aspen-kv-types`, domain traits) rather than root `aspen-core`, handler registries, node runtime, or concrete adapters unless explicitly feature-gated.
- Establish verification requirements for positive downstream fixtures, negative boundary checks, adapter compatibility rails, and trait object/generic compile proofs.
- Document a staged implementation path starting with `aspen-traits` capability traits and `aspen-cache` persistence ports, then continuing through blob, storage, branch/DAG, and clock seams.

## Capabilities

### New Capabilities
- `low-level-trait-seams`: Defines reusable trait/interface seams for low-level crates and adapter placement rules.

### Modified Capabilities
- `architecture-modularity`: Extends extraction-readiness rules so low-level candidates prove narrow trait contracts, adapter ownership, and absence of broad root/runtime dependencies.

## Impact

- **Crates**: `aspen-traits`, `aspen-kv-types`, `aspen-cache`, `aspen-blob`, `aspen-storage-types`, `aspen-redb-storage`, `aspen-kv-branch`, `aspen-commit-dag`, `aspen-time`, `aspen-hlc`, plus compatibility consumers.
- **APIs**: Adds smaller capability traits and adapter traits while preserving compatibility through composite traits and migration re-exports where needed.
- **Dependencies**: Moves root/runtime dependencies behind adapter crates or named features; reusable defaults depend on leaf type and trait crates.
- **Testing**: Requires compile, cargo-tree, downstream fixture, negative boundary, and compatibility evidence before any affected crate is marked extraction-ready.
