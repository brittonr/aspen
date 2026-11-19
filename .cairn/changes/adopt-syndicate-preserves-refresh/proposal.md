## Why

`tracing-subscriber@0.2.25` (RUSTSEC-2025-0055) reaches the default closure
through `syndicate 0.44.0-rc.3`. `syndicate 0.44.0-rc.9` already requires
`tracing-subscriber ^0.3.20`, so this advisory is pure dependency work — but the
bump also needs `preserves 5.0.0-rc.10`, which the stack's own Choregraph pin
held at `=5.0.0-rc.7`.

## What Changes

- Take `syndicate 0.44.0-rc.9` and `preserves 5.0.0-rc.10` in Molten.
- Adopt Choregraph `318d4932`, whose only change is the `preserves` bump plus the
  two Kamacite receipts the newer encoder re-emits.
- Repin everything that carries that revision: root and `molten-core` manifests,
  the `choregraph-src` flake input, the release-dependency profile, the four
  `checks/world-*-octet` workspaces and their locks, the plans, the source
  hashes, `THIRD_PARTY_LICENSES.md` and the four world documents that quote it.
- Re-run the pinned audit and record the before/after.

Non-goals: no source change in Molten, no advisory exception, no new capability.
`rsa` (secretspec) and `imbl-sized-chunks` (pinned Nickel) stay recorded as
blocked upstream.

## Impact

- **Files**: two manifests, `Cargo.lock`, `flake.nix`/`flake.lock`,
  `config/release-dependencies/profile.ncl`, `crate-hashes.json`, both plans,
  four focused workspaces, `THIRD_PARTY_LICENSES.md`, four docs.
- **Testing**: `cargo check --workspace --all-targets`, `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, the pinned audit
  before and after, strict Cairn validation and the proposal, design and tasks
  gates.
