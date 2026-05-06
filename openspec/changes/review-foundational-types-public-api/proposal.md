## Why

Aspen foundational type crates are split far enough for a reviewed public API boundary. This drain records owner-grade classifications, canonical imports, downstream fixture evidence, live no-std/readiness checks, and promotes the family to `extraction-ready-in-workspace` while leaving publication/repo-split decisions blocked on license policy.

## What Changes

- **Classify each foundational crate as reusable API, compatibility shell, or internal helper**: `aspen-constants`, `aspen-hlc`, `aspen-storage-types`, `aspen-cluster-types`, `aspen-traits`, and `aspen-time` are recorded as reusable foundational APIs with explicit feature boundaries and no root `aspen` dependency for the downstream fixture.
- **Pin canonical import paths and compatibility shims for portable consumers**: Canonical imports are documented in `docs/crate-extraction/foundational-types.md`; `aspen-traits` keeps compatibility re-exports for portable request types while async/runtime traits remain gated.
- **Record readiness evidence using the live no-std and extraction-boundary checkers**: The change adds downstream fixture metadata, forbidden runtime dependency checks, no-default package checks, the aspen-core no-std checker transcript, and extraction-readiness reports.

## Capabilities

### New Capabilities
- `foundational-types-extraction`: Review foundational types public API readiness/evidence requirements.

### Modified Capabilities
- Existing extraction and dogfood evidence inventories gain an active implementation target with explicit verification rails.

## Impact

- **Files**: Updates `docs/crate-extraction.md`, `docs/crate-extraction/foundational-types.md`, `docs/crate-extraction/policy.ncl`, and OpenSpec/evidence artifacts under `openspec/changes/review-foundational-types-public-api/`.
- **APIs**: No Rust API break; this records the current canonical public API and compatibility re-export policy.
- **Dependencies**: No new workspace dependency; downstream fixture proves the portable default graph excludes Redb/Iroh/runtime shells.
- **Testing**: Downstream fixture test, no-default package checks, aspen-core no-std checker, extraction-readiness checker, `openspec validate review-foundational-types-public-api --strict`, preflight, and `git diff --check`.

## Verification Expectations

- `foundational-types-extraction.classification-records-reusable-surface`: docs and policy must classify the reviewed crates, owner, readiness state, canonical imports, and rejected surfaces.
- `foundational-types-extraction.classification-records-reusable-surface.evidence`: verification must cite `docs/crate-extraction/foundational-types.md`, `docs/crate-extraction.md`, and `docs/crate-extraction/policy.ncl` as the classification record.
- `foundational-types-extraction.classification-records-reusable-surface.ready`: readiness checker output must show `foundational-types` passing as `extraction-ready-in-workspace`, with publishable/repo-split still blocked by license policy.
- `foundational-types-extraction.live-boundary-evidence`: evidence must include the aspen-core no-default boundary checker, downstream fixture metadata, and negative dependency checks for forbidden runtime shells.
- `foundational-types-extraction.live-boundary-evidence.evidence`: `verification.md` must map evidence files to task coverage.
- `foundational-types-extraction.live-boundary-evidence.downstream-fixture`: downstream fixture tests and the forbidden-boundary grep must pass, including the negative-path expectation that root `aspen`, Redb, Iroh, Axum, Hyper, Tokio, and Snix are absent from the portable fixture graph.
