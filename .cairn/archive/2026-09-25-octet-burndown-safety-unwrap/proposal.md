# Proposal: Octet burn-down, safety `no_unwrap`

## Why

The pinned Octet run on this base (`d825dc14b`, on `origin/molten` `4e31cee55`) reports `no_unwrap` 80 in the workspace
scope (70 distinct sites) and 3 in the `-p molten --lib` scope. `no_unwrap` is a critical family, so it drives the strict
gate's `no-critical-findings` failure. 67 of the 70 sites are `.expect()` in integration-test binaries, where Octet does
not treat an integration `#[test]` function as test context; 3 are in library source. They must reach zero by repair,
without allows, baselines, or catalog changes.

## What Changes

- Integration tests (`tests/content_replication.rs`, `tests/nativesystemextension.rs`,
  `tests/nativesystemextension/support.rs`, `tests/parts/cliharness/p011`, `p016`): make each test and helper return a
  `Result` with a boxed error, and replace every `.expect()`/`.unwrap()` with `?` through a labelled `OrFail` step helper. The one
  `panic!` in `tests/content_replication.rs` becomes a returned error in the same step, because it shares the helper.
- `src/cluster_harness/fabric_transport.rs`: the fixture transport profile helper propagates its admission error.
- `src/cluster_harness/runner.rs`: the lifecycle summary builder returns the missing-config error instead of
  `expect("complete config")`.
- `src/wasm/performance/comparison.rs`: `integer_sqrt` halves the search range with a shift, which needs no fallible
  division.

## Impact

- **Files**: the five integration-test files and three library files above.
- **Testing**: pinned Octet root and lib runs; `cargo fmt --check`; `cargo clippy --workspace --all-targets -D
  warnings`; `cargo test --workspace`; the flake checks that build the touched targets.

## Out of Scope

- Accepted specifications do not change. A failing test still fails, now through a returned `Err` rather than a panic.
  The library repairs return the same errors on the same inputs, and `integer_sqrt` returns the same value for every
  input.
- The other safety families belong to the next two slices (C4b collections, C4c the rest).
