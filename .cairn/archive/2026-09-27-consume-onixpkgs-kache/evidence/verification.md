# Verification: consume kache from onixpkgs

## Baseline

Pre-change checkout: `3258a2614` (clean tree), `onix-core-src` pinned at `ae895854eb049ff152d3f1b96cb90a5fa45c3ec6`.
`onix-core-src` had exactly two consumers in `flake.nix`: the `kachePackage`/`kacheLib` bindings and `checkedKacheLib` in `kache-nix-rust-wrapper-contract`.
Derivation paths for all 89 `x86_64-linux` checks, 10 packages, and the default dev shell were recorded before any edit.

## Library identity

`onixpkgs` `lib/kache-nix-rust.nix` at `56a93169f4ace2394dc4c474c6d28911c24c35e7` is byte-identical to `onix-core` `lib/kache-nix-rust.nix` at `ae895854` (`diff` empty).
The onixpkgs `pkgs/kache/default.nix` differs from the onix-core file only in `version` (0.6.0 → 0.16.0), the source `hash`, and `cargoHash`.

## Shadowing audit

Onixpkgs overlay names: every `pkgs/` directory plus `herdr-unwrapped`.
Names also present in the pinned nixpkgs (`f9d8b65950353691ab56561e7c73d2e1063d810b`): `dumbpipe`, `iroh-ssh`, `sendme`, `sone`.
Word-boundary search of `flake.nix` for every overlay name matched only `kache` (the wrapper bindings) and `tracey` (`tools/tracey` and `evidence/tracey` paths).
No `*.nix` file outside `vendor/` and `target/` references `pkgs.<overlay name>`.

## Lock diff against `HEAD`

- Removed: `onix-core-src`.
- Added: `onixpkgs` (rev `56a93169`, `nixpkgs` follows root `nixpkgs`) and its transitive nodes `llm-agents`, `bun2nix`, `flake-parts`, `systems_14`, `treefmt-nix`, `treefmt-nix_2`, `wrappers`, `horizon`.
- Renamed only: unit2nix's former `flake-parts` node is now `flake-parts_2` with identical content; the `unit2nix` node changed only that input pointer.
- `nix flake metadata` shows no onix-core node.

## Derivation comparison

Every derivation built from the flake's own source changes with any tracked edit, so 50 checks and the `default`, `molten`, `all`, and `verified-node-replication-pilot` packages change before/after.
`nix-diff` of `packages.x86_64-linux.default` shows only `The input source named source differs`.

Source-fixed control: the switched tree evaluated with `--override-input onixpkgs` pointing at a stub flake whose overlay adds only the onix-core `ae895854` kache 0.6.0 package and whose `lib.kacheNixRust` is the onix-core file.

- Checks: 89 of 89 equal between the real and stub evaluations.
- Dev shell: equal, and equal to the pre-change derivation.
- Packages: 8 of 10 equal; only `molten-kache` and `molten-kache-rust` differ.
- The stub `molten-kache-rust` equals the pre-change derivation exactly.
- 39 checks and the `doltlite-oracle`, `flux-profiler-cli`, `molten-node-host`, and `molten-release-policy` packages are unchanged against the pre-change derivations without any control, including `kache-nix-rust-wrapper-contract`, `molten-node-host`, `dev-function-profiling`, and the Octet deny-all and schema-inventory checks.

`nix-diff` of `molten-kache-rust` before/after: the only change is the `molten-kache-rust-rustc` input, whose input derivation set swaps `kache-0.6.0` for `kache-0.16.0` and whose wrapper text differs only in the `PATH` kache store path.
`molten-kache` additionally differs by the flake `source` input and by crate derivations compiled with the wrapped toolchain.

## Builds

- `kache-nix-rust-wrapper-contract` (fake kache, unchanged derivation): already valid; `nix build --rebuild` re-ran it and produced an identical output.
- `kache-0.16.0` (`/nix/store/ziyy9xk02r8qv83fqp3xfjrk490rx58p-kache-0.16.0.drv`): not in the store and not substitutable (the tailnet cache was unreachable; onixpkgs' own and other local kache 0.16.0 outputs use a different nixpkgs), so it built locally from source. `kache --version` prints `kache 0.16.0`.
- `packages.x86_64-linux.molten-kache-rust`: built. Its `bin/rustc` wrapper references only the kache 0.16.0 store path. With `KACHE_NIX_DISABLED=1`, `rustc -vV` reports `rustc 1.98.0-nightly (31a9463c6 2026-05-25)`. With a missing cache directory it prints the documented diagnostic and exits 78 without invoking kache.

## Not run

- `molten-kache` (full workspace built through the kache wrapper) was not built, and kache 0.16.0 was not run against a real cache.
- `nix flake check`, cargo, Octet, and Clippy gates were not run; no Rust source changed.
- No `aarch64-linux` or darwin evaluation.

## Claim boundary

The evidence shows the source switch, lock contents, derivation isolation for `x86_64-linux`, and the wrapper contract. It makes no cache-hit, speed, or reproducibility claim for kache 0.16.0.
