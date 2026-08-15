# Tasks: config-portability-tooling

## Phase 1: Portable path and toolchain model

- [x] [serial] r[molten.project.config_portability.relocatable_paths] Inventory repo-owned config paths in flake inputs, hooks, docs, and validation commands; classify required sibling paths versus user-local overrides.
- [x] [serial] r[molten.project.config_portability.relocatable_paths] Replace hard-coded user home paths with workspace-relative defaults, environment-variable overrides, or checked flake inputs.
- [x] [serial] r[molten.project.config_portability.toolchain_pin] Pin the release/CI Rust toolchain identity and document any separate exploratory local shell behavior.

## Phase 2: Drift and lint checks

- [x] [parallel] r[molten.project.config_portability.git_source_pin_drift] Add a pure Cargo-lock/Nix-source-map comparison core and shell-owned config drift check.
- [x] [parallel] r[molten.project.config_portability.config_lint] Add positive and negative config-lint fixtures for forbidden user paths, floating release toolchains, placeholder release refs, and mismatched private dependency pins.
- [x] [parallel] r[molten.project.config_portability.named_config_constants] Extract or name repeated Nix check constants for VM addresses, attempt/event bounds, timeout values, and evidence profile names as touched by the change.

## Phase 3: Documentation and validation

- [x] [serial] r[molten.project.config_portability.config_lint] Document the portable workspace variables and config lint/readback command.
- [x] [serial] r[molten.project.config_portability.git_source_pin_drift] Run the focused config drift check, `nix build .#checks.$system.contract-export-drift-gate --no-link`, `nix build .#checks.$system.nextest-config --no-link`, formatting if Rust changes, and Cairn validation/gates.
