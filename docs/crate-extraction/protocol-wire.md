# Extraction Manifest Stub: Protocol and Wire Crates

## Candidate

- **Family**: Protocol/wire
- **Canonical class**: `protocol/wire`
- **Crates**: `aspen-client-api`, `aspen-forge-protocol`, `aspen-jobs-protocol`, `aspen-coordination-protocol`
- **Intended audience**: Rust projects that need Aspen wire schemas, request/response enums, and protocol compatibility without handler registries or runtime transports.
- **Public API owner**: owner needed
- **Readiness state**: `workspace-internal`
- **Dependency policy class**: reusable protocol/wire candidates

## Package and release metadata

- **Documentation entrypoint**: crate-level Rustdoc plus serialization compatibility docs.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: monorepo path until publication policy is decided.
- **Semver/compatibility policy**: postcard discriminants and encoded formats are append-only compatibility contracts once ready.
- **Publish readiness**: blocked; do not mark publishable during this change.

## Feature contract

| Crate | Default-feature contract | Current status | Next action |
| --- | --- | --- | --- |
| `aspen-client-api` | Client request/response wire schema with optional domains explicitly feature-gated. | `workspace-internal` | Preserve postcard baselines; tighten alloc/default feature surface; keep auth pointed at `aspen-auth-core`. |
| `aspen-forge-protocol` | Forge wire types. | `workspace-internal` | Demote `serde_json` to dev/test surface where possible. |
| `aspen-jobs-protocol` | Jobs wire types. | `workspace-internal` | Preserve wire compatibility and remove test-only runtime baggage. |
| `aspen-coordination-protocol` | Coordination wire types. | `workspace-internal` | Keep protocol types independent from coordination runtime. |

## Dependency decisions

- Protocol crates must not depend on handler registries, node bootstrap, dogfood, UI/TUI/web, or concrete iroh endpoints by default.
- `aspen-client-api` may depend on portable protocol crates and `aspen-auth-core`, but not runtime `aspen-auth` for portable auth types.
- Serialization features must be explicit and tested with postcard baselines.

## Compatibility and aliases

No aliases are introduced by this stub. Any future enum variant additions must preserve append-only postcard discriminants and update wire compatibility evidence.

## Verification rails

- compile protocol crates with default/minimal features;
- postcard/golden compatibility tests for public wire types;
- dependency-boundary checker proving no handler/runtime/binary leaks;
- positive examples that serialize/deserialize canonical wire types;
- negative checks proving handler registries and concrete transport APIs are unavailable.

## First blocker

`aspen-client-api` is the highest priority because it is the main cross-domain wire surface. Keep it pointed at `aspen-auth-core`, preserve postcard baselines, and audit optional domain features for transitive runtime leaks.
