# Config/Plugin Extraction Manifest

## Candidate

- Family: `config-plugin`
- Candidates: `aspen-nickel`, `aspen-plugin-api`
- Canonical class: protocol/wire plus config helper library
- Owner: Aspen config/plugin maintainers
- Audience: downstream Rust services that want Aspen's typed Nickel config schema helpers or plugin manifest/protocol types without the Aspen node runtime, CLI, Hyperlight host, or job/plugin execution stack.
- Readiness: `workspace-internal`

## Package and release metadata

- Packages stay workspace-internal until standalone examples and dependency-boundary evidence exist.
- License follows workspace AGPL-3.0-or-later policy.
- Repository/homepage metadata remains the Aspen monorepo until human publication policy exists.
- Semver policy: plugin manifest fields, dependency resolution behavior, permission schema, and Nickel schema entrypoints are compatibility-sensitive and need golden tests before promotion.

## Feature contract

### `aspen-nickel`

- `default`: Nickel evaluation/schema support for Aspen-authored configuration contracts.
- Current default graph includes `aspen-core`; readiness work must either justify that as a config type contract or split reusable schema helpers from app/runtime config types.
- Standalone example target: load/typecheck `crates/aspen-nickel/src/schema/node_config.ncl` and deserialize a minimal config fixture without requiring node runtime startup.

### `aspen-plugin-api`

- `default`: plugin manifest, permissions, dependency, protocol, and install validation types backed by `serde`, `serde_json`, and `semver`.
- No runtime host, Hyperlight, VM executor, CLI, handler, root app, or concrete transport dependency is allowed in the default graph.
- Standalone example target: parse a plugin manifest, validate dependency/protocol constraints, and serialize the public manifest schema.

## Dependencies

### `aspen-nickel`

- Default Aspen dependency: currently `aspen-core` for config types; this is the primary readiness question and must be narrowed or explicitly justified before promotion.
- Default external dependencies: `nickel-lang`, `serde`, `serde_json`, `snafu`, `thiserror`, and `tracing`.
- Forbidden by default for a reusable config helper: root `aspen`, node binaries, handler crates, CI/job runtime shells, concrete transport, Hyperlight/VM plugin host crates, and CLI/TUI shells.

### `aspen-plugin-api`

- Default external dependencies: `serde`, `serde_json`, and `semver`.
- Default Aspen dependencies: none.
- Forbidden by default: root `aspen`, `aspen-cli`, plugin host/runtime crates, handler crates, Hyperlight/VM executors, concrete transport, cluster bootstrap, and job execution stacks.

## Compatibility and aliases

- `aspen-cli` consumes `aspen-plugin-api` for plugin manifest commands and dependency validation.
- Runtime plugin host and worker crates may consume `aspen-plugin-api`, but they are final consumers rather than reusable defaults.
- `schemas/typed-nickel-contract-registry.ncl` records the node config schema as a Nickel-authored contract owned by Aspen cluster config maintainers.
- Compatibility re-exports: none planned; consumers should import the leaf crates directly.

## Representative consumers

- `aspen-cli` plugin commands for `aspen-plugin-api` manifest parsing and dependency validation.
- Runtime plugin host/worker crates when enabled, as compatibility consumers only.
- Typed Nickel contract checks: `nix run nixpkgs#nickel -- typecheck crates/aspen-nickel/src/schema/node_config.ncl` and `python3 scripts/check-typed-nickel-contract-fixtures.py`.
- Future downstream fixtures under the active OpenSpec change for standalone config and plugin API examples.

## Dependency exceptions

- `aspen-nickel -> aspen-core`: temporary workspace-internal dependency for config types; must be narrowed, split, or justified with explicit evidence before readiness can advance.
- `aspen-nickel -> nickel-lang`: owned config-language dependency and core purpose of the crate.
- `aspen-plugin-api -> semver`: owned manifest/dependency compatibility dependency.

## Verification rails

- positive downstream: compile standalone examples for Nickel schema/typecheck use and plugin manifest/dependency validation.
- negative boundary: grep fixture metadata for root Aspen, CLI/TUI, handler crates, Hyperlight/VM host crates, concrete transport, cluster bootstrap, and job/runtime shells.
- compatibility: compile `aspen-cli` plugin command paths and runtime plugin host consumers through explicit feature bundles.
- dependency-boundary: run `cargo tree -p aspen-nickel -e normal` and `cargo tree -p aspen-plugin-api -e normal`; default `aspen-plugin-api` must have no Aspen workspace dependencies.
- typed-contract: run the Nickel schema typecheck and typed contract fixture checker.

## Blocked reasons and next action

- Readiness remains `workspace-internal` because standalone downstream examples and negative boundary evidence are not yet captured.
- `aspen-nickel` must resolve the current `aspen-core` default dependency before promotion.
- Publication/repository split remains blocked on human license/publication policy even after technical rails pass.
- Next action: add standalone examples and checker policy support for the `config-plugin` family.
