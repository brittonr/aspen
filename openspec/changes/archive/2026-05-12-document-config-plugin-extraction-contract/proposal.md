## Why

The broader inventory still lists Config/plugin APIs as `owner needed`, `manifest not yet created`, and `workspace-internal`. This creates recurring ambiguity around `aspen-nickel` and `aspen-plugin-api` reuse. A spec-first slice should define the manifest, owner, feature minima, and standalone examples before any readiness promotion.

## What Changes

- Create a config/plugin extraction manifest covering `aspen-nickel` and `aspen-plugin-api`.
- Record the stable reusable contracts, feature minima, representative consumers, and forbidden runtime dependencies.
- Add policy/inventory rows and checker expectations without raising readiness until evidence exists.

## Capabilities

### New Capabilities
- `config-plugin-extraction`: Documented extraction contract and verification plan for Aspen Nickel config and plugin API crates.

### Modified Capabilities
- `architecture-modularity`: Broader inventory no longer has an ownerless manifest gap for config/plugin APIs.

## Impact

- **Files**: `docs/crate-extraction/config-plugin.md`, `docs/crate-extraction.md`, `docs/crate-extraction/policy.ncl`, OpenSpec artifacts/evidence.
- **APIs**: Spec/documentation first; no code API change required in this slice.
- **Dependencies**: Future evidence must prove reusable defaults avoid app/runtime shells.
- **Testing**: manifest checker, example compile checks, readiness checker negative/positive evidence when implemented.
