# Design: secrets core type boundary

## Context

Prior trust/crypto/secrets slices made crypto identity support explicit, moved token parsing to `aspen-auth-core`, replaced broad `aspen-core` KV signatures with lightweight KV crates, and gated handler runtime adapters. A fresh delegated inspection found the next smallest high-ROI boundary: portable secrets DTO/state modules and constants.

## Decisions

### 1. New `aspen-secrets-core` type/state crate

**Choice:** Move constants plus KV, Transit, and PKI type/state modules to `crates/aspen-secrets-core`.

**Rationale:** These modules are data contracts and pure helpers. They require only `serde` in normal builds and should not force consumers through runtime stores, cryptographic execution, SOPS IO, or handler adapters.

**Alternative:** Move all secrets storage/runtime code at once. Rejected because storage and cryptographic execution still require a larger boundary design.

### 2. Compatibility re-exports stay in `aspen-secrets`

**Choice:** Replace the old runtime-crate type modules with compatibility re-exports from `aspen-secrets-core`.

**Rationale:** Existing runtime code and downstream imports continue to compile while the core crate becomes the canonical owner.

### 3. Runtime execution remains out of core

**Choice:** Leave stores, SOPS, auth runtime helpers, trust integration, Nix cache manager, and handler adapters in existing runtime crates.

**Rationale:** This keeps the slice small and preserves the negative dependency guarantee for `aspen-secrets-core`.

## Risks / Trade-offs

- Some moved types contain secret/private-key material. The move preserves existing derives and behavior; it does not add display/logging behavior.
- `aspen-secrets-core` is not yet a full publishable package. Publication/license policy and broader public API review remain future work.
