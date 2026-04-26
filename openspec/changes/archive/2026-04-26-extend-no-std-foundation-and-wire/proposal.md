## Why

Six of eight `aspen-core` UI fixture tests fail, blocking CI validation
of the public API surface under no-std and feature-subset configurations.
The passing fixtures (`constants-no-std`, `kv-types-no-std`) prove the
pattern works; extending it to the remaining six types completes the
no-std foundation seam.

## What Changes

Each failing fixture asserts that a specific type is importable from
`aspen_core` under a restricted configuration:

| Fixture | Configuration | Missing type | Current location |
|---------|--------------|--------------|------------------|
| `app-registry-no-std` | `default-features = false` | `AppRegistry` | `aspen-core-shell/src/app_registry.rs` |
| `network-transport-no-std` | `default-features = false` | `NetworkTransport` | `aspen-core-shell/src/transport.rs` |
| `simulation-artifact-no-std` | `default-features = false` | `SimulationArtifact` | `aspen-core-shell/src/simulation.rs` |
| `storage-table-no-std` | `default-features = false` | `SM_KV_TABLE` | `aspen-storage-types` (redb) |
| `content-discovery-std-without-global-discovery` | `aspen-core-shell` without `global-discovery` | `ContentDiscovery` | `aspen-core-shell/src/context/discovery.rs` |
| `directory-layer-std-without-layer` | `aspen-core-shell` without `layer` | `DirectoryLayer` | `aspen-core-shell/src/layer/directory/layer.rs` |

The first four require moving type definitions (or alloc-safe subsets)
from `aspen-core-shell` into the alloc-only `aspen-core`. The last two
require relaxing feature gates in `aspen-core-shell`.

## Capabilities

### Modified Capabilities
- `no-std-api-surface`: Extend the alloc-only `aspen-core` re-exports
  to include `AppRegistry`, `NetworkTransport`, `SimulationArtifact`, and
  `SM_KV_TABLE` (or alloc-safe equivalents).
- `feature-gated-exports`: Make `ContentDiscovery` and `DirectoryLayer`
  available without their current feature gates.

## Impact

- **Files**: `crates/aspen-core/`, `crates/aspen-core-shell/src/lib.rs`,
  `crates/aspen-core-shell/src/context/mod.rs`,
  `crates/aspen-storage-types/`
- **APIs**: New re-exports from `aspen_core` in no-std mode
- **Dependencies**: May need to split types into alloc-safe data models
  vs std-only persistence/runtime layers
- **Testing**: All 6 UI fixture tests must pass
