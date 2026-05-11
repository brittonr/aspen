{
  id = "multi-node-blob-vm",
  display_name = "Multi-node blob VM test",
  description = "Three-node NixOS VM coverage for blob replication and failover survival.",
  layer = "vm",
  owner = "blob",
  runtime_class = "nixos-vm",
  prerequisites = ["nix-command"],
  tags = ["blob", "replication", "vm"],
  target = {
    kind = "nix-build",
    flake_attr = "checks.x86_64-linux.multi-node-blob-test",
    check_attr = "multi-node-blob-test",
    nix_file = "nix/tests/multi-node-blob.nix",
    package_presets = {
      aspenNodePackage = "ci-aspen-node",
      aspenCliPackage = "ci-aspen-cli",
    },
    register_flake_check = false,
  },
}
