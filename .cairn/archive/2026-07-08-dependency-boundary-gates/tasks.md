## Tasks

- [x] [serial] r[molten.modularity.boundary_gates.policy] Defined reviewed dependency-boundary rules in `docs/dependency-boundary-policy/valid.ncl` for core, codec, runtime, adapter, CLI, public API, and generated-code exemptions.
- [x] [serial] r[molten.modularity.boundary_gates.validator] Implemented `molten_core::dependency::validate_dependency_boundaries` with deterministic rule id, source file, forbidden target, and remediation guidance diagnostics.
- [x] [parallel] r[molten.modularity.boundary_gates.fixtures] Added positive Nickel policy fixtures and negative fixtures for duplicate rules, invalid layers, and missing allow/deny targets; Rust tests cover allowed imports, forbidden core-to-adapter imports, and reviewed exemptions.
- [x] [serial] r[molten.modularity.boundary_gates.integration] Documented focused boundary validation commands in `docs/modularity-boundaries.md`.
- [x] [serial] r[molten.modularity.boundary_gates.fixtures] Ran dependency-boundary Nickel fixture checks, `cargo test -p molten-core`, `cargo test --lib`, and Cairn validation.
