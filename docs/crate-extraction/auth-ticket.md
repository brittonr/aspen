# Extraction Manifest: Auth and Tickets

## Candidate

- **Family**: `auth-ticket`
- **Canonical class**: `leaf type/helper`
- **Crates**: `aspen-auth-core`, `aspen-auth`, `aspen-ticket`, `aspen-hooks-ticket`
- **Intended audience**: Rust projects that need portable capability/token/ticket types, with runtime verification helpers only through explicit shells.
- **Public API owner**: owner needed
- **Readiness state**: `workspace-internal`

## Package metadata

- **Documentation entrypoint**: crate-level Rustdoc for `aspen-auth-core`, `aspen-ticket`, and `aspen-hooks-ticket`; runtime examples for `aspen-auth` only after feature contract is explicit.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: Aspen monorepo path until publication policy is decided.
- **Semver policy**: token/ticket serialization formats become compatibility contracts once ready.
- **Publication policy**: no publishable/repo-split state in this change.

## Feature contract

| Crate | Default contract | Runtime/adapter surface | First action |
| --- | --- | --- | --- |
| `aspen-auth-core` | Portable capabilities, operations, tokens, errors, postcard/base64 helpers. | none | Add goldens and malformed-token rejection. |
| `aspen-auth` | Runtime shell over auth-core. | HMAC, verifier, revocation storage, runtime auth helpers. | Document compatibility re-exports and runtime-only API boundary. |
| `aspen-ticket` | Portable ticket helper types with minimal defaults. | optional Iroh/gossip helpers. | Inventory feature gates and serialization format. |
| `aspen-hooks-ticket` | Portable hook trigger ticket URLs. | runtime hook crate re-exports only. | Prove malformed ticket rejection and lightweight defaults. |

## Dependency decisions

- Portable consumers should import token/capability types from `aspen-auth-core`.
- Runtime consumers use `aspen-auth` for HMAC/verifier/revocation storage.
- Hook config/event schema stays in `aspen-hooks-types`; hook ticket URLs stay in `aspen-hooks-ticket`.
- `iroh-base` key types are allowed key-only dependencies, not concrete transport runtime.

## Compatibility plan

- If `aspen-auth` keeps re-exporting portable types, document retention/removal owner, tests, and criteria.
- Representative consumers: `aspen-client-api`, `aspen-cli`, `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-hooks`, `aspen-ci`, Forge/CI ticket paths.

## Downstream fixture plan

- Fixture imports `Capability`, `Operation`, and token/ticket types directly from portable crates.
- Fixture serializes/deserializes valid token and hook ticket values.
- Negative fixture rejects malformed token bytes, malformed ticket URL, and runtime verifier/revocation API access from portable defaults.

## Verification rails

- Positive downstream: portable crate `cargo check`, serialization goldens, downstream fixture metadata/check/test.
- Negative boundary: malformed token/ticket tests and checker mutation for runtime `aspen-auth` dependency in portable consumers.
- Compatibility: compile/test representative consumers and any documented re-export paths.

## First blocker

Migrate portable consumers to canonical `aspen-auth-core` and `aspen-hooks-ticket` imports where possible, then document any retained runtime `aspen-auth` compatibility re-exports.
