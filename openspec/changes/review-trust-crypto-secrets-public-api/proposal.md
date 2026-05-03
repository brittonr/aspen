## Why

The trust/crypto/secrets extraction family has completed a sequence of boundary slices: transport-free `aspen-crypto` defaults, portable secrets auth parsing, lightweight KV traits, explicit secrets-handler runtime adapters, `aspen-secrets-core` type ownership, and the secrets mount-provider boundary. The remaining documented gate before any readiness promotion is an owner/public API review that decides which surfaces are reusable APIs and which remain runtime or compatibility consumers.

## What Changes

- **Public API review**: Record canonical reusable trust/crypto/secrets surfaces, compatibility re-export ownership, runtime-adapter exclusions, and blockers.
- **Readiness decision**: Keep the family at `workspace-internal` until checker coverage, serialization contracts, and async/runtime policy are explicit.
- **Fresh evidence**: Capture current dependency graphs, package checks, source/API findings, and readiness-checker behavior under this change.

## Capabilities

### Modified Capabilities

- `trust-crypto-secrets-extraction`: Adds owner/public API review as a required readiness gate for the trust/crypto/secrets extraction family.

## Impact

- **Files**: `docs/crate-extraction.md`, `docs/crate-extraction/trust-crypto-secrets.md`, `openspec/specs/trust-crypto-secrets-extraction/spec.md`, and this change package.
- **APIs**: No Rust API changes; this is a readiness and ownership review.
- **Dependencies**: No dependency changes.
- **Testing**: Fresh crate checks, dependency-tree/source guards, readiness checker output, OpenSpec validation, markdownlint, and `git diff --check`.
