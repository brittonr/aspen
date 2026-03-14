# Rust/crane build logic.
#
# The core Rust build pipeline (source filtering, cargo artifacts, crane
# package definitions) remains in flake.nix for now. These ~2000 lines
# of tightly-coupled let-bindings are the backbone of the build and share
# state with checks, apps, devShells, and VM test infrastructure.
#
# Migration plan:
#   1. Define flake-parts options for the key outputs (craneLib, commonArgs,
#      cargoArtifacts, binary packages)
#   2. Move source assembly and vendor patching here
#   3. Move crane build definitions here
#   4. Have checks.nix, apps.nix, dev.nix consume via config.aspen.*
#
# This requires flake-parts option definitions (lib.mkOption) to expose
# values across modules — a follow-up change.
{...}: {}
