# Proposal: Pin the Octet toolchain in the dev shell

## Why

The strict Octet source-gate sequence in README runs `cargo octet`, but `devShells.default` does not provide it.
Evidence has come from whatever `cargo-octet` is on the operator's `PATH` (here a home-manager build,
`/nix/store/x4a5p5vn…`), not from the catalogued `octet-toolchain` flake input at `fc38f59330b626961d166febfdf1a5aa6575460f`.
The two builds report different finding sets for the same tree. The pinned tool reports 3627 findings, including two
more `underscore_in_module_filename` sites through molten-core paths. So Octet evidence is not reproducible from the
repository.

README:741 also claims that configured Octet evidence is clean (0 findings, passing receipt
`blake3:0179853a…`). With the pinned tool at `origin/molten` `4e31cee55`, the workspace run is `warning-only` with 3627
findings, and the strict gate denies with 312 unreviewed critical findings.

## What Changes

- Add `octet-toolchain.packages.${system}.cargo-octet` to `devShells.default`. `cargo octet` in the dev shell then
  resolves to `/nix/store/n8bkxc9p155a27xnx0iqm3l694s8ckkp-cargo-octet-0.1.0`.
- Rewrite README:741 to the measured state: pinned-tool counts per lint, the lib count, the gate deny and its failing
  checks, the burn-down series, deferred enforcement, and the expired quarantine example dates. It makes no clean
  claim.

## Impact

- **Files**: `flake.nix` (dev shell only), `README.md`.
- **Testing**: `nix develop -c cargo octet --version` resolves to the pinned store path; the full strict sequence with
  the dev shell tool; `nix flake check` evaluation of the dev shell.

## Out of Scope

- Accepted specifications do not change. The change selects the already catalogued tool pin for local runs and
  corrects documentation. Hook or CI enforcement that denies `warning-only` is deferred to the end of the burn-down,
  by owner decision, so commits are not blocked meanwhile. No lint is disabled, and no baseline or quarantine is added.
