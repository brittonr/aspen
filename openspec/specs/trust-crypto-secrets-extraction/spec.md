# trust-crypto-secrets-extraction Specification

## Purpose

Defines reusable extraction boundaries for trust, crypto, and secrets helpers so portable cryptographic/state-machine surfaces stay separate from Aspen runtime adapters.

## Requirements

### Requirement: Transport-free crypto default [r[trust-crypto-secrets-extraction.transport-free-crypto-default]]

The trust/crypto/secrets extraction SHALL keep reusable default crypto helpers free of concrete Aspen runtime, transport endpoint, Redb, handler, and node bootstrap dependencies.

#### Scenario: Aspen crypto default excludes runtime identity lifecycle [r[trust-crypto-secrets-extraction.transport-free-crypto-default.aspen-crypto-default]]

- GIVEN `aspen-crypto` is built with no default features
- WHEN its normal dependency graph is inspected
- THEN it SHALL expose cookie/hash helper behavior without normal dependencies on concrete `iroh`, `iroh-base`, Tokio, or randomness crates
- AND node identity lifecycle helpers SHALL require an explicit feature.

#### Scenario: Identity compatibility is explicit [r[trust-crypto-secrets-extraction.transport-free-crypto-default.identity-feature]]

- GIVEN a runtime consumer needs node identity key lifecycle helpers
- WHEN it enables `aspen-crypto/identity`
- THEN it MAY use `iroh-base` key types, randomness, and Tokio file lifecycle helpers without pulling concrete `iroh` endpoint/runtime dependencies through the default crypto surface.

### Requirement: Explicit secrets auth runtime boundary [r[trust-crypto-secrets-extraction.secrets-auth-runtime-boundary]]

The trust/crypto/secrets extraction SHALL keep reusable `aspen-secrets` token parsing independent from the runtime `aspen-auth` shell unless an explicit runtime-auth feature is enabled.

#### Scenario: Secrets default uses portable token data [r[trust-crypto-secrets-extraction.secrets-auth-runtime-boundary.default-token-data]]

- GIVEN `aspen-secrets` is built with no default features
- WHEN a prebuilt capability token is parsed from secrets config
- THEN it SHALL use the portable `aspen-auth-core::CapabilityToken` data type without requiring runtime `aspen-auth` verifier or builder services.

#### Scenario: Runtime auth helpers are opt-in [r[trust-crypto-secrets-extraction.secrets-auth-runtime-boundary.runtime-helper-feature]]

- GIVEN a node bootstrap/runtime consumer needs `TokenVerifier` or `TokenBuilder` helpers
- WHEN it enables `aspen-secrets/auth-runtime`
- THEN it MAY construct those helpers through the runtime `aspen-auth` shell without making that shell part of the reusable default secrets boundary.

### Requirement: Secrets storage uses lightweight KV traits [r[trust-crypto-secrets-extraction.secrets-kv-traits-boundary]]

The trust/crypto/secrets extraction SHALL keep reusable `aspen-secrets` Aspen-backed storage adapter signatures on the lightweight KV trait/type crates instead of the broad `aspen-core` compatibility surface.

#### Scenario: Secrets storage signatures avoid aspen-core [r[trust-crypto-secrets-extraction.secrets-kv-traits-boundary.no-aspen-core-signatures]]

- GIVEN `aspen-secrets` exposes `AspenSecretsBackend`, `MountRegistry`, or SOPS runtime KV manager APIs
- WHEN those APIs accept or store a KV backend
- THEN they SHALL use `aspen_traits::KeyValueStore` and `aspen-kv-types` operation types rather than naming `aspen_core::KeyValueStore` or `aspen_core` request/result compatibility re-exports.

#### Scenario: Runtime compatibility remains intact [r[trust-crypto-secrets-extraction.secrets-kv-traits-boundary.runtime-compatibility]]

- GIVEN existing runtime stores implement the Aspen KV trait through compatibility re-exports
- WHEN `aspen-secrets-handler` and node secrets bootstrap are checked
- THEN they SHALL continue to compile without adding `aspen-core` back to the `aspen-secrets` no-default dependency boundary.

### Requirement: Secrets handler runtime adapter is explicit [r[trust-crypto-secrets-extraction.secrets-handler-runtime-adapter-boundary]]

The trust/crypto/secrets extraction SHALL keep `aspen-secrets-handler` default builds free of broad Aspen runtime-context and Redb/Raft storage dependencies; only the node/runtime factory adapter MAY enable the full runtime context graph.

#### Scenario: Portable handler default avoids runtime storage [r[trust-crypto-secrets-extraction.secrets-handler-runtime-adapter-boundary.default-portable]]

