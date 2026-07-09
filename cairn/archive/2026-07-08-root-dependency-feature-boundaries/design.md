## Context

Molten needs many dependencies, but not every layer needs all of them. The current root package makes dependency purpose hard to review and increases accidental coupling risk.

## Design

### Dependency classes

- `core`: pure data, bounded helpers, ids, validation.
- `codec`: Preserves canonicalization and BLAKE3 refs.
- `policy-evidence`: authority, capability, provenance, Cairn/Octet/Valence/Trellis evidence summaries.
- `runtime`: dataspace, vat, node planning, service state machines.
- `adapters`: Iroh, Redb, Wasmtime, Steel execution, filesystem/network shells.
- `cli`: Clap and user-facing command shells.
- `test-integration`: dogfood, NixOS VM, soak, harness fixtures.

### Migration path

Start by documenting classes and adding a minimal feature or workspace member that compiles core/codec without adapter dependencies. Keep the existing default feature set for developer compatibility.

### Validation

Add a focused check that builds the minimal core surface and fails if adapter crates leak into it. Later changes can wire this into dependency-boundary gates.

## Non-goals

- Do not remove dependencies required by current default workflows.
- Do not break `molten` CLI default builds.
- Do not claim minimal builds cover adapter behavior.
