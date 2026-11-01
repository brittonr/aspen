# Tasks: adopt the patched Wasmtime line

## Implementation

- [x] [serial] Consume the shared hardened-runtime crate's published patched revision and move Molten's engine pins to the same exact cohort. r[molten.wasm_component_adoption.patched_engine]
      `hardened-wasmtime-runtime`/`shell` at `57c834dfd0da2f092e2cbbd0e48bb60b61e2d06e`;
      `wasmtime = "49.0.5"`, `wasmtime-wasi = "49"`.
- [x] [serial] Re-freeze the admitted profile documents, generated exports, runtime source cohort and release-dependency profile at that cohort. r[molten.wasm_component_adoption.patched_engine]
      `docs/wasm-component-runtime/contracts.ncl` and its three generated exports,
      `src/wasm/component/profile.rs`, `src/wasm/component/runtime/provenance.rs`,
      `src/wasm/performance/policy/profile.rs`, `src/spacewasm/evidence.rs`,
      `config/release-dependencies/profile.ncl`.
- [x] [serial] Update the WAT parity fixtures to the syntax the patched engine requires. r[molten.wasm_component_adoption.patched_engine]
      `src/wasm/component/tests/parity/fixtures.rs`: `(memory (core memory $i "memory"))`,
      `(realloc (core func $i "realloc"))`, `(post-return (core func $i "post"))`.
- [x] [serial] Run the pin procedure for the moved source: crate hashes, flake input, and both unit2nix plans. r[molten.wasm_component_adoption.patched_engine]
      `scripts/git-source-hashes.sh --write`, `nix flake lock`, `unit2nix --workspace` and
      `unit2nix -p molten-release-policy` regenerated.

## Verification

- [x] [serial] Run the workspace and feature suites, three Clippy configurations, Octet and the affected flake checks. r[molten.wasm_component_adoption.patched_engine]
      Workspace 2922 passed; component 99 passed; Clippy default/spacewasm-pilot/component-rpc-telemetry exit 0;
      Octet primary and lib `Status: clean`; wasm-component-profile, release-dependency-profile and
      release-profile-validation built. Evidence: `evidence/2026-10-31-wasmtime-49.md`.
- [x] [serial] Record the advisory ranges, published shared revision and the pre-existing gate failures before publication. r[molten.wasm_component_adoption.patched_engine]
      Same evidence file, "Pre-existing failures".
