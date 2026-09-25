# Proposal: Octet burn-down, import hygiene

## Why

The pinned Octet run at `origin/molten` (`4e31cee55`) reports `non_trait_imports` 34 and `explicit_defaults` 31 in the
workspace scope (23 and 17 distinct sites), and 9 and 13 in the `-p molten --lib` scope. These are the smallest
families in the burn-down series, which follows the archived `octet-burndown-*` packages. They must reach zero by
repair, without allows, baselines, or catalog changes.

## What Changes

- `non_trait_imports`: in dry run, `scripts/octet-qualify-imports.rs` refuses all 8 flagged files (public
  re-exports, a descendant re-import, a trait-only import, a shared use group, a part-body scope, and `#[path]`
  modules). Apply the same repair by hand: remove each flagged private import and write the owner path at each use.
  Replace the two `pub(crate) use command::*Command` re-exports with a `pub(crate) mod command` path. Call
  `clap::Parser` through `<super::Cli as clap::Parser>::try_parse_from`.
- `explicit_defaults`: replace `T::default()` on other-crate ADTs with explicit constructors in the owning module
  (`TransportState::new`, `TransportCounters::new`, `ResourceUsage::zero`, `SimulationSchedulerState::initial`,
  `TranscriptParseInput::empty`, `InMemoryNativeCallbackValuePort::empty`, `InMemoryNativeHostJournal::empty`), and
  make each `Default` impl delegate to that constructor. Replace `Default::default()` fields with
  `BTreeMap::new()`. Replace bare `#[serde(default)]` with named default functions that return the same values.

## Impact

- **Files**: the 18 flagged files plus the owning modules in `crates/molten-core` and `src/` that gain constructors.
- **Testing**: pinned Octet root and lib runs; `cargo fmt --check`; `cargo clippy --workspace --all-targets -D
  warnings`; `cargo test --workspace`; the flake checks that scan source (including `native-system-extension-host-profile`).

## Out of Scope

- Accepted specifications do not change. Every constructor produces the value the derived default produced, and
  every serde default function returns the previous default, so behavior and wire compatibility are unchanged.
- The other lint families belong to later burn-down slices. Enforcement is deferred until the series reaches zero.
