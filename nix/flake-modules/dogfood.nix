# Dogfood pipeline (self-hosted build).
#
# Dogfood apps and VM configurations remain in flake.nix for now —
# they depend on crane-built aspen-node and VM infrastructure.
#
# Migration plan: extract after rust.nix and vms.nix are available.
{...}: {}
