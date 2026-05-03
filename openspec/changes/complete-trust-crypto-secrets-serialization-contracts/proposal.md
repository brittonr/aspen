# Complete trust/crypto/secrets serialization contracts

## Why

The trust/crypto/secrets family still remains `workspace-internal` after the owner/API review, checker real-crate coverage, and `aspen-trust` async/default policy because stable serialization contracts lack golden/roundtrip evidence.

## What Changes

- Add deterministic serialization contract tests for `aspen-trust` share, envelope, protocol, threshold, and chain state formats.
- Add deterministic JSON contract tests for `aspen-secrets-core` KV, Transit, and PKI persisted state/config types.
- Record which formats are stable compatibility contracts and which request/response DTOs remain internal until explicitly serialized.
- Keep `trust-crypto-secrets` at `workspace-internal`; this change provides evidence but does not promote the aggregate family.

## Impact

- **Files**: `aspen-trust` tests, `aspen-secrets-core` tests, extraction docs, readiness evidence.
- **Dependencies**: no runtime dependency changes.
- **Testing**: focused cargo tests for both crates, readiness checker, OpenSpec validation, markdownlint, and diff hygiene.
