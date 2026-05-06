## Why

Auth/ticket serialization goldens and malformed-input coverage existed, but the reusable API boundary still needed an explicit readiness decision: portable consumers should depend on `aspen-auth-core`, `aspen-ticket`, and `aspen-hooks-ticket`, while runtime verifier/HMAC/revocation APIs remain in `aspen-auth` or runtime adapters.

## What Changes

- **Review portable auth/token/ticket crates as public API candidates**: Document canonical portable imports, stable types, compatibility re-export ownership, and maintainers for `aspen-auth-core`, `aspen-ticket`, and `aspen-hooks-ticket`.
- **Keep HMAC/verifier/revocation/runtime storage in runtime shells or gated adapters**: Document `aspen-auth` as the runtime shell and add negative fixture evidence proving portable consumers cannot reach `TokenVerifier`/revocation APIs without depending on `aspen-auth`.
- **Prove portable consumers and negative runtime-boundary fixtures**: Add standalone positive/negative fixtures, focused golden/malformed tests, dependency graph evidence, and readiness checker artifacts.
- **Raise workspace readiness only**: Promote the auth/ticket family to `extraction-ready-in-workspace`; publication/repo-split remains blocked on human license/publication policy.

## Capabilities

### New Capabilities
- `auth-ticket-extraction`: Review auth and ticket public API readiness/evidence requirements.

### Modified Capabilities
- Extraction inventory and policy record the auth/ticket readiness decision and evidence rail.

## Impact

- **Files**: `docs/crate-extraction.md`, `docs/crate-extraction/auth-ticket.md`, `docs/crate-extraction/policy.ncl`, and OpenSpec artifacts under `openspec/changes/review-auth-ticket-public-api/`.
- **APIs**: No production code API change; canonical imports and compatibility ownership are documented.
- **Dependencies**: No workspace dependency change; the downstream fixture patches `iroh-tickets` to Aspen's vendored graph.
- **Testing**: Positive downstream fixture, negative runtime-boundary fixture, auth/token/ticket serialization goldens, malformed-input rejection, extraction readiness checker, `openspec validate review-auth-ticket-public-api --strict`, `scripts/openspec-preflight.sh review-auth-ticket-public-api`, and `git diff --check`.

## Verification Expectations

- Requirement `auth-ticket-extraction.portable-api-owned` / scenario `auth-ticket-extraction.portable-api-owned.evidence`: `verification.md` MUST include changed files and evidence showing canonical portable imports, stable types, compatibility re-exports, and owner expectations.
- Requirement `auth-ticket-extraction.runtime-leakage-rejected` / scenario `auth-ticket-extraction.runtime-leakage-rejected.evidence`: the portable downstream fixture MUST compile without root Aspen, `aspen-auth`, handler crates, or concrete Iroh runtime dependencies, and the negative runtime fixture MUST fail because `aspen_auth::TokenVerifier` is unavailable without an explicit `aspen-auth` dependency.
- Requirement `auth-ticket-extraction.workspace-readiness-evidenced` / scenario `auth-ticket-extraction.workspace-readiness-evidenced.evidence`: readiness checker evidence MUST pass for `--candidate-family auth-ticket` before archive while preserving the license/publication blocker.
- `verification.md` MUST include a `## Verification Commands` section listing exact commands and artifacts.
