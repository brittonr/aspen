# Runnable apps (cluster, dogfood, fuzz, bench, coverage, verify-verus).
#
# Apps remain in flake.nix for now — they depend on crane-built
# binaries, verus, and script definitions from the main perSystem block.
#
# Migration plan: extract after rust.nix and verus.nix expose their
# outputs via flake-parts options.
{...}: {}