- GIVEN `aspen-secrets-handler` is built with no default features
- WHEN its normal dependency graph is inspected
- THEN it SHALL compile without normal dependencies on `aspen-core`, `aspen-core-shell`, `aspen-raft`, `aspen-redb-storage`, or `redb`
- AND its executor SHALL use `aspen_traits::KeyValueStore` plus `aspen-kv-types` request types instead of `aspen_core` compatibility re-exports.

#### Scenario: Runtime factory adapter remains opt-in compatible [r[trust-crypto-secrets-extraction.secrets-handler-runtime-adapter-boundary.runtime-adapter]]

- GIVEN Aspen node/runtime handler registration needs `ClientProtocolContext` and `HandlerFactory`
- WHEN `aspen-secrets-handler/runtime-adapter` or the aggregate `aspen-rpc-handlers/secrets` bundle is enabled
- THEN the secrets handler factory SHALL compile with the runtime context and preserve node secrets compatibility.

### Requirement: Secrets core owns portable type contracts [r[trust-crypto-secrets-extraction.secrets-core-type-boundary]]

The trust/crypto/secrets extraction SHALL expose portable secrets constants, KV DTO/state types, Transit DTO/key state types, and PKI DTO/state types from a dependency-light core crate instead of requiring consumers to depend on the runtime secrets service shell.

#### Scenario: Core type crate avoids runtime dependencies [r[trust-crypto-secrets-extraction.secrets-core-type-boundary.core-dependency-rail]]

- GIVEN `aspen-secrets-core` is built with no default features
- WHEN its normal dependency graph is inspected
- THEN it SHALL depend only on portable serialization support and SHALL NOT pull Tokio, age, rcgen, x509, Aspen client/transport/trust/auth runtime crates, Iroh, AEAD runtime crates, Redb, or handler/runtime shells.

#### Scenario: Runtime compatibility re-exports remain [r[trust-crypto-secrets-extraction.secrets-core-type-boundary.compatibility-reexports]]

- GIVEN existing runtime code imports secrets constants or KV, Transit, or PKI type modules through `aspen-secrets`
- WHEN `aspen-secrets`, `aspen-secrets-handler`, and the node secrets bundle are checked
- THEN those compatibility paths SHALL compile while the canonical type/state definitions live in `aspen-secrets-core`.

### Requirement: Secrets handler uses mount provider boundary [r[trust-crypto-secrets-extraction.secrets-mount-provider-boundary]]

The trust/crypto/secrets extraction SHALL keep the native secrets handler service boundary coupled to a mounted-store provider contract instead of the concrete runtime mount registry implementation.

#### Scenario: Handler service depends on provider trait [r[trust-crypto-secrets-extraction.secrets-mount-provider-boundary.handler-provider-trait]]

- GIVEN `aspen-secrets-handler` constructs a `SecretsService`
- WHEN it resolves PKI, KV, or Transit stores by mount
- THEN it SHALL call a provider trait exported by `aspen-secrets` rather than storing or invoking the concrete `MountRegistry` implementation directly.

#### Scenario: Mount registry remains runtime-compatible [r[trust-crypto-secrets-extraction.secrets-mount-provider-boundary.mount-registry-compatibility]]

- GIVEN existing node/runtime code constructs a concrete `MountRegistry`
- WHEN that registry is passed to `SecretsService::new`
- THEN it SHALL compile through the provider trait while preserving existing mount validation, caching, and store creation behavior.

### Requirement: Owner/public API readiness review [r[trust-crypto-secrets-extraction.owner-public-api-review]]

The trust/crypto/secrets extraction SHALL complete an owner/public API review before promoting the aggregate family beyond `workspace-internal`.

#### Scenario: Review separates reusable and runtime surfaces [r[trust-crypto-secrets-extraction.owner-public-api-review.surface-classification]]

- GIVEN the family includes crypto helpers, trust state, secrets type contracts, secrets implementation services, and handler/runtime adapters
- WHEN readiness is reviewed
- THEN the review SHALL record which crates/features are canonical reusable public API surfaces
- AND it SHALL record which crates/features remain runtime adapters or compatibility consumers.

#### Scenario: Promotion requires real checker coverage [r[trust-crypto-secrets-extraction.owner-public-api-review.checker-coverage]]

- GIVEN the crate-extraction checker is used as readiness evidence
- WHEN the family is considered for `extraction-ready-in-workspace`
- THEN the checker SHALL validate real crate/package candidates for the reusable surfaces instead of deferring direct dependency checks through an aggregate pseudo-candidate
- AND the reusable package candidates SHALL match the owner/public API review's canonical reusable surfaces until additional surfaces are explicitly promoted.

#### Scenario: Stability contracts are explicit [r[trust-crypto-secrets-extraction.owner-public-api-review.stability-contracts]]

