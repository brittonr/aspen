## Why

Aspen foundational type crates are already split far enough that stale storage/Redb blockers are resolved, but the family still needs an owner-grade public API review before readiness can be raised or rejected with evidence.

## What Changes

- **Classify each foundational crate as reusable API, compatibility shell, or internal helper**: Classify each foundational crate as reusable API, compatibility shell, or internal helper.
- **Pin canonical import paths and compatibility shims for portable consumers**: Pin canonical import paths and compatibility shims for portable consumers.
- **Record readiness evidence using the live no-std and extraction-boundary checkers**: Record readiness evidence using the live no-std and extraction-boundary checkers.

## Capabilities

### New Capabilities
- `foundational-types-extraction`: Review foundational types public API readiness/evidence requirements.

### Modified Capabilities
- Existing extraction and dogfood evidence inventories gain an active implementation target with explicit verification rails.

## Impact

- **Files**: OpenSpec artifacts under `openspec/changes/review-foundational-types-public-api/`.
- **APIs**: No immediate code API change; implementation tasks will decide stable public API or evidence surfaces.
- **Dependencies**: No dependency change in this spec-only slice.
- **Testing**: `openspec validate review-foundational-types-public-api --strict`, helper verification, `git diff --check`, and the change-specific verification tasks.
