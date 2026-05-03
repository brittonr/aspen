# Design

## Context

The canonical reusable trust/crypto/secrets candidates are `aspen-crypto`, `aspen-secrets-core`, and selected pure `aspen-trust` helper/state/wire surfaces. The remaining blocker before any readiness promotion is explicit serialization-contract and golden/roundtrip evidence for trust and secrets state.

## Decisions

### Plain Rust contract tests

Use ordinary Rust tests with inline golden byte arrays and `serde_json::Value` comparisons. This matches existing Aspen test style and avoids snapshot/golden-file tooling dependencies.

### Stable trust formats

Treat the following as current compatibility contracts:

- `Share::to_bytes` / `Share::from_bytes` 33-byte layout.
- `EncryptedValue::to_bytes` / `EncryptedValue::from_bytes` envelope layout.
- `TrustRequest` / `TrustResponse` postcard enum layout for existing variants.
- `Threshold` JSON scalar representation.
- `EncryptedSecretChain` JSON field representation for persisted chain state.

### Stable secrets-core state formats

Treat KV, Transit, and PKI persisted state/config structs that already derive serde as stable JSON contracts. Request/response DTOs that do not derive serde remain service implementation convenience types, not serialization compatibility contracts, until a later change explicitly derives serde and adds goldens.

## Risks

- HashMap serialization order is not stable for raw-string goldens, so tests compare `serde_json::Value` instead of exact JSON strings for map-bearing types.
- Postcard enum bytes intentionally pin current variant order; new variants must be appended or guarded by compatibility tests.
