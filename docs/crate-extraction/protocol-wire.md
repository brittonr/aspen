# Extraction Manifest: Protocol and Wire Crates

## Candidate

- **Family**: Protocol/wire
- **Canonical class**: `protocol/wire`
- **Canonical crate/path**: `crates/aspen-client-api`, `crates/aspen-forge-protocol`, `crates/aspen-jobs-protocol`, `crates/aspen-coordination-protocol`
- **Intended audience**: Rust clients, tools, fixtures, gateways, and protocol adapters that need Aspen request/response schemas and domain wire structs without handler registries, node bootstrap, concrete transports, UI/web binaries, or runtime auth shells.
- **Public API owner**: Aspen protocol maintainers
- **Readiness state**: `extraction-ready-in-workspace`
- **Dependency policy class**: reusable protocol/wire family

## Package and release metadata

- **Package description**: Aspen client RPC schemas plus Forge, jobs, and coordination protocol structs.
- **Documentation entrypoint**: Crate-level Rustdoc plus wire-contract tests and downstream fixture evidence.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: Monorepo path until publication policy is decided.
- **Semver/wire-contract policy**: Postcard discriminants and encoded public wire shapes are append-only contracts; new enum variants must be appended and wire-contract evidence updated.
- **Publish readiness**: Blocked; do not mark publishable during this change.

## Feature contract

| Crate | Feature set | Status | Purpose |
| --- | --- | --- | --- |
| `aspen-client-api` | default | reusable wire default | Client request/response enums, authenticated request wrapper, app/capability metadata, and domain protocol references. Default keeps all enum-layout features enabled. |
| `aspen-client-api` | no-default-features | minimal wire check | Compiles the minimal schema surface for boundary evidence; not used to change runtime wire-contract expectations. |
| `aspen-client-api` | `auth` | portable auth types | Uses `aspen-auth-core` via dependency key `aspen-auth`; runtime verifier/builder shell stays out of the graph. |
| `aspen-forge-protocol` | default / no-default-features | reusable domain protocol | Forge response and metadata structs over serde alloc. |
| `aspen-jobs-protocol` | default / no-default-features | reusable domain protocol | Jobs response, worker, queue, and execution metadata structs over serde alloc. |
| `aspen-coordination-protocol` | default / no-default-features | reusable domain protocol | Coordination primitive wire responses over serde alloc. |

## Dependencies

### Internal Aspen dependencies

| Crate | Dependency | Decision | Reason |
| --- | --- | --- | --- |
| `aspen-client-api` | `aspen-auth-core` (dependency key `aspen-auth`) | keep behind `auth`, default-on for layout stability | Portable capability/token wire types without runtime HMAC/verifier/revocation shell. |
| `aspen-client-api` | `iroh-base` through `aspen-auth-core` | documented key-type exception | `aspen-auth-core` stores iroh public keys; no iroh endpoint, QUIC, relay, or transport runtime is pulled in. |
| `aspen-client-api` | `aspen-coordination-protocol`, `aspen-forge-protocol`, `aspen-jobs-protocol` | keep | Domain protocol structs are part of the public client wire surface. |
| `aspen-forge-protocol` | none beyond serde | keep | Domain-only protocol crate. |
| `aspen-jobs-protocol` | none beyond serde | keep | Domain-only protocol crate. |
| `aspen-coordination-protocol` | none beyond serde | keep | Domain-only protocol crate. |

### External dependencies

| Dependency | Decision | Reason |
| --- | --- | --- |
| `serde` | keep | Public schema serialization. |
| `thiserror` | keep for `aspen-client-api` | Client protocol error type. |
| `postcard`, `serde_json`, `insta`, `syn`, `tokio` | dev/test only | Compatibility, snapshot, source audit, and async test rails. |

### Binary/runtime dependencies

