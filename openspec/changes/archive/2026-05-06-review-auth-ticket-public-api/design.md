## Context

Auth/ticket serialization goldens and malformed-input coverage are now paired with an explicit public API/readiness decision. `aspen-auth-core`, `aspen-ticket`, and `aspen-hooks-ticket` own the portable surfaces; `aspen-auth` remains the runtime shell for builders/verifiers/HMAC/revocation storage and retains compatibility re-exports for existing shell consumers.

## Goals / Non-Goals

**Goals:**
- Define canonical portable imports and compatibility re-export ownership for auth/ticket consumers.
- Preserve current Aspen compatibility while proving portable downstream fixture coverage.
- Prove runtime verifier, HMAC, revocation storage, root Aspen, and handler APIs do not leak into portable defaults.
- Promote the auth/ticket extraction family to `extraction-ready-in-workspace` with evidence.

**Non-Goals:**
- Do not publish or split crates out of the Aspen monorepo; license/publication policy remains a blocker.
- Do not remove `aspen-auth` compatibility re-exports in this change.
- Do not rewrite runtime verifier, HMAC, or revocation storage APIs.

## Decisions

### 1. Canonical portable imports live in portable crates

**Choice:** Portable consumers import `Capability`, `Operation`, `CapabilityToken`, `Audience`, `AuthError`, and pure verified helpers from `aspen-auth-core`; cluster bootstrap ticket types from `aspen-ticket`; and hook trigger ticket types from `aspen-hooks-ticket`.

**Rationale:** These crates are dependency-light, bounded, and usable without the runtime verifier shell. They are the surfaces a downstream fixture can compile without root Aspen, handlers, `aspen-auth`, concrete Iroh runtime, or revocation storage.

**Alternative:** Keep new portable consumers on `aspen-auth` re-exports. Rejected because `aspen-auth` intentionally owns runtime-only builders/verifiers/HMAC/revocation helpers.

### 2. Runtime-shell compatibility remains explicit

**Choice:** `aspen-auth` keeps compatibility re-exports for existing runtime consumers, but documentation now names them as shell compatibility rather than canonical portable imports.

**Rationale:** This avoids a broad migration while preventing future reusable crates from accidentally depending on runtime verification/storage APIs.

### 3. Evidence uses standalone downstream fixtures

**Choice:** The positive fixture imports only portable auth/ticket crates and patches `iroh-tickets` to the workspace-vendored graph; the negative fixture depends only on `aspen-auth-core` and intentionally fails when importing `aspen_auth::TokenVerifier`.

**Rationale:** The fixtures prove both sides of the boundary independently of Aspen workspace membership and preserve the existing vendored iroh dependency contract.

## Verification Strategy

- For `auth-ticket-extraction.portable-api-owned` and `auth-ticket-extraction.portable-api-owned.evidence`, run focused serialization-golden and malformed-input tests for `aspen-auth-core`, `aspen-ticket`, and `aspen-hooks-ticket`, then record canonical import/owner evidence in docs and `verification.md`.
- For `auth-ticket-extraction.runtime-leakage-rejected` and `auth-ticket-extraction.runtime-leakage-rejected.evidence`, run the positive downstream fixture, save metadata evidence, run the negative runtime-verifier fixture as an expected failure, and run `cargo tree` over the positive fixture to scan for forbidden runtime dependencies.
- For `auth-ticket-extraction.workspace-readiness-evidenced` and `auth-ticket-extraction.workspace-readiness-evidenced.evidence`, run the crate-extraction readiness checker for `auth-ticket` after docs/policy/inventory are updated.
- Run strict OpenSpec validation, repo preflight, and whitespace checks before committing/archive.

## Risks / Trade-offs

**Standalone fixture dependency skew** → The positive fixture patches `iroh-tickets` to the workspace-vendored graph so it uses the same iroh-base family as Aspen.

**False publication signal** → Readiness is limited to `extraction-ready-in-workspace`; publication/repo-split remains blocked on license policy.

**Compatibility ambiguity** → `aspen-auth` re-exports are explicitly documented as compatibility shell ownership, while canonical portable imports live in `aspen-auth-core` and ticket crates.
