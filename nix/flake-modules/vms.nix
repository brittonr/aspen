# VM images and microvm configurations.
#
# VM infrastructure (cloud hypervisor, virtiofs, CI VM kernel/initrd)
# remains in flake.nix for now — it's tightly coupled with the Rust
# build outputs and the NixOS test infrastructure.
#
# Migration plan: extract after rust.nix exposes built packages.
{...}: {}
