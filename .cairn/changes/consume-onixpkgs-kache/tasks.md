# Consume kache from onixpkgs tasks

See [the verification record](evidence/verification.md) for derivation comparisons, builds, and unrun checks.

- [x] [serial] Record derivation paths for all `x86_64-linux` packages, checks, and dev shells before the switch. Cite r[molten.onixpkgs_kache.isolation].
- [x] [serial] Audit onixpkgs overlay names against `flake.nix` and nixpkgs and record the shadowing result in the design. Cite r[molten.onixpkgs_kache.isolation].
- [x] [serial] Replace the `onix-core-src` input with a rev-pinned `onixpkgs` input following `nixpkgs`, apply its overlay to `pkgsBase`, and relock. Cite r[molten.onixpkgs_kache.source].
- [x] [serial] Take kache as `pkgs.kache` and build both helper call sites through `onixpkgs.lib.kacheNixRust`. Cite r[molten.onixpkgs_kache.source] and r[molten.onixpkgs_kache.contract].
- [x] [serial] Update README override examples and the kache reference entry. Cite r[molten.onixpkgs_kache.source].
- [x] [serial] Compare derivation paths after the switch and `nix-diff` the kache-bearing wrapper. Cite r[molten.onixpkgs_kache.isolation] and r[molten.onixpkgs_kache.contract].
- [x] [serial] Build `pkgs.kache` and the `kache-nix-rust-wrapper-contract` check, then run Cairn validation. Cite r[molten.onixpkgs_kache.contract].
