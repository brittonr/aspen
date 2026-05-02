## Why

Jobs/CI post-extraction cleanup moved portable payload, wire, keyspace, route, status, and handle contracts onto their owning surfaces. The remaining documented blocker before any readiness change is an owner/public API review that records which surfaces are canonical reusable APIs, which paths are compatibility shells, and which crates remain runtime adapters.

## What Changes

- **Public API review**: Record Jobs/CI canonical reusable API surfaces, compatibility re-export ownership, runtime adapter exclusions, and readiness criteria.
- **Fresh evidence plan**: Define the evidence required before raising `jobs-ci-core` beyond `workspace-internal`.
- **Spec hygiene**: Replace the archived placeholder purpose and add a requirement for owner/public API review.

## Capabilities

### Modified Capabilities

- `jobs-ci-core-extraction`: Adds owner/public API review as an explicit readiness gate for the Jobs/CI extraction family.

## Impact

- **Files**: `openspec/specs/jobs-ci-core-extraction/spec.md`, `docs/crate-extraction.md`, `docs/crate-extraction/jobs-ci-core.md`, and this change package.
- **APIs**: No Rust API changes in the review-start slice.
- **Dependencies**: No dependency changes.
- **Testing**: OpenSpec helper verification, markdown linting, stale-path grep, and crate-extraction readiness checker runs using checked-in evidence or active-change evidence as appropriate.
