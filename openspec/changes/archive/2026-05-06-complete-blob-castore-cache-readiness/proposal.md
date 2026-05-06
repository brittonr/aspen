## Why

Blob/castore/cache is high-value for self-hosting and cache infrastructure; current manifests say key couplings are moved/gated but readiness still lacks complete fixtures, checker updates, and compatibility evidence.

## What Changes

- **Promote the family only after downstream fixtures and negative policy checks prove reusable defaults**: Promote the family only after downstream fixtures and negative policy checks prove reusable defaults.
- **Capture compatibility evidence for Aspen runtime consumers using explicit adapter paths**: Capture compatibility evidence for Aspen runtime consumers using explicit adapter paths.
- **Update extraction inventory and checker expectations for the family**: Update extraction inventory and checker expectations for the family.

## Capabilities

### New Capabilities
- `blob-castore-cache-extraction`: Complete blob/castore/cache readiness evidence readiness/evidence requirements.

### Modified Capabilities
- Existing extraction and dogfood evidence inventories gain an active implementation target with explicit verification rails.

## Impact

- **Files**: OpenSpec artifacts under `openspec/changes/complete-blob-castore-cache-readiness/`.
- **APIs**: Blob/castore/cache API evidence remains bounded to reusable defaults plus explicit Aspen adapter paths.
- **Dependencies**: The implementation may adjust compatible dependency patch levels only when package and fixture evidence proves the final graph.
- **Testing**: `openspec validate complete-blob-castore-cache-readiness --strict`, helper verification, `git diff --check`, and the change-specific verification tasks.

## Verification Expectations

- Covered IDs: `blob-castore-cache-extraction.promotion-requires-complete-evidence`, `blob-castore-cache-extraction.promotion-requires-complete-evidence.evidence`, `blob-castore-cache-extraction.adapter-paths-explicit`, `blob-castore-cache-extraction.adapter-paths-explicit.evidence`.
- Capture downstream fixture checks for `aspen-blob` and `aspen-cache`/`aspen-castore` consumers under the change-local `evidence/` directory, including negative-path expectations for forbidden app-shell dependency leakage.
- Capture package checks for `aspen-blob`, `aspen-cache`, and `aspen-castore`, including the castore circuit-breaker regression.
- Regenerate readiness checker Markdown/JSON for the `blob-castore-cache` candidate family after policy/doc updates.
- Run strict OpenSpec validation, repo-local preflight, no-std boundary verification for any lockfile dependency adjustments, and whitespace checks before archive.
