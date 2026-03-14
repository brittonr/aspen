# NixOS VM integration tests.
#
# VM tests remain in flake.nix for now — they depend on crane-built
# packages and VM infrastructure defined in the main perSystem block.
#
# Migration plan: extract after rust.nix and vms.nix expose their
# outputs via flake-parts options.
{...}: {}
