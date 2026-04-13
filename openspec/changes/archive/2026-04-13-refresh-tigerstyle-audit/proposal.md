## Why

The 2026-04-13 Tiger Style refresh exposed two separate needs:

1. repo-visible audit evidence against the 2026-04-09 baseline
2. an immediate first remediation slice for the highest-value production hotspots

User asked to begin the remediation backlog and carry it through completion. Leaving this change audit-only would preserve the report, but it would not capture the production refactors that actually removed the named hotspots.

## What Changes

- Keep the refreshed Tiger Style audit report and baseline comparison under this change.
- Land the first remediation slice for the backlog named by the report:
  - CI executor and flake-lock monoliths
  - federation handler monoliths
  - request metadata dispatch tables
  - discovery / log-subscriber / alert orchestration helpers
  - trust / secrets rotation helpers
  - highest-priority `usize_api` leaks
  - Tiger Style scanner parse-error and doc-comment false-positive fixes
- Save targeted verification artifacts for both the code changes and the scanner changes.

## Capabilities

### Modified Capabilities

- `tigerstyle-audit-report`: audit evidence now records both the refresh baseline and the completed remediation slice that followed from that audit.
- `tigerstyle-remediation-backlog`: first remediation slice is implemented with repo-visible verification artifacts instead of remaining a backlog-only plan.
- `tigerstyle-audit-tooling`: scanner heuristics now distinguish lifetimes from char literals and ignore comment-only ambient-time mentions.

## Impact

- **Files**: `openspec/changes/refresh-tigerstyle-audit/`, `crates/aspen-ci-executor-nix/`, `crates/aspen-client-api/`, `crates/aspen-cluster/`, `crates/aspen-core-essentials-handler/`, `crates/aspen-forge/`, `crates/aspen-forge-handler/`, `crates/aspen-jobs/`, `crates/aspen-raft/`, `crates/aspen-transport/`, `scripts/tigerstyle-audit.py`, `tools/tigerstyle/test_tigerstyle_audit.py`
- **APIs**: internal helper factoring plus request-metadata lookup table; no user-facing protocol changes intended
- **Dependencies**: none
- **Testing**: targeted Rust tests, targeted cargo checks, scanner regression tests, and targeted Tiger Style scan artifacts saved under `evidence/`
