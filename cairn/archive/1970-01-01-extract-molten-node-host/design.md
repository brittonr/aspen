## Context

The published Molten revision `ee3998eca2fc8a1d119407e3d58cc501212a1be3` has one broad root crate, `molten-core`, and `molten-release-policy`. The root crate owns CLI code, release surfaces, node orchestration, capability-rooted state, and local stores.

The node and service trees use many root modules for Preserves schemas, ledgers, retention, transport, identity, jobs, and policy. A direct full-daemon move would make `molten-node-host` depend on `molten` while `molten` depends on `molten-node-host`. Cargo rejects that cycle.

The capability state and local-store modules form the weakest useful dependency leaf. They own explicit filesystem authority and only need the shared error type, `cap-std`, and `cap-fs-ext`.

## Decisions

### Decision: Extract the capability leaf before daemon orchestration

**Choice:** Move `error`, `node_state`, and `local_store` into `molten-node-host`. Keep daemon, identity, transport, service, and workload modules in `molten` as consumers.

**Rationale:** This creates a compiler-checked host boundary without a cycle or semantic ownership transfer. A later compatibility release can introduce narrower ports before moving daemon orchestration.

### Decision: Preserve exact public type identity

**Choice:** Root modules re-export the new crate's modules and types. The shared `MoltenError`, node-state types, and local-store types have one definition.

**Rationale:** Wrappers would change return types and break callers. Re-exports preserve type and method identity.

### Decision: Keep compatibility bridges narrow

**Choice:** Methods that root internals need across the new crate boundary become hidden public bridge methods. They continue to require already-open capabilities and do not accept ambient paths.

**Rationale:** Cargo crate privacy replaces package privacy. The bridges must not create new authority.

### Decision: Reject non-host dependencies structurally

**Choice:** A manifest boundary test admits exactly `molten-core`, `cap-std`, and `cap-fs-ext`. It rejects missing required dependencies, malformed manifests, Clap, release-policy, harness, NixOS, presentation, process, and transport client dependencies.

**Rationale:** The crate owns local capability effects, not operator policy or presentation.

### Decision: Keep canonical behavior byte-stable

**Choice:** Move source without changing error text, constants, path validation, namespace layout, BLAKE3 domains, schemas, or receipt formats. Focused facade tests use both new and old paths.

**Rationale:** This extraction changes ownership only.

## Validation

- Capture the unchanged workspace test baseline.
- Run `molten-node-host` unit and boundary tests.
- Run root facade tests and the full workspace test suite.
- Run Clippy with warnings denied.
- Run repository Octet or source-gate checks without resetting reviewed baselines.
- Run Cairn validation and proposal, design, and tasks gates.
- Run focused and full Nix checks.

## Risks and Stops

- Stop if the move needs a `molten` dependency from `molten-node-host`.
- Stop if a compatibility bridge reopens ambient filesystem authority.
- Stop if public error, node-state, or local-store types become wrappers rather than aliases.
- Stop if stable output, state layout, or path validation changes.
- Keep inherited test or Tracey debt bounded and record it exactly.

## Rollout

1. Publish the leaf extraction behind unchanged root facades.
2. Keep one compatibility release with both paths tested.
3. Design typed daemon ports only after this crate boundary is stable.
4. Move daemon and service orchestration in later bounded slices without moving workload semantics.
