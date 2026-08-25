# Design

## Context

Pinned Octet computes metadata hashes over JSON whose object keys use lexical order. Molten constructs equivalent `serde_json::Value` objects, but workspace feature unification enables insertion-order preservation. The same fields and values therefore serialize in a different order and produce different BLAKE3 identities.

## Ownership

Octet owns the `status.json` metadata contract. Molten is a consumer and must reproduce that published serialization exactly when it checks freshness. Dependency feature selection must not alter evidence identity.

## Architecture

The existing gate shell continues to read `Cargo.toml` and `dylint.toml`. Typed serialization payloads declare fields in pinned Octet's lexical order. Struct serialization preserves declaration order independently from `serde_json` map features. No new port or adapter is needed because the capability boundary does not change.

## Invariants

- Configuration and profile payload identities match pinned Octet revision `fc38f59330b626961d166febfdf1a5aa6575460f`.
- File identities retain pinned Octet's raw BLAKE3 semantics.
- Missing files remain represented as absent values.
- Warning-only, critical, malformed, or stale evidence still denies strict admission.
- Molten content references outside this interoperability field keep their existing framed semantics.

## Validation

Run the existing Octet gate tests before and after the change. Add canonical-order positive and changed-input negative tests. Then run formatting, Clippy, pinned Octet evidence, strict Cairn gates, and the broad Nix rail.
