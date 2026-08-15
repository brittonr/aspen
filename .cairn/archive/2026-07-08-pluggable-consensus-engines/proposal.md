## Why

Molten now records consensus algorithm profiles, but the runtime path is still effectively Raft-specific: production startup rejects non-Raft profiles and the control-plane code chooses behavior through hard-coded profile matches. That is safe, but it is not genuinely pluggable or swappable.

The next step is to turn algorithm profiles into an admitted consensus-engine boundary. A control-plane group should resolve its manifest profile through a policy-backed registry, run through an engine interface with common read/propose/snapshot receipts, and require an explicit switchover plan before changing the algorithm behind existing control-plane state.

## What Changes

- Add a consensus engine registry keyed by algorithm profile id and profile version, with fail-closed admission for unknown, disabled, experimental, or evidence-incomplete engines.
- Define a `ControlPlaneConsensusEngine` boundary that separates pure deterministic state/receipt decisions from the imperative shell that owns storage, networking, clocks, and operator I/O.
- Replace Raft-only runtime selection with manifest-driven engine selection while keeping Raft as the only admitted production engine until another engine has accepted implementation, proof/model, simulation, placement, and policy evidence.
- Add explicit engine switchover receipts for changing a group from one consensus engine/profile to another, including state compatibility, fencing epoch, replay/conformance evidence, rollback posture, and denial diagnostics.
- Keep coordination services engine-agnostic by consuming common commit/read/currentness receipts rather than inspecting Raft-specific internals.
- Add deterministic conformance and negative fixtures proving that engines produce the same canonical application state for admitted operations and fail safely for unsupported registry entries or unsafe switchover plans.

## Impact

- **Files**: Adds a native Cairn change package under `cairn/changes/pluggable-consensus-engines/` with deltas for `consensus`, `coordination`, and `testing-harness`.
- **Compatibility**: Existing Raft-backed groups remain the default admitted production path. New engines are opt-in and denied unless explicitly registered, policy-admitted, and evidence-complete.
- **Testing**: Lifecycle validation and gates should pass for this package. Future implementation should run focused consensus, coordination, and deterministic simulation tests plus pre-commit before sync/archive.