No root app, CLI/TUI, web, dogfood, handler, concrete transport, trust, secrets runtime service, SQL engine, node bootstrap, or runtime `aspen-auth` crate is allowed in default reusable protocol graphs. Auth, hook, capability, and ticket wire references must stay in leaf/protocol crates such as `aspen-auth-core` and `aspen-hooks-ticket`.

## Compatibility and aliases

- **Old paths**: Existing public import paths stay direct crate imports (`aspen_client_api::*`, `aspen_forge_protocol::*`, `aspen_jobs_protocol::*`, `aspen_coordination_protocol::*`).
- **Compatibility re-exports**: none. Consumers import canonical protocol crates directly.
- **Postcard policy**: `ClientRpcRequest` and `ClientRpcResponse` discriminants are append-only. Existing tests in `aspen-client-api` pin critical request/response discriminants, default-on enum-layout features, and first/last/critical variants.
- **Domain struct policy**: Forge/jobs/coordination protocol structs must retain serde field names or update wire-contract evidence when intentionally changed.

## Representative consumers

- `aspen-client`
- `aspen-cli`
- `aspen-rpc-core`
- `aspen-rpc-handlers`
- `aspen-forge-handler`
- `aspen-job-handler`
- `aspen-coordination`
- downstream serialization fixture under `openspec/changes/extract-protocol-wire-crates/fixtures/downstream-protocol-wire`

## Representative consumers and re-exporters

- **Representative consumers**: `aspen-client`, `aspen-cli`, `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-forge-handler`, `aspen-job-handler`, `aspen-coordination`, and the downstream serialization fixture.
- **Re-exporters**: wire-contract re-exports: none. Protocol consumers use canonical crates directly.

## Dependency exceptions

| candidate | feature_set | dependency_path | owner | reason |
| --- | --- | --- | --- | --- |
| `aspen-client-api` | default | `aspen-client-api -> aspen-auth-core` | Aspen protocol maintainers | Portable auth/capability wire types; not runtime `aspen-auth`. |
| `aspen-client-api` | default | `aspen-client-api -> aspen-auth-core -> iroh-base` | Aspen protocol maintainers | Key-only public-key type dependency from auth-core; not concrete transport runtime. |
| `aspen-client-api` | default | `aspen-client-api -> aspen-coordination-protocol` | Aspen protocol maintainers | Coordination wire responses are part of client RPC schema. |
| `aspen-client-api` | default | `aspen-client-api -> aspen-forge-protocol` | Aspen protocol maintainers | Forge wire responses are part of client RPC schema. |
| `aspen-client-api` | default | `aspen-client-api -> aspen-jobs-protocol` | Aspen protocol maintainers | Jobs wire responses are part of client RPC schema. |

## Verification rails

- `cargo check -p aspen-client-api`
- `cargo check -p aspen-client-api --no-default-features`
- `cargo check -p aspen-forge-protocol` and `--no-default-features`
- `cargo check -p aspen-jobs-protocol` and `--no-default-features`
- `cargo check -p aspen-coordination-protocol` and `--no-default-features`
- supported wasm/minimal target checks when the local toolchain has the target available
- `cargo tree` for all protocol crates plus negative boundary grep for root app, handlers, concrete transport, runtime auth, trust/secrets, SQL, and binaries
- `cargo test -p aspen-client-api` and protocol crate serialization tests for positive encode/decode and negative discriminant/baseline-change rails
- positive downstream serialization fixture using canonical protocol crates directly
- dependency-boundary checker with `--candidate-family protocol-wire`, including negative mutations for forbidden dependency, missing owner, missing wire-contract rail, invalid readiness state, and missing downstream fixture evidence

## First-slice status

Current status is `extraction-ready-in-workspace`. The family manifest, policy entries, postcard wire-contract rails, downstream serialization fixture, compatibility tests, and deterministic dependency-boundary evidence are current. Do not raise any protocol crate above `extraction-ready-in-workspace` until human license/publication policy is resolved.
