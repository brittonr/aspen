## Why

Review findings show verification indexes can become stale: artifacts are referenced before they exist, generated before final edits, or still point to active paths after archive. Freshness must be checked deterministically.

## What Changes

- Add verification-index freshness checks for generated-after-final-diff, no stale active paths, and no placeholder preflight artifacts.
- Require final preflight artifacts to be captured after staging the files they validate.
- Preserve saved diff artifacts while excluding them from stale-path checks.

## Capabilities

### New Capabilities
- `openspec-governance.verification-index-freshness`: Verification indexes and artifacts reflect the final reviewed state.

## Impact

- **Files**: preflight/archive helper, verification template, fixtures.
- **Testing**: Positive fresh index; negative stale path, placeholder preflight, and pre-diff artifact fixtures.
