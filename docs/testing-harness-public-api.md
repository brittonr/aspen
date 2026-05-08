# Testing harness public API boundary

The reusable `aspen-testing` crate exposes cheap inventory, manifest, run-report, and diagnostic helpers by default. Heavier adapters remain behind explicit Cargo features so downstream crates can depend on the reusable harness API without pulling runtime-host, patchbay, madsim, runtime-app, forge, CI, jobs, or Raft execution stacks.

## Default API

Default/no-feature consumers may use:

- suite inventory loading, rendering, writing, and freshness checks;
- structured inventory freshness diagnostics (`InventoryCheckReport` and `InventoryCheckDiagnostic`);
- manifest/runtime-class metadata types;
- nextest JUnit run-report parsing.

The command-line harness exposes the same freshness diagnostics:

```bash
cargo run -p aspen-testing --bin aspen-test-harness -- check --json
```

`current: true` means the committed generated inventory matches Nickel manifests. `current: false` includes diagnostics with stable `code`, `severity`, and `message` fields.

## Explicit feature boundary

Use explicit features for heavier adapters:

- `router` / `simulation` for Raft/openraft and madsim-backed testing;
- `network` for VM/network adapter utilities;
- `jobs`, `ci`, and `forge` for runtime-app specific adapters;
- `testing` / `full` only when a broad local test dependency graph is acceptable.

Run the boundary check before changing public API dependencies:

```bash
scripts/test-harness.sh public-api-boundary
```

The check inspects the `aspen-testing --no-default-features` dependency graph and fails if forbidden heavy adapter packages appear in the default public API. It is also included in `scripts/test-harness.sh quick-confidence`.
