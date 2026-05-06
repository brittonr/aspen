# Extraction Manifest: Auth and Tickets

## Candidate

- **Family**: `auth-ticket`
- **Canonical class**: `leaf type/helper`
- **Crates**: `aspen-auth-core`, `aspen-auth`, `aspen-ticket`, `aspen-hooks-ticket`
- **Intended audience**: Rust projects that need portable capability/token/ticket types, with runtime verification helpers only through explicit shells.
- **Public API owner**: Aspen auth and ticket maintainers
- **Readiness state**: `extraction-ready-in-workspace`

## Package metadata

- **Documentation entrypoint**: crate-level Rustdoc for `aspen-auth-core`, `aspen-ticket`, and `aspen-hooks-ticket`; runtime examples for `aspen-auth` only after feature contract is explicit.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: Aspen monorepo path until publication policy is decided.
- **Semver policy**: token/ticket serialization formats become compatibility contracts once ready.
- **Publication policy**: no publishable/repo-split state in this change.

## Feature contract

| Crate | Default contract | Runtime/adapter surface | First action |
| --- | --- | --- | --- |
| `aspen-auth-core` | Portable `Capability`, `Operation`, `CapabilityToken`, `Audience`, `AuthError`, postcard/base64 helpers, and pure verified-auth helpers. | none | Stable canonical import for portable consumers. |
| `aspen-auth` | Runtime shell over auth-core with compatibility re-exports for existing shell consumers. | HMAC, `TokenBuilder`, `TokenVerifier`, `Credential`, and revocation storage. | Runtime-only API boundary documented; portable consumers should not depend on it. |
| `aspen-ticket` | Portable `AspenClusterTicket`, `BootstrapPeer`, `ClusterTopicId`, `ClusterEndpointId`, and bounded ticket errors with default features disabled. | optional `iroh`, `signed`, and `std` helpers. | Serialization golden and malformed unsigned-ticket rejection pass. |
| `aspen-hooks-ticket` | Portable `AspenHookTicket`, `HookTicketError`, ticket constants, and bounded URL-safe hook ticket serialization. | runtime hook crate re-exports only. | Serialization golden and invalid hook-ticket rejection pass. |

## Dependency decisions

- Portable consumers should import token/capability types from `aspen-auth-core`.
- Runtime consumers use `aspen-auth` for HMAC/verifier/revocation storage.
- Hook config/event schema stays in `aspen-hooks-types`; hook ticket URLs stay in `aspen-hooks-ticket`.
- `iroh-base` key types are allowed key-only dependencies, not concrete transport runtime.

## Compatibility plan

- `aspen-auth` keeps compatibility re-exports for `Capability`, `Operation`, `CapabilityToken`, `Audience`, `AuthError`, `constants`, `token`, `verified_auth`, and `verified_credential`; new portable crates import from `aspen-auth-core` directly.
- `aspen-hooks` may re-export `aspen-hooks-ticket` for runtime hook consumers, but portable ticket consumers import from `aspen-hooks-ticket` directly.
- Representative consumers: `aspen-client-api`, `aspen-cli`, `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-hooks`, `aspen-ci`, Forge/CI ticket paths.

## Downstream fixture plan

- Fixture imports `Capability`, `Operation`, and token/ticket types directly from portable crates.
- Fixture serializes/deserializes valid token and hook ticket values.
- Negative fixture rejects malformed token bytes, malformed ticket URL, and runtime verifier/revocation API access from portable defaults.

## Verification rails

- Positive downstream: portable crate `cargo check`, serialization goldens, downstream fixture metadata/check/test.
- Negative boundary: malformed token/ticket tests and dependency-boundary checker mutation for runtime `aspen-auth` dependency in portable consumers.
- Compatibility: compile/test representative consumers and any documented re-export paths.

## Readiness decision

The auth/ticket family is `extraction-ready-in-workspace`: canonical portable imports are documented, compatibility re-export ownership is explicit, token/ticket serialization goldens and malformed-input rejection pass, the downstream portable fixture compiles, and the negative runtime-verifier fixture proves portable defaults cannot reach `aspen-auth` verifier/revocation APIs. Publishable/repo-split status remains blocked on human license/publication policy.

## Evidence status

- `aspen-auth-core` now has deterministic capability token binary/base64 roundtrip coverage and malformed token/base64 rejection tests.
- `aspen-ticket` and `aspen-hooks-ticket` pin deterministic ticket serialization goldens and roundtrip those goldens through deserializers.
- `auth-ticket-portable-smoke` proves portable consumers can compile against portable auth/ticket crates without the runtime `aspen-auth` shell.
- `auth-ticket-runtime-negative` proves verifier/revocation runtime APIs are unavailable from portable defaults unless `aspen-auth` is explicitly added.
