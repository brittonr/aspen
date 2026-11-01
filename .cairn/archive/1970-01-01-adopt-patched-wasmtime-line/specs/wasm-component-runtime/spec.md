## ADDED Requirements

### Requirement: The admitted component engine is a patched cohort

r[molten.wasm_component_adoption.patched_engine] Molten MUST admit exactly one Wasmtime and Wasmtime-WASI cohort whose version lies inside every patched range of the RUSTSEC advisories that affected the previous cohort, MUST consume the shared hardened-runtime crate at a published revision pinning that same cohort, and MUST reject any other toolchain at profile admission. Generated profile exports, the runtime source-cohort record and the release-dependency profile MUST name the same cohort.

#### Scenario: Admitted profiles name the patched cohort

- GIVEN the generated `molten.wasm.component.v1` and time-sliced profile exports
- WHEN component profile admission runs
- THEN it admits Wasmtime and Wasmtime-WASI 49.0.5 and the runtime source cohort names the shared crate revision that pins them.

#### Scenario: Any other cohort is refused

- GIVEN a profile document naming a different Wasmtime or Wasmtime-WASI version
- WHEN component profile admission runs
- THEN it denies with the toolchain-drift blockers and no engine is constructed.
