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
- **APIs**: No immediate code API change; implementation tasks will decide stable public API or evidence surfaces.
- **Dependencies**: No dependency change in this spec-only slice.
- **Testing**: `openspec validate complete-blob-castore-cache-readiness --strict`, helper verification, `git diff --check`, and the change-specific verification tasks.
