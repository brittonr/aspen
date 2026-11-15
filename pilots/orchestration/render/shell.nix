# SPDX-License-Identifier: AGPL-3.0-or-later
# Pilot-only shell, consuming the repository's locked public tool providers.
# Does not install an ambient rustup target or change the root hosted Wasm cohort.
let
  lock = builtins.fromJSON (builtins.readFile ../../../flake.lock);
  source = name: builtins.fetchTree lock.nodes.${name}.locked;
  pkgs = import (source "nixpkgs") {
    overlays = [ (import (source "rust-overlay")) ];
  };
  guestToolchain = (pkgs.rust-bin.fromRustupToolchainFile ../../../rust-toolchain.toml).override {
    targets = [ "wasm32-unknown-unknown" ];
  };
in pkgs.mkShell {
  nativeBuildInputs = [
    guestToolchain
    pkgs.wasm-tools
    pkgs.git
    pkgs.pkg-config
    pkgs.clang
    pkgs.mold
    pkgs.python3
    pkgs.b3sum
  ];
}
