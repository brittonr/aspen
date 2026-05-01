# I13 testing harness surface inventory

Status: reusable testing helper ownership inventory complete.

## Reusable defaults

- `aspen-testing-core`: preferred reusable root for deterministic in-memory helpers, event recording, bounded assertions, retry/wait utilities, and generic mock state. Dependency tree stays lightweight relative to cluster/runtime shells.
- `aspen-testing-fixtures`: reusable fixture/builders surface when used for generic sample data and helper construction.

## Compatibility facades / adapters

- `aspen-testing`: compatibility facade for existing suites; may depend on broader Aspen crates and should not be treated as the default downstream harness root.
- `aspen-testing-madsim`: simulation adapter; concrete madsim/OpenRaft/Iroh/bootstrap behavior remains explicit adapter purpose.
- `aspen-testing-network` and `aspen-testing-patchbay`: network/namespace/transport adapters, not generic reusable defaults.

## Boundary decision

Reusable defaults should start at `aspen-testing-core` and generic portions of `aspen-testing-fixtures`. Root app, node bootstrap, handler registries, concrete cluster runtime, madsim setup, patchbay namespaces, binary shells, and transport adapters remain outside the default harness boundary.

## Evidence captured

`i13-testing-harness-inventory.txt` records checks and dependency trees for core/fixtures/facade/adapters.
