# CI checks (clippy, fmt, audit, nextest, VM integration tests).
#
# Checks remain in flake.nix for now — they depend on craneLib,
# commonArgs, ciCommonArgs, and other Rust build infrastructure
# that hasn't been extracted into flake-parts options yet.
#
# Migration plan: extract after rust.nix exposes build config via options.
{...}: {}
