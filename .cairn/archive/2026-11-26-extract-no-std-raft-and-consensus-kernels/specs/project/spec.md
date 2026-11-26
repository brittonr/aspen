# Project deltas

## ADDED Requirements

### Requirement: Tier 1 zero-Preserves pure cores extract into encapsulated no_std workspace crates
r[molten.project.tier1_no_std_kernel_crates] `molten-raft-core`, `molten-wasm-policy-core`, `molten-reliability-model-core`, `molten-fastpath-core` (if retained), `molten-composition-core`, and `molten-runtime-admission-core` MUST compile as `#![no_std]` + `alloc` workspace crates using standard Rust module encapsulation (no `parts/**/body.rs` `include!()` shards) and MUST be gated by hermetic `no_std` compilation, test, and Octet checks in `flake.nix`.

#### Scenario: Every Tier 1 kernel compiles cleanly under no_std
- GIVEN all extracted Tier 1 kernel crates
- WHEN the hermetic Nix check compiles them with `#![no_std]`
- THEN every crate MUST compile and pass unit tests with zero `std` imports in non-test code.
