# NixOS modules for Aspen distributed consensus system.
#
# This module set provides:
# - aspen-node: Service module for running Aspen consensus nodes
#
# Usage in a flake:
#   nixosModules.aspen-node = import ./nix/nixos-modules/aspen-node.nix;
#
# Or import directly:
#   imports = [ ./nix/nixos-modules ];
{
  imports = [
    ./aspen-node.nix
  ];
}
