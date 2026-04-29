# blob-castore-cache-extraction Specification

## Purpose
TBD - created by archiving change extract-blob-castore-cache. Update Purpose after archive.
## Requirements
### Requirement: Blob default graph exposes reusable iroh-backed storage without Aspen app shells
`aspen-blob` MUST expose reusable blob storage APIs over iroh/iroh-blobs without requiring root Aspen app crates, handler registries, node bootstrap, client RPC schemas, trust/secrets/SQL services, UI/web/binary shells, Raft compatibility crates, or root `aspen-core` where leaf KV trait/type crates are sufficient.

ID: blob-castore-cache-extraction.blob-default-avoids-app-shells

#### Scenario: Replication RPC is adapter-only
ID: blob-castore-cache-extraction.blob-default-avoids-app-shells.replication-rpc-is-adapter-only

- **WHEN** the default reusable blob graph is checked
- **THEN** it SHALL NOT include `aspen-client-api`, `aspen-rpc-core`, `aspen-rpc-handlers`, root `aspen`, root `aspen-core`, or node bootstrap crates
- **AND** any Aspen replication/client-RPC integration SHALL be behind an explicit feature or adapter crate with its own compatibility evidence
- **AND** the replication adapter SHALL use `aspen-traits` and `aspen-kv-types` for KV metadata persistence instead of importing root `aspen-core`

### Requirement: Castore and cache reusable APIs avoid Aspen shells
`aspen-castore` and `aspen-cache` MUST separate reusable snix/Nix cache APIs from Aspen-specific node, RPC, cluster, testing, and core-shell integration.

ID: blob-castore-cache-extraction.castore-cache-avoid-app-shells

#### Scenario: Castore circuit breaker does not force core shell
ID: blob-castore-cache-extraction.castore-cache-avoid-app-shells.castore-circuit-breaker-is-reusable-or-gated

- **WHEN** reusable `aspen-castore` feature sets are checked
- **THEN** circuit-breaker behavior SHALL come from a reusable helper or be disabled/gated
- **AND** the reusable graph SHALL NOT require `aspen-core-shell` or root Aspen app crates by default

#### Scenario: Cache metadata and signing are reusable
ID: blob-castore-cache-extraction.castore-cache-avoid-app-shells.cache-metadata-signing-reusable

- **WHEN** `aspen-cache` reusable feature sets are checked
- **THEN** Nix narinfo parsing, signing, and cache metadata helpers SHALL compile without Aspen cluster/testing/runtime crates
- **AND** cluster-backed publication or RPC integration SHALL be feature-gated or adapter-owned

### Requirement: Downstream fixtures prove blob/castore/cache standalone use
Downstream-style fixtures MUST prove the family can be used outside the root Aspen app for direct blob storage and cache/castore API usage.

ID: blob-castore-cache-extraction.downstream-fixtures

#### Scenario: Blob fixture uses canonical blob APIs directly
ID: blob-castore-cache-extraction.downstream-fixtures.blob-fixture-uses-canonical-api

- **GIVEN** a downstream fixture manifest
- **WHEN** it compiles and records cargo metadata
- **THEN** it SHALL import `aspen-blob` directly
- **AND** it SHALL exercise canonical blob store construction or type-level APIs without root Aspen, handlers, node bootstrap, or client RPC crates

#### Scenario: Cache/castore fixture uses reusable domain APIs
ID: blob-castore-cache-extraction.downstream-fixtures.cache-castore-fixture-uses-domain-apis

- **GIVEN** a downstream fixture manifest
- **WHEN** it compiles and records cargo metadata
- **THEN** it SHALL import `aspen-castore` and/or `aspen-cache` directly
- **AND** it SHALL exercise reusable snix/Nix cache metadata/signing or adapter types without Aspen app shells

### Requirement: Aspen compatibility adapters remain verified
Existing Aspen blob/cache/castore integration paths MUST continue to compile and pass focused tests through explicit features, adapter crates, or compatibility re-exports.

ID: blob-castore-cache-extraction.compatibility-adapters-verified

#### Scenario: Runtime consumers compile through explicit integration paths
ID: blob-castore-cache-extraction.compatibility-adapters-verified.runtime-consumers-compile

- **WHEN** representative Aspen runtime consumers are checked with their documented blob/cache/castore features
- **THEN** they SHALL compile without import-path breakage
- **AND** evidence SHALL identify whether each consumer uses canonical reusable APIs, adapter features, or temporary compatibility paths

### Requirement: Extraction inventory tracks blob/castore/cache family
The crate extraction inventory and policy MUST include the blob/castore/cache family with readiness state, class, owner, allowed backend dependencies, forbidden app dependencies, representative consumers, and next action.

ID: blob-castore-cache-extraction.inventory-and-policy

#### Scenario: Readiness checker verifies backend exceptions and app-shell bans
ID: blob-castore-cache-extraction.inventory-and-policy.checker-verifies-backend-exceptions

- **WHEN** `scripts/check-crate-extraction-readiness.rs --candidate-family blob-castore-cache` runs
- **THEN** it SHALL allow documented backend-purpose dependencies
- **AND** it SHALL fail on unowned app-shell dependencies, missing owners, invalid readiness labels, or missing downstream fixture evidence

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
