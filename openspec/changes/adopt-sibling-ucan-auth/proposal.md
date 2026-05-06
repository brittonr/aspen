## Why

Aspen currently owns bespoke capability-token, delegation, and verifier logic in `aspen-auth-core`/`aspen-auth`, while the sibling `../ucan` repository now contains the reusable UCAN kernel and std shell. Keeping two independent auth kernels will create drift in attenuation, proof-chain, expiration, DID/signature, and interoperability behavior. Aspen should switch to the sibling UCAN implementation through a bounded adapter instead of continuing to evolve a parallel token format.

## What Changes

- **Adopt sibling UCAN as source of truth**: Use `../ucan` / `../ucan/crates/ucan-core` for UCAN issuance, parsing, verification, and proof-chain semantics at Aspen's auth boundary.
- **Preserve Aspen capability vocabulary**: Keep Aspen-specific capabilities, operations, receipts, and existing CLI/RPC expectations behind explicit translation/adapters rather than leaking UCAN internals everywhere.
- **Migrate compatibly and fail closed**: Define compatibility behavior for existing Aspen tokens, migration fixtures, and negative proofs before changing runtime admission.
- **Document dependency boundary**: Capture how the relative sibling dependency is wired for local development, Nix/CI, and eventual pin/vendor/release usage.

## Capabilities

### New Capabilities
- `ucan-auth-integration`: Aspen auth can be backed by the sibling UCAN implementation through explicit adapter and migration requirements.

### Modified Capabilities
- `auth-ticket-extraction`: Portable auth crates keep their dependency-light shape, but UCAN semantics become delegated to the sibling UCAN core where accepted by this change.
- `federation-credential`: Federation credentials/proof chains must align with UCAN proof-chain verification rather than an Aspen-only credential kernel.

## Impact

- **Files**: `Cargo.toml`, `flake.nix`/Nix source wiring if needed, `crates/aspen-auth-core`, `crates/aspen-auth`, `src/bin/aspen-token.rs`, federation credential code/tests, docs under `docs/`, and this OpenSpec change.
- **APIs**: Existing Aspen CLI/RPC-facing token commands and capability names should remain stable unless migration evidence proves a breaking change is required and documented.
- **Dependencies**: Introduces a controlled sibling dependency on `../ucan`; implementation must prove it does not leak `std` into protected `aspen-auth-core` / alloc-only paths unless gated through the runtime shell.
- **Testing**: UCAN round-trip/proof-chain fixtures, Aspen compatibility fixtures, negative escalation/expiry/revocation/replay cases, dependency-tree checks, Nix/CI source-boundary checks, `openspec validate adopt-sibling-ucan-auth --strict`, helper verification, and `git diff --check`.

## Verification Expectations

- Requirement `ucan-auth-integration.sibling-source-of-truth` proves the accepted auth path calls UCAN issuance/parsing/verification/proof-chain APIs from the sibling repository rather than duplicating equivalent logic in Aspen.
- Requirement `ucan-auth-integration.adapter-preserves-aspen-boundary` proves Aspen capability vocabulary and operator-facing token CLI/RPC behavior remain stable behind an adapter or have documented migration receipts.
- Requirement `ucan-auth-integration.dependency-boundary-evidenced` proves the sibling dependency is reproducible for local/Nix/CI use and does not contaminate protected no-std/alloc-only graphs.
