# Proposal: Octet burn-down, collection growth in validators

## Why

`unbounded_collection_growth` is a critical Octet family: it drives the strict gate's `no-critical-findings` failure.
On this base (after `octet-burndown-safety-unwrap`) the pinned Octet run reports 214 findings in 111 distinct sites.
This change covers the 55 sites in the governance and evidence validators. The runtime and adapter sites follow in
`octet-burndown-safety-collections-runtime`. The lint fires when a local `Vec`/`BTreeMap`/`HashMap` starts empty and
grows in a loop with no reservation and no explicit length bound.

## What Changes

In `src/testing/hardening.rs`, `src/harness` (gate p002, schema p021 and p026), `src/provenance`, `src/project`,
`src/peer`, `src/raft/control` (p008, p009), `src/resources` (p003, p004, p005, p007, p008), `src/effects`,
`src/capability`, `src/audit`, `src/cli/ops/dogfood/archive.rs`, `src/lifecycle`, `src/coordination`,
`src/coordination_delivery`, `src/retention`, and `src/chunk`:

- Pure filter/map loops become iterator chains that `collect` or `extend`. Order, duplicates, and first-error
  propagation stay the same.
- Loops that add at most a known number of items per bounded input element reserve exactly that maximum. The inputs are
  already bounded upstream (`ensure_bound` / `MAX_ITEMS` in hardening, the parsed record and manifest bounds elsewhere).
  Per-element multipliers are named constants.
- Two single-shot diagnostics become options. The admission chain stops at its first denial. Release archive
  verification rejects duplicate members, so the archive manifest is seen at most once.

## Impact

- **Files**: the 24 flagged files listed above.
- **Testing**: the pinned Octet root and lib runs; `cargo fmt --check`; `cargo clippy --workspace --all-targets -D
  warnings`; focused module tests, then `cargo test --workspace`.

## Out of Scope

- No new denial bound is added, so accepted behavior and canonical receipts do not change. Every accepted input yields
  the same collections in the same order.
- The runtime and adapter sites, and the other safety families, belong to later slices.
