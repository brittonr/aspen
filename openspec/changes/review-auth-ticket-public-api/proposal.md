## Why

Auth/ticket serialization goldens and malformed-input coverage are in place; the remaining blocker is to decide the stable reusable API and keep runtime verifier shells explicit.

## What Changes

- **Review portable auth/token/ticket crates as public API candidates**: Review portable auth/token/ticket crates as public API candidates.
- **Keep HMAC/verifier/revocation/runtime storage in runtime shells or gated adapters**: Keep HMAC/verifier/revocation/runtime storage in runtime shells or gated adapters.
- **Prove portable consumers and negative runtime-boundary fixtures**: Prove portable consumers and negative runtime-boundary fixtures.

## Capabilities

### New Capabilities
- `auth-ticket-extraction`: Review auth and ticket public API readiness/evidence requirements.

### Modified Capabilities
- Existing extraction and dogfood evidence inventories gain an active implementation target with explicit verification rails.

## Impact

- **Files**: OpenSpec artifacts under `openspec/changes/review-auth-ticket-public-api/`.
- **APIs**: No immediate code API change; implementation tasks will decide stable public API or evidence surfaces.
- **Dependencies**: No dependency change in this spec-only slice.
- **Testing**: `openspec validate review-auth-ticket-public-api --strict`, helper verification, `git diff --check`, and the change-specific verification tasks.
