## Why

Federation clone (`git clone aspen://ticket/fed:origin:repo`) truncates the object graph for large repos. The `export_commit_dag` BFS silently skips objects it can't read (logging a warning and `continue`), orphaning entire subtrees. For the Aspen repo (~34K objects), the mirror exports only ~7.5K objects — git sees missing commits and fails. The root cause is two-fold: (1) the convergent import loop may leave some objects unimported due to SHA-1 mapping gaps, and (2) the BFS export silently swallows errors instead of propagating them, making failures invisible.

## What Changes

- Make `export_commit_dag_collect` propagate object-read failures instead of silently continuing, so callers get actionable errors
- Add a post-import DAG integrity check in `federation_import_objects` that walks the tree from each ref head and verifies all objects are reachable
- Write a gpgsig commit round-trip test (claimed in napkin but never written)
- Add an end-to-end federation-style import→export test with a multi-level DAG (~100 objects) that asserts export count == import count

## Capabilities

### New Capabilities

- `federation-clone-integrity`: DAG completeness verification for federation import/export

### Modified Capabilities

## Impact

- `crates/aspen-forge/src/git/bridge/exporter.rs` — error handling in `export_commit_dag_collect`
- `crates/aspen-forge-handler/src/handler/handlers/federation.rs` — post-import integrity check
- `crates/aspen-forge/src/git/bridge/tests.rs` — new tests
