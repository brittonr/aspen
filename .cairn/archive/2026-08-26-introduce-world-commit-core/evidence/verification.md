# World-commit core verification

Verified on 2026-08-26 in the dedicated Molten worktree.

## Focused behavior

- `nix develop -c cargo test -p molten-core world_commit`: 9 passed.
- `nix develop -c cargo test -p molten world_commit`: 8 passed across the library and command parser.
- `nix develop -c cargo clippy -p molten-core --all-targets --all-features -- -D warnings`: passed.

## Strict Octet

`nix build path:$PWD#checks.x86_64-linux.world-commit-octet-deny-all -L --builders ''` compiled the real `crates/molten-core/src/worldcommit` source and its positive and negative tests under the full reviewed catalog.

- Status: `clean`
- Findings: `0`
- Warnings: `0`
- Errors: `0`
- Config: `b3:3a5720b71d9e24fac98afd68c8ec3a978e08d9e33271534e613e8169cc2908f3`
- Profile: `b3:780346dee4c3210cca3ce84351247cb031d595b5501f512611b2998a83c61b4b`

The focused gate does not convert inherited repository-wide findings into a pass. Shell behavior remains covered by the focused Molten tests, strict Clippy, Cairn gates, and existing Nix checks.

## Lifecycle and claim boundary

Strict Cairn validation and the relevant Nix checks passed before archival. These checks establish the bounded world-commit implementation. They do not prove external RealmCommit compatibility, current authority, release eligibility, complete remote availability, or whole-repository Octet conformance.
