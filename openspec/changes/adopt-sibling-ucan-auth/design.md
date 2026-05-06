## Context

Aspen has an alloc-focused `aspen-auth-core` crate for `Capability`, `Operation`, `CapabilityToken`, `Audience`, `AuthError`, and pure helpers, plus a runtime `aspen-auth` shell for builders, verifiers, HMAC, and revocation/storage integration. The sibling `../ucan` repository now exposes a reusable UCAN implementation with a `#![no_std]` `ucan-core` crate and a std shell for token issuance, signer-backed issuance, parsing, verification, and proof-chain APIs.

This change is a migration contract, not a blind dependency swap. Aspen still owns Aspen-specific capability vocabulary, Raft/RPC admission policy, redacted receipts, and compatibility expectations. UCAN should own the generic token/proof-chain semantics once adapter evidence proves the boundary.

## Goals / Non-Goals

**Goals:**
- Make the sibling `../ucan` implementation the source of truth for UCAN token issuance, parsing, verification, and proof-chain semantics used by Aspen.
- Keep `aspen-auth-core` portable/alloc-focused and prevent `std`, filesystem, signer storage, or runtime revocation dependencies from leaking into protected graphs.
- Preserve existing Aspen CLI/RPC capability vocabulary through explicit translations or documented migration receipts.
- Require negative evidence for capability escalation, expired tokens, invalid proof links, replay/revocation policy, and missing sibling dependency/source wiring.

**Non-Goals:**
- Do not rewrite all Aspen authorization call sites in one unreviewed sweep.
- Do not remove Aspen-specific capability enums, operation checks, or runtime policy vocabulary unless a later OpenSpec explicitly replaces them.
- Do not claim production interoperability with arbitrary UCAN ecosystems beyond the sibling crate's verified public surface.
- Do not rely on an unreproducible `../ucan` path for release artifacts without a pin/vendor/override plan recorded in evidence.

## Decisions

### 1. UCAN owns generic token semantics

**Choice:** Aspen's token issuance, parsing, signature/proof-chain verification, expiry checks, and attenuation checks SHALL route through the sibling UCAN public APIs where those semantics are generic UCAN behavior.

**Rationale:** This prevents two local kernels from drifting and lets Aspen benefit from UCAN's controlled-integration proof-chain work.

**Alternative:** Keep Aspen's bespoke `CapabilityToken`/`Credential` verifier as the primary implementation and only compare against UCAN in tests. Rejected because the user's requested direction is to switch to `../ucan`, not merely evaluate it.

### 2. Aspen keeps an adapter boundary

**Choice:** `aspen-auth-core` remains the Aspen-facing compatibility boundary. It may wrap or translate UCAN core types, but Aspen capabilities, operations, CLI strings, RPC wire expectations, and redacted receipt summaries remain explicitly modeled in Aspen.

**Rationale:** Aspen authorization includes project-specific capabilities (`KV`, `Forge`, `CI`, `snix`, federation, trust/secrets, runtime services) that are not generic UCAN concepts. The adapter keeps those stable while replacing generic token mechanics.

**Alternative:** Expose UCAN structs directly everywhere. Rejected because it would couple every Aspen call site to sibling crate internals and make compatibility/migration evidence hard to localize.

### 3. Relative dependency needs a reproducibility plan

**Choice:** Implementation SHALL start from the local sibling path requested by the user (`../ucan`), but acceptance requires evidence for Nix/CI/release source behavior: either the local path is intentionally a developer-only override with a pinned fallback, or the dependency is vendored/pinned in a way Aspen's flake/source filters can build reproducibly.

**Rationale:** Relative dependencies outside the flake tree are convenient for co-development but often fail in cleaned Nix sources or CI checkouts.

**Alternative:** Immediately vendor UCAN into Aspen. Rejected for this OpenSpec because the user asked for `../ucan`; vendoring may be the eventual release answer but should be decided with evidence.

## Verification Strategy

- Add mapping tests that issue UCAN tokens through the sibling API and authorize equivalent Aspen operations through the adapter.
- Add compatibility fixtures for current Aspen token CLI/RPC behavior or explicit migration receipts for any intentional break.
- Add negative tests for escalation, expired tokens, malformed proof links, replay/revocation policy, wrong audience, and denied capability mappings.
- Run dependency graph checks proving `aspen-auth-core --no-default-features` stays alloc/no-std-compatible and runtime-only UCAN shell dependencies stay out of protected paths.
- Run Nix/CI source-boundary checks proving the chosen `../ucan` wiring works or fails with an intentional, documented, actionable fallback.
- Run strict OpenSpec validation, helper verification, and whitespace checks before commit/archive.

## Risks / Trade-offs

**Token compatibility drift** → Mitigate with CLI/RPC fixtures and migration receipts before runtime admission switches to UCAN-backed verification.

**Sibling path reproducibility** → Mitigate with Nix/CI evidence and a documented pin/vendor/override strategy before claiming release readiness.

**no_std boundary regression** → Mitigate by depending on `ucan-core` from portable crates and keeping the std `ucan` shell in `aspen-auth` or other runtime crates only.

**Semantic mismatch between Aspen capabilities and UCAN abilities** → Mitigate with an explicit ability/capability mapping table, attenuation tests, and negative escalation fixtures.
