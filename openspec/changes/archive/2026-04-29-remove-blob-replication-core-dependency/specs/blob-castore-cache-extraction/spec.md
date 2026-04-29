## ADDED Requirements

### Requirement: Blob replication KV metadata uses leaf KV contracts
`aspen-blob` replication metadata storage SHALL use leaf KV trait and type crates for replica metadata persistence instead of depending on root `aspen-core`.

ID: blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts

#### Scenario: Replication adapter compiles without root core
ID: blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replication-adapter-compiles-without-root-core

- **GIVEN** `aspen-blob` is built with the `replication` feature
- **WHEN** Cargo resolves normal dependencies for that feature set
- **THEN** the graph SHALL NOT include root package `aspen-core`
- **AND** the feature MAY include `aspen-client-api` because replication RPC wire messages are adapter-owned
- **AND** the feature SHALL use `aspen-traits` and `aspen-kv-types` for KV persistence contracts

#### Scenario: Replica metadata behavior is preserved
ID: blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replica-metadata-behavior-preserved

- **GIVEN** `KvReplicaMetadataStore` persists replica sets under the existing replica metadata key prefix
- **WHEN** get, save, delete, and scan-by-status operations are exercised
- **THEN** they SHALL keep the same key format, JSON payload format, and status filtering behavior
- **AND** errors from the KV layer SHALL continue to be mapped at the adapter boundary with actionable context

#### Scenario: Policy rejects stale core exception
ID: blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.policy-rejects-stale-core-exception

- **GIVEN** the blob/castore/cache extraction policy is checked
- **WHEN** `aspen-blob` or the blob/castore transitive path reintroduces `aspen-core` for replication metadata storage
- **THEN** the extraction-readiness checker SHALL fail unless a future OpenSpec change documents a new owner, feature gate, reason, and compatibility evidence

## MODIFIED Requirements

### Requirement: Blob default graph exposes reusable iroh-backed storage without Aspen app shells
`aspen-blob` MUST expose reusable blob storage APIs over iroh/iroh-blobs without requiring root Aspen app crates, handler registries, node bootstrap, client RPC schemas, trust/secrets/SQL services, UI/web/binary shells, Raft compatibility crates, or root `aspen-core` where leaf KV trait/type crates are sufficient.

ID: blob-castore-cache-extraction.blob-default-avoids-app-shells

#### Scenario: Replication RPC is adapter-only
ID: blob-castore-cache-extraction.blob-default-avoids-app-shells.replication-rpc-is-adapter-only

- **WHEN** the default reusable blob graph is checked
- **THEN** it SHALL NOT include `aspen-client-api`, `aspen-rpc-core`, `aspen-rpc-handlers`, root `aspen`, root `aspen-core`, or node bootstrap crates
- **AND** any Aspen replication/client-RPC integration SHALL be behind an explicit feature or adapter crate with its own compatibility evidence
- **AND** the replication adapter SHALL use `aspen-traits` and `aspen-kv-types` for KV metadata persistence instead of importing root `aspen-core`
