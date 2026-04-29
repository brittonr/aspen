## ADDED Requirements

### Requirement: Blob replication adapter prefers leaf KV contracts over root core packages
The `aspen-blob/replication` adapter SHALL depend on root core packages only for behavior that is not available through leaf type, trait, or domain-port crates.

ID: architecture.modularity.blob-replication-prefers-leaf-kv-contracts

#### Scenario: Blob replication KV metadata uses leaf KV contracts
ID: architecture.modularity.blob-replication-prefers-leaf-kv-contracts.kv-metadata-uses-leaf-kv-contracts

- **GIVEN** the blob replication metadata adapter needs KV read, write, delete, and scan behavior
- **WHEN** leaf KV crates such as `aspen-traits` and `aspen-kv-types` expose the needed contract
- **THEN** the adapter SHALL use those leaf crates instead of root `aspen-core`
- **AND** cargo tree evidence SHALL prove `aspen-blob --features replication` does not pull root `aspen-core` for replica metadata persistence

#### Scenario: Runtime wire schemas remain separately gated
ID: architecture.modularity.blob-replication-prefers-leaf-kv-contracts.runtime-wire-schemas-remain-separately-gated

- **GIVEN** blob replication also needs client RPC request/response wire schemas
- **WHEN** those schemas are required for runtime compatibility
- **THEN** the adapter MAY depend on `aspen-client-api`
- **AND** that permission SHALL NOT justify importing root `aspen-core` for unrelated KV persistence contracts

#### Scenario: Boundary policy catches stale blob core dependency
ID: architecture.modularity.blob-replication-prefers-leaf-kv-contracts.boundary-policy-catches-stale-blob-core-dependency

- **GIVEN** crate-extraction policy records the blob/castore/cache family
- **WHEN** a broad `aspen-blob -> aspen-core` dependency or exception is reintroduced for replica metadata persistence
- **THEN** deterministic policy or cargo tree checks SHALL fail with a clear dependency path
