# Tasks: consensus-engine-trait-decomposition

## Phase 1: Trait split

- [x] [serial] r[molten.consensus_engine_traits.capability_traits] Split descriptor/readback, proposal, read, snapshot, and recovery behavior into explicit consensus engine capability traits.
- [x] [parallel] r[molten.consensus_engine_traits.unsupported_denials] Add explicit denial paths and tests for unsupported engine capabilities.

## Phase 2: Pure transition core

- [x] [serial] r[molten.consensus_engine_traits.pure_transition_core] Extract a pure proposal transition core from the mutable runtime shell.
- [x] [parallel] r[molten.consensus_engine_traits.hash_stability] Add before/after canonical ref tests for representative proposal, duplicate, read, snapshot, and recovery fixtures.

## Phase 3: Conformance and validation

- [x] [parallel] r[molten.consensus_engine_traits.conformance] Extend consensus conformance receipts or tests to assert declared capabilities match implemented trait support.
- [x] [serial] r[molten.consensus_engine_traits.pure_transition_core] r[molten.consensus_engine_traits.capability_traits] Run focused consensus tests, property tests, and `nix run path:$PWD#cairn -- validate --root .`.
