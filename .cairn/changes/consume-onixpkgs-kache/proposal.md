# Consume kache from onixpkgs

## Why

The opt-in kache Nix Rust path pins the whole `onix-core` repository as the non-flake input `onix-core-src` at revision `ae895854eb049ff152d3f1b96cb90a5fa45c3ec6`.
Molten uses only two files from it: `pkgs/kache` for the kache package and `lib/kache-nix-rust.nix` for the rustc wrapper helpers.
The same package and helper library now live in onixpkgs, the Onix package overlay flake.
Keeping a second copy through `onix-core-src` duplicates package ownership across the Onix stack.

## Proposed change

- Replace the `onix-core-src` input with an `onixpkgs` flake input pinned to `56a93169f4ace2394dc4c474c6d28911c24c35e7` whose `nixpkgs` follows Molten's `nixpkgs`.
- Apply `onixpkgs.overlays.default` where Molten instantiates its base package set and take the kache package as `pkgs.kache`.
- Build the wrapper helpers for `molten-kache-rust`, `molten-kache`, and the `kache-nix-rust-wrapper-contract` check through `onixpkgs.lib.kacheNixRust`.
- Remove the `onix-core-src` input, its outputs argument, and its lock node.
- Point README override examples and the reference entry at onixpkgs.

## Ownership and durable capability

Onixpkgs owns the kache package and the kache Nix Rust helper library.
Molten owns the opt-in wrapper configuration: cache directory, key salt, and wrapped toolchain name.
The durable capability is one pinned source for kache across Onix consumers.

## Evidence and non-claims

The kache package moves from 0.6.0 to onixpkgs' 0.16.0. This is a deliberate behavior change of the opt-in wrapper.
The helper library at the pinned onixpkgs revision is byte-identical to `lib/kache-nix-rust.nix` at `onix-core` revision `ae895854`.
Derivations that do not use kache must keep their derivation paths.
This change makes no cache-hit, speed, or reproducibility claim for kache 0.16.0.
