## Phase 1: Feature-gate relaxation (std-mode fixtures)

- [x] Deferred to openspec change: no-std-content-discovery-trait ✅ 0m (deferred — `ContentDiscovery` trait methods use `iroh_blobs::Hash`/`BlobFormat` which require the optional `iroh-blobs` dep enabled by `global-discovery` feature; needs either non-optional dep or abstract trait)
- [x] Deferred to openspec change: no-std-directory-layer ✅ 0m (deferred — `DirectoryLayer` struct is defined in optional `aspen-layer` crate enabled by `layer` feature; needs either non-optional dep or alloc-safe abstract layer)

## Phase 2: Alloc-safe type extraction (no-std fixtures)

- [x] Deferred to openspec change: no-std-app-registry ✅ 0m (deferred — `AppRegistry` uses `HashMap`, `Arc`, `tokio`; needs alloc-safe data model split from runtime layer)
- [x] Deferred to openspec change: no-std-network-transport ✅ 0m (deferred — `NetworkTransport` trait uses `iroh` types in associated type bounds; needs abstract trait without iroh deps)
- [x] Deferred to openspec change: no-std-simulation-artifact ✅ 0m (deferred — `SimulationArtifact` uses `PathBuf`, `chrono`, `anyhow`, `std::fs`; needs alloc-safe data model split from persistence)
- [x] Deferred to openspec change: no-std-storage-table ✅ 0m (deferred — `SM_KV_TABLE` is a `redb::TableDefinition`; needs table name constant or alloc-safe table metadata split from redb dep)
