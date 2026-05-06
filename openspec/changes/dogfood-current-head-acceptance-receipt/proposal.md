## Why

Aspen needs an operator-grade receipt proving the current pushed head still satisfies the self-hosting loop instead of relying on historical dogfood evidence from older commits.

## What Changes

- **Run or gate the full dogfood acceptance loop on current `main`**: Run or gate the full dogfood acceptance loop on current `main`.
- **Capture local and cluster-published receipts with schema, commit, run id, timings, artifact IDs, and redacted diagnostics**: Capture local and cluster-published receipts with schema, commit, run id, timings, artifact IDs, and redacted diagnostics.
- **Document diagnose/show/readback commands and failure triage**: Document diagnose/show/readback commands and failure triage.

## Capabilities

### New Capabilities
- `dogfood-evidence`: Capture current-head dogfood acceptance receipt readiness/evidence requirements.

### Modified Capabilities
- Existing extraction and dogfood evidence inventories gain an active implementation target with explicit verification rails.

## Impact

- **Files**: OpenSpec artifacts under `openspec/changes/dogfood-current-head-acceptance-receipt/`.
- **APIs**: No immediate code API change; implementation tasks will decide stable public API or evidence surfaces.
- **Dependencies**: No dependency change in this spec-only slice.
- **Testing**: `openspec validate dogfood-current-head-acceptance-receipt --strict`, helper verification, `git diff --check`, and the change-specific verification tasks.