- GIVEN trust and secrets types expose serialized state, custom binary formats, or service DTOs
- WHEN the owner/public API review completes
- THEN it SHALL identify which formats are compatibility contracts and which remain internal implementation details before promotion.

### Requirement: Aspen Trust Async Default Policy

`aspen-trust` SHALL keep async runtime service APIs behind an explicit feature so its default/no-default reusable helper/state/wire surface has a documented dependency policy.

#### Scenario: Default trust surface excludes async runtime service dependencies

- GIVEN a consumer depends on `aspen-trust` with default or no default features
- WHEN dependency evidence is captured for trust/crypto/secrets readiness
- THEN the normal dependency graph SHALL NOT include `tokio` or `async-trait` through `aspen-trust`
- AND `key_manager` and `reencrypt` SHALL require the explicit `async` feature.

#### Scenario: Runtime trust consumers opt into async service APIs

- GIVEN a runtime consumer needs trust key-manager or reencryption service APIs
- WHEN it enables its trust runtime feature
- THEN it SHALL enable `aspen-trust/async` explicitly and continue to compile.

#### Scenario: Trust checker validates real trust package coverage

- GIVEN the trust/crypto/secrets family readiness checker runs
- WHEN selected pure trust helper/state surfaces are considered reusable candidates
- THEN it SHALL validate `aspen-trust` as a real package candidate with the documented default dependency policy.

### Requirement: Aspen Trust Async Default Policy [r[trust-crypto-secrets-extraction.aspen-trust-async-default-policy]]

`aspen-trust` SHALL make its default/no-default reusable surface explicit by keeping async runtime service APIs behind a named feature while preserving runtime compatibility for consumers that opt in.

#### Scenario: Default trust surface excludes async runtime service dependencies [r[trust-crypto-secrets-extraction.aspen-trust-async-default-policy.default-excludes-async]]

- GIVEN a consumer depends on `aspen-trust` with default or no default features
- WHEN dependency evidence is captured for the trust/crypto/secrets readiness checker
- THEN the normal dependency graph SHALL NOT include `tokio` or `async-trait` through `aspen-trust`
- AND `key_manager` and `reencrypt` SHALL require the explicit `async` feature.

#### Scenario: Runtime trust consumers opt into async service APIs [r[trust-crypto-secrets-extraction.aspen-trust-async-default-policy.runtime-opt-in]]

- GIVEN a runtime consumer needs the trust key manager or reencryption service APIs
- WHEN it enables its trust runtime feature
- THEN it SHALL enable `aspen-trust/async` explicitly and continue to compile.

#### Scenario: Trust checker validates real trust package coverage [r[trust-crypto-secrets-extraction.aspen-trust-async-default-policy.checker-coverage]]

- GIVEN the trust/crypto/secrets family readiness checker runs
- WHEN selected pure trust helper/state surfaces are considered reusable candidates
- THEN it SHALL validate `aspen-trust` as a real package candidate with the documented default dependency policy.

### Requirement: Trust and secrets serialization contract evidence [r[trust-crypto-secrets-extraction.serialization-contract-evidence]]

The trust/crypto/secrets extraction SHALL capture deterministic serialization contract evidence before promoting reusable trust or secrets surfaces beyond `workspace-internal`.

#### Scenario: Stable trust formats have goldens and roundtrips [r[trust-crypto-secrets-extraction.serialization-contract-evidence.trust-goldens]]

- GIVEN `aspen-trust` exposes share, encrypted envelope, encrypted chain, threshold, or trust protocol formats as reusable compatibility contracts
- WHEN serialization evidence is captured
- THEN deterministic golden bytes or JSON fixtures SHALL exist for each stable format
- AND roundtrip tests SHALL prove decoding preserves the contract value
- AND malformed, truncated, or wrong-version bytes SHALL be rejected where the format has an explicit parser.

#### Scenario: Stable secrets-core persisted state has goldens and roundtrips [r[trust-crypto-secrets-extraction.serialization-contract-evidence.secrets-core-goldens]]

- GIVEN `aspen-secrets-core` owns KV, Transit, and PKI DTO/state contracts used by storage or compatibility re-exports
- WHEN serialization evidence is captured
- THEN persisted state and config formats SHALL have deterministic golden/roundtrip coverage
- AND request/response DTOs SHALL be identified as stable compatibility contracts or internal implementation details.

#### Scenario: Serialization contract scope is recorded before promotion [r[trust-crypto-secrets-extraction.serialization-contract-evidence.scope-recorded]]

- GIVEN trust/crypto/secrets readiness is considered for `extraction-ready-in-workspace`
- WHEN verification evidence is reviewed
- THEN the change SHALL record which trust/secrets formats are stable compatibility contracts
- AND it SHALL record which serialized forms remain internal and are excluded from compatibility guarantees.
