# Consume kache from onixpkgs design

## Source switch

`flake.nix` declares `onixpkgs` as `git+ssh://git@github.com/OnixResearch/onixpkgs.git?rev=56a93169f4ace2394dc4c474c6d28911c24c35e7` with `inputs.nixpkgs.follows = "nixpkgs"`.
The revision stays explicit in the URL, matching the other rev-pinned inputs.
Transitive onixpkgs inputs (`llm-agents`, `wrappers`, `horizon`, `treefmt-nix`) enter the lock; only `horizon` is a non-flake source, and none of them is evaluated unless an overlay attribute that needs it is forced.
Molten forces only `kache`, which depends on nothing outside nixpkgs.

## Overlay placement

`pkgsBase` imports `nixpkgs` with `[ (import rust-overlay) onixpkgs.overlays.default ]`.
`pkgs`, `unit2nixPkgsBase`, and `unit2nixPkgs` derive from `pkgsBase`, so every kache consumer sees the same `pkgs.kache`.
Onixpkgs documents `import nixpkgs { overlays = [ ... ]; }` as the consumer pattern, and `pkgsBase` is already that import.

Shadowing audit: the overlay defines the onixpkgs `pkgs/` directory names plus `herdr-unwrapped`.
Of those, the pinned nixpkgs defines `dumbpipe`, `iroh-ssh`, `sendme`, and `sone`.
`flake.nix` and the repository Nix files reference none of them.
`kache` and `tracey` appear in `flake.nix` only as the kache wrapper bindings and as `tools/tracey` and `evidence/tracey` paths, never as package attributes.
The overlay therefore adds `kache` without shadowing any package Molten builds from nixpkgs.
A control evaluation confirms it: the same working tree evaluated with `--override-input onixpkgs` pointing at a stub flake whose overlay defines only the former onix-core `kache` 0.6.0 package yields the same derivation paths for every non-kache package, check, and dev shell.

## Helper library

`kacheLib` and the contract check's `checkedKacheLib` call `onixpkgs.lib.kacheNixRust { lib; pkgs; kachePackage; }`.
The contract check keeps its fake kache package, so it still exercises only wrapper behavior.
The library source is byte-identical to the former `onix-core` file, so the wrapper scripts do not change apart from the kache store path.

## Behavior change

The rustc wrapper in `molten-kache-rust` and the `molten-kache` workspace now runs kache 0.16.0 instead of 0.6.0.
Wrapper interface, cache directory `/var/cache/kache-nix`, key salt `molten-unit2nix-kache-v1`, missing-cache exit code, and disabled mode are unchanged.
The default `molten` package and the non-kache checks do not use kache and keep their derivations.

## Validation approach

Record derivation paths for all `x86_64-linux` packages, checks, and dev shells before and after the switch.
Every derivation built from the flake's own source changes with any tracked edit, including `flake.nix`, `flake.lock`, and `README.md`, so a raw before/after comparison cannot isolate the overlay.
Hold the source fixed instead: evaluate the switched tree against a stub `onixpkgs` whose overlay only adds the former onix-core kache 0.6.0 and whose `lib.kacheNixRust` is the former file.
Require equality between the real and stub evaluations for everything except the kache-bearing attributes, and require the stub `molten-kache-rust` to equal the pre-switch derivation.
Use `nix-diff` on before/after pairs to show that the remaining differences are the flake `source` input or the kache package.
Use `nix-diff` on `molten-kache-rust` to show that the only input change is the kache package.
Build `pkgs.kache` and the `kache-nix-rust-wrapper-contract` check.
Confirm `flake.lock` drops only `onix-core-src` and adds onixpkgs with its transitive nodes.
