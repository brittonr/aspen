## Why

Deterministic replay is the central law of the harness. Many focused tests exercise replay, but evidence-bearing suites should all get the same small smoke contract: run, replay, fresh rerun, and compare canonical refs or explain why the run is non-replayable and ineligible for pass evidence.

A replay smoke rail catches ambient-state dependence early and gives reviewers a consistent readback for deterministic claims.

## What Changes

- Define a replay smoke contract for evidence-bearing harness suites.
- Add a reusable runner or test helper that performs run, replay, fresh rerun, and canonical comparison.
- Exclude exploratory or non-replayable suites from deterministic gates with explicit diagnostics.
- Record replay-smoke results in reports and traceability coverage.

## Impact

The suite gets stronger flake prevention with a cheap, repeatable check that applies before expensive VM or dogfood evidence.
