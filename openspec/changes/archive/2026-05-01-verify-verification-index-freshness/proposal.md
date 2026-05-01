## Why

Review findings show verification indexes can become stale: artifacts are referenced before they exist, generated before final edits, or still point to active paths after archive. Freshness must be checked deterministically.

## What Changes

- Add verification-index freshness checks for generated-after-final-diff, no stale active paths, and no placeholder preflight artifacts.
- Require final preflight artifacts to be captured after staging the files they validate.
- Preserve saved diff artifacts while excluding them from stale-path checks.

## Capabilities

### New Capabilities
- `openspec-governance.verification-index-freshness`: Verification indexes and artifacts reflect the final reviewed state.

## Verification Expectations

- `openspec-governance.verification-index-freshness`: positive freshness fixture covers final verification indexes with current paths and non-placeholder transcripts.
- `openspec-governance.verification-index-freshness.fresh-index-passes`: fresh active/archived index fixture must pass.
- `openspec-governance.verification-index-freshness.stale-active-path-fails`: archived `verification.md` or `tasks.md` citing `openspec/changes/<name>/` must fail.
- `openspec-governance.verification-index-freshness.saved-diff-may-contain-historical-paths`: stale active paths inside saved diff artifacts must remain allowed as historical context.
- `openspec-governance.verification-index-freshness.placeholder-preflight-fails`: cited preflight artifact with placeholder text must fail.
- `openspec-governance.verification-index-freshness.generated-before-final-diff-fails`: final evidence older than changed files must fail and require regeneration.

## Impact

- **Files**: preflight/archive helper, verification template, fixtures.
- **Testing**: Positive fresh index; negative stale path, placeholder preflight, and pre-diff artifact fixtures.
