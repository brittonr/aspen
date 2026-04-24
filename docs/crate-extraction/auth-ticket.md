# Extraction Manifest Stub: Auth and Tickets

## Candidate

- **Family**: Auth and tickets
- **Canonical class**: `leaf type/helper`
- **Crates**: `aspen-auth-core`, `aspen-auth`, `aspen-ticket`, `aspen-hooks-ticket`
- **Intended audience**: Rust projects that need portable capability/token/ticket types, plus optional runtime verification helpers when explicitly enabled.
- **Public API owner**: owner needed
- **Readiness state**: `workspace-internal`
- **Dependency policy class**: reusable leaf/helper family with runtime shell split

## Package and release metadata

- **Documentation entrypoint**: crate-level Rustdoc for `aspen-auth-core` and `aspen-hooks-ticket`; runtime examples for `aspen-auth` only after feature contract is explicit.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: monorepo path until publication policy is decided.
- **Semver/compatibility policy**: no external semver guarantee yet; token serialization formats become compatibility-relevant once ready.
- **Publish readiness**: blocked; do not mark publishable during this change.

## Feature contract

| Crate | Default-feature contract | Current status | Next action |
| --- | --- | --- | --- |
| `aspen-auth-core` | Portable capability, operation, token, and error types. | `workspace-internal` | Document wire/serialization compatibility and examples. |
| `aspen-auth` | Runtime shell for token builder/verifier, HMAC auth, revocation storage. | `workspace-internal` | Keep std/runtime helpers out of portable consumers. |
| `aspen-ticket` | Ticket helper types. | `workspace-internal` | Inventory dependencies and serialization compatibility. |
| `aspen-hooks-ticket` | Portable hook ticket URL type. | `workspace-internal` | Prove default deps stay lightweight and document examples. |

## Dependency decisions

- Portable consumers should prefer `aspen-auth-core` for capability/token types.
- Runtime consumers use `aspen-auth` for builder/verifier/HMAC/revocation behavior.
- Hook-ticket consumers use `aspen-hooks-ticket` or `aspen-hooks` re-exports only when the runtime crate is already desired.

## Compatibility and aliases

No new aliases are introduced by this stub. If runtime `aspen-auth` continues to re-export portable types, tests must cover both direct `aspen-auth-core` imports and compatibility imports with a removal/retention policy.

## Verification rails

- compile `aspen-auth-core` and `aspen-hooks-ticket` with minimal defaults;
- serialization/wire compatibility tests for token/ticket formats;
- negative checks proving runtime revocation/HMAC/storage APIs are unavailable from portable crates;
- dependency-boundary checker for transitive runtime leaks.

## First blocker

Document which consumers still import portable auth/ticket types through runtime shells, then migrate or record compatibility re-export policy.
