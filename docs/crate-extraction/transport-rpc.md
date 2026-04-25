# Transport/RPC Extraction Manifest

## Candidate

- Family: `transport-rpc`
- Candidates: `aspen-transport`, `aspen-rpc-core`
- Canonical class: runtime adapter plus service library
- Owner: Aspen transport/RPC maintainers
- Audience: downstream Rust services that want Aspen's Iroh protocol helper and RPC dispatch pattern without the Aspen node runtime.
- Readiness: `workspace-internal`

## Package and release metadata

- Packages stay workspace-internal while this split is proven.
- License follows workspace AGPL-3.0-or-later policy.
- Repository/homepage metadata remains the Aspen monorepo until human publication policy exists.
- Semver policy: default reusable APIs may stabilize after downstream fixtures and compatibility rails keep passing.

## Feature contract

### `aspen-transport`

- `default`: protocol identifiers, bounded Iroh connection/stream helpers, snapshot history, and pure verified encoding helpers.
- `raft-rpc`: OpenRaft RPC request/response wire helpers and IRPC request declarations.
- `raft-handler`: legacy unauthenticated Raft protocol handler plus metrics and postcard framing.
- `auth-raft-handler`: authenticated Raft protocol handler and trusted-peer registry.
- `sharded-raft-handler`: sharded Raft handler and shard-prefix routing integration.
- `trust-handler`: trust-share request protocol handler.
- `log-subscriber`: log subscriber wire protocol and server/client support.
- `runtime`: compatibility bundle for Aspen runtime consumers.

### `aspen-rpc-core`

- `default`: `ClientProtocolContext` empty token, `RequestHandler`, `HandlerFactory`, `ServiceExecutor`, `ServiceHandler`, `DispatchRegistry`, and proxy config.
- `runtime-context`: full Aspen `ClientProtocolContext`, network metrics provider, span forwarder, and runtime service fields.
- Domain features (`blob`, `forge`, `ci`, `jobs`, `hooks`, `worker`, `deploy`, `sql`, `testing`) all imply `runtime-context` when they need concrete service graph fields.

## Dependencies

### `aspen-transport`

- Default Aspen dependency: `aspen-constants` for canonical ALPN and limit data.
- Default adapter dependency: `iroh`, because Iroh endpoint/connection/stream types are the explicit transport surface.
- Feature-gated dependencies: `openraft`, `irpc`, `postcard`, `metrics`, `aspen-raft-types`, `aspen-core-shell`, `aspen-auth`, `aspen-sharding`, and `aspen-trust`.
- Forbidden by default: root `aspen`, handlers, cluster bootstrap, auth runtime, trust, sharding, Raft compatibility, and binary shells.

### `aspen-rpc-core`

- Default Aspen dependencies: `aspen-client-api` and `aspen-constants` for request/response dispatch and proxy defaults.
- Default external dependencies: `anyhow`, `async-trait`, and `serde`.
- Feature-gated dependencies: full Aspen context graph, Raft, transport, sharding, coordination, jobs, Forge, CI, hooks, blob, testing, metrics exporter, tokio, tracing, and Iroh.
- Forbidden by default: concrete node runtime, handler crates, app shells, and domain service implementations.

## Compatibility and aliases

- `aspen-transport` compatibility re-exports: none. Existing imports remain available when consumers enable the documented feature bundle.
- `aspen-rpc-core` compatibility re-exports: `aspen-rpc-handlers` continues to re-export core handler traits for runtime users.
- `aspen-raft`, `aspen-cluster`, `aspen-client`, and `aspen-cluster-bridges` now opt into the exact transport feature bundle they use.
- Handler crates that need concrete context fields opt into `aspen-rpc-core/runtime-context` directly or through their domain features.
- Compatibility removal plan: keep runtime feature bundles until all callers import from narrower adapter crates or explicitly own their feature needs.

## Representative consumers

- `aspen-raft-network`: Raft network adapter compatibility.
- `aspen-raft`: log subscriber, trust share, authenticated Raft, and snapshot metrics compatibility.
- `aspen-cluster`: node bootstrap router compatibility.
- `aspen-client`: watch/log-subscriber protocol compatibility.
- `aspen-rpc-handlers`: full runtime context and handler registry compatibility.
- `openspec/changes/split-transport-rpc-core/fixtures/downstream-transport`: positive downstream transport fixture.
- `openspec/changes/split-transport-rpc-core/fixtures/downstream-rpc-core`: positive downstream RPC fixture.

## Dependency exceptions

- `aspen-transport -> iroh`: default adapter-purpose dependency; Iroh connection and stream types are the public reusable API.
- `aspen-transport[raft-rpc] -> irpc`: named feature owns IRPC request declarations for Raft RPC compatibility.
- `aspen-rpc-core -> aspen-client-api`: request and response enums are the dispatch contract, not a runtime service implementation.
- `aspen-rpc-core -> aspen-client-api -> aspen-auth-core -> iroh-base`: key-only public key type path inherited from protocol schema; no Iroh endpoint/runtime is exposed by default.

## Verification rails

- positive downstream: compile both downstream fixtures with default features only and save cargo metadata.
- negative boundary: grep fixture metadata for root Aspen, cluster bootstrap, trust, sharding, Raft compatibility, handler crates, and binary shells.
- compatibility: compile representative runtime consumers through explicit feature bundles.
- dependency-boundary: run `cargo tree -p aspen-transport -e normal` and `cargo tree -p aspen-rpc-core -e normal`, then run `scripts/check-crate-extraction-readiness.rs --candidate-family transport-rpc`.
- negative boundary mutations: prove checker failure for unowned runtime dependency, missing owner, invalid readiness state, missing downstream fixture, and missing compatibility evidence.

## Blocked reasons and next action

- Readiness remains `workspace-internal` until final evidence proves all default graphs and runtime compatibility rails.
- Publication/repository split remains blocked on human license/publication policy even after technical rails pass.
- Next action: keep narrowing runtime-context fields into smaller adapter crates after this staged split lands.
