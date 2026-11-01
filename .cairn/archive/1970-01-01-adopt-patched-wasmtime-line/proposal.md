## Why

Molten pins `wasmtime = 45.0.0` for its hosted component runtime, core-module execution
profile and SpaceWasm pilot. That cohort carries eight unfixed RUSTSEC advisories whose patched
ranges intersect at `>= 49.0.4`: -0269 (>=47.0.4), -0222 (>=47.0.3), -0316 (>=49.0.1),
-0327 (>=49.0.2), -0359/-0362/-0363 (>=49.0.3) and -0379 (>=49.0.4). The workspace operator
chose to move to a patched line instead of recording an advisory exception. The shared
`hardened-wasmtime-runtime` crate pins the engine exactly, so this Molten change consumes that
repository's patched revision rather than moving the engine alone.

## What Changes

- Consume `hardened-wasmtime-runtime`/`hardened-wasmtime-shell` at the published patched
  revision and move Molten's own `wasmtime`/`wasmtime-wasi` pins to `49.0.5`, the lowest
  release inside every advisory's patched range. r[molten.wasm_component_adoption.patched_engine]
- Re-freeze the admitted `molten.wasm.component.v1` and time-sliced profile documents, their
  generated exports and the runtime source-cohort provenance at the same engine cohort.
  r[molten.wasm_component_adoption.patched_engine]
- Update the WAT parity fixtures to the nested core-reference syntax Wasmtime 49 requires, and
  the release-dependency profile's immutable revision and evidence digest.
  r[molten.wasm_component_adoption.patched_engine]

## Impact

- **Files**: `Cargo.toml`/`Cargo.lock`, `crate-hashes.json`, `flake.nix`/`flake.lock`,
  `build-plan.json`, `release-policy-build-plan.json`,
  `docs/wasm-component-runtime/{contracts.ncl,generated/*.json}`,
  `config/release-dependencies/profile.ncl`, `src/wasm/component/{profile.rs,runtime/provenance.rs}`,
  `src/wasm/performance/policy/profile.rs`, `src/spacewasm/evidence.rs`,
  `src/wasm/component/tests/parity/fixtures.rs`.
- **Testing**: workspace tests, the component-runtime and SpaceWasm suites, three Clippy
  configurations, Octet primary and library cohorts, and the profile/release-dependency flake
  checks. This does not claim compiler correctness, production admission or release readiness.
