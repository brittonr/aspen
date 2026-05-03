## Why

Aspen's README promise is system-level: source is pushed to Aspen Forge, built by Aspen CI, deployed to an Aspen cluster, and verified by `nix run .#dogfood-local`. Today the dogfood binary logs progress, but it does not leave a durable, machine-checkable proof of what happened. Operators and future agents need a receipt they can inspect after both success and failure.

Crunch's release evidence work provides the useful pattern: explicit schema names, bounded artifact records, canonical JSON, linkage between workflow identity and artifacts, and validation before trusting a bundle. Aspen should apply that pattern to dogfood runs without turning the first slice into a full release-attestation system.

## What Changes

- **Dogfood run receipt model**: Introduce structured receipt types for the dogfood pipeline, including run identity, command/config snapshot, stages, artifacts, and failure summaries.
- **Canonical JSON receipt**: Serialize receipts as stable JSON suitable for saving under the dogfood run directory and later hashing or uploading.
- **Stage evidence contract**: Define the minimum stage statuses and identifiers the dogfood orchestrator must eventually record for start, push, build, deploy, verify, and stop.
- **First implementation slice**: Add the model plus serialization/validation tests; later slices will write receipts during live orchestration.

## Capabilities

### New Capabilities

- `dogfood-evidence-receipts`: Dogfood runs produce durable receipts that can be inspected after success or failure.
- `operator-trust-evidence`: Operators can answer which repo, commit, CI run, artifacts, nodes, and failure cause belonged to a dogfood run.

### Modified Capabilities

- `dogfood-orchestration`: The pipeline becomes evidence-producing instead of log-only.

## Impact

- **Files**: `crates/aspen-dogfood/src/receipt.rs`, `crates/aspen-dogfood/src/main.rs`, OpenSpec artifacts under this change.
- **APIs**: Internal dogfood data model first; no client/RPC/wire changes in this slice.
- **Dependencies**: No new crate dependency expected for the first model slice.
- **Testing**: `cargo nextest run -p aspen-dogfood`; `openspec validate dogfood-evidence-receipts --json`; helper verification; `git diff --check`.
