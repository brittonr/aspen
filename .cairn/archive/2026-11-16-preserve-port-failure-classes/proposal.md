## Why

Issue #58: `From<FabricPortError> for Failure` converted every port failure class, including timeout, cancellation, transport failure and uncertain external outcome, into `InvalidHarness` text. Any `?` on a port result lost the class, and the node lifecycle shell recognised uncertainty by matching a literal message.

## What Changes

- `Failure` gains `Classified { class, message }` with a `FailureClass` enum. Port errors convert to it and back without losing the class, and `Display` names the class instead of "invalid harness artifact".
- Native lifecycle uncertainty is raised as an uncertain-external-outcome failure; the node control shell selects its public detail by class, so an `InvalidHarness` with the same text no longer matches.
- The collaboration-scope denial-code parser follows the new display, and the harness report maps classified failures to stable kind codes.
- AGENTS.md describes the class-preserving conversion and the single Snafu derive.

## Impact

- **Files**: `crates/molten-node-host/src/error/mod.rs`, `src/fabric/port.rs`, `src/cli/ops/node/lifecycle/control.rs`, `src/node/servicehosting/groups/lifecycle.rs`, `src/harness/parts/schema/p030/body.rs`, `src/cli/test/harness.rs`, `src/cli/ops/parts/traceability/p003/body.rs`, `AGENTS.md`.
- **Testing**: round-trip conversion of every port class, classified display, lifecycle control selection by class, and the existing harness and collaboration-scope suites.
- **Owner / consumer**: `molten-node-host` owns the shared failure type; fabric ports, the node lifecycle shell and harness reports consume it.
