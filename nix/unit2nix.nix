# Per-crate Nix builds via unit2nix.
#
# Replaces crane's monolithic workspace builds with individual buildRustCrate
# derivations. Changing one crate rebuilds only its reverse dependencies
# instead of all 457 crates.
#
# Build plans are generated outside Nix by running:
#   unit2nix --manifest-path Cargo.toml --features X --bin Y -o nix/build-plan-Y.json
#
# Usage from flake.nix:
#   let u2n = import ./nix/unit2nix.nix { inherit pkgs; src = ./.; };
#   in {
#     packages.aspen-node = u2n.node;
#     packages.git-remote-aspen = u2n.git-remote-aspen;
#   }
{
  pkgs,
  lib ? pkgs.lib,
  src,
  # unit2nix Nix library (path to build-from-unit-graph.nix)
  unit2nixLib ?
    builtins.fetchGit {
      url = "https://github.com/brittonr/unit2nix";
      ref = "main";
    },
  defaultCrateOverrides ? pkgs.defaultCrateOverrides,
}: let
  buildFromUnitGraph = resolvedJson:
    import "${unit2nixLib}/lib/build-from-unit-graph.nix" {
      inherit
        pkgs
        src
        resolvedJson
        defaultCrateOverrides
        ;
    };

  # aspen-node: jobs,docs,blob,hooks,automerge features
  nodeWorkspace = buildFromUnitGraph ./build-plan-node.json;

  # git-remote-aspen: git-bridge feature
  gitRemoteWorkspace = buildFromUnitGraph ./build-plan-git-remote.json;
in {
  # Primary binaries
  node = nodeWorkspace.workspaceMembers."aspen".build;
  git-remote-aspen = gitRemoteWorkspace.workspaceMembers."aspen".build;

  # Full workspace access for advanced use
  inherit nodeWorkspace gitRemoteWorkspace;

  # Regenerate build plans (run outside Nix, requires nightly Rust):
  #   unit2nix --manifest-path Cargo.toml --features jobs,docs,blob,hooks,automerge --bin aspen-node -o nix/build-plan-node.json
  #   unit2nix --manifest-path Cargo.toml --features git-bridge --bin git-remote-aspen -o nix/build-plan-git-remote.json
}
