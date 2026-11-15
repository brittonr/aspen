# SPDX-License-Identifier: AGPL-3.0-or-later
# Build from the declared repository source closure, not a pilot-directory copy.
let
  lock = builtins.fromJSON (builtins.readFile ../../flake.lock);
  provider = name: builtins.fetchTree lock.nodes.${name}.locked;
  pkgs = import (provider "nixpkgs") { overlays = [ (import (provider "rust-overlay")) ]; };
  toolchain = pkgs.rust-bin.fromRustupToolchainFile ../../rust-toolchain.toml;
  platform = pkgs.makeRustPlatform { cargo = toolchain; rustc = toolchain; };
  root = ../..;
  src = builtins.path {
    path = root;
    name = "source";
    filter = path: type:
      let rel = pkgs.lib.removePrefix (toString root + "/") (toString path);
      in toString path == toString root || rel == "pilots" || rel == "pilots/orchestration"
        || (pkgs.lib.hasPrefix "pilots/orchestration/" rel
          && !(pkgs.lib.hasInfix "/target/" (rel + "/"))
          && !(pkgs.lib.hasInfix "/.git/" (rel + "/")))
        || builtins.elem rel [ "LICENSE" "crates" "crates/molten-core" "crates/molten-core/src" ]
        || pkgs.lib.hasPrefix "crates/molten-core/src/committed_stream" rel;
  };
in platform.buildRustPackage {
  pname = "molten-standalone-batch-pilot";
  version = "0.2.0";
  inherit src;
  sourceRoot = "source/pilots/orchestration";
  cargoLock.lockFile = ./Cargo.lock;
  nativeBuildInputs = [ pkgs.pkg-config pkgs.clang pkgs.mold ];
  cargoBuildFlags = [ "--workspace" ];
  cargoTestFlags = [ "--workspace" ];
  postInstall = ''
    install -Dm644 ../../LICENSE "$out/share/licenses/aspen-batch-pilot/LICENSE"
  '';
  meta.description = "Aspen standalone bounded batch orchestration and original CPU renderer";
  meta.license = pkgs.lib.licenses.agpl3Plus;
}
