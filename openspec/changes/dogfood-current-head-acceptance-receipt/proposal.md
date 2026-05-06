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

- **Files**: OpenSpec artifacts under `openspec/changes/dogfood-current-head-acceptance-receipt/`, `docs/operator-receipts.md`, and current-head receipt/log evidence.
- **APIs**: Dogfood receipt/readback behavior now records and surfaces the exact git commit for operator evidence.
- **Dependencies**: No new runtime dependency is required by this evidence closeout.
- **Testing**: `openspec validate dogfood-current-head-acceptance-receipt --strict`, helper verification, `git diff --check`, and the change-specific verification tasks.

## Verification Expectations

- Capture a current-head dogfood run receipt tied to an exact git commit and run id.
- Save local receipt readback/diagnose outputs so operators can inspect evidence without scraping raw logs.
- Redact cluster tickets, remote URLs, credentials, tokens, and secrets before committing evidence.
- Document whether the full loop passed or deliberately gated; if gated, include a reproducible blocker command and triage summary.
- Validate the OpenSpec change with `openspec validate dogfood-current-head-acceptance-receipt --strict`, `scripts/openspec-preflight.sh dogfood-current-head-acceptance-receipt`, and `git diff --check` before archive.
