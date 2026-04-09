{
  id = "multi-node-kv-vm",
  display_name = "Multi-node KV VM test",
  description = "Three-node NixOS VM coverage for KV replication and follower forwarding.",
  layer = "vm",
  owner = "kv",
  runtime_class = "nixos-vm",
  prerequisites = ["nix-command"],
  tags = ["kv", "replication", "vm"],
  target = {
    kind = "nix-build",
    flake_attr = "checks.x86_64-linux.multi-node-kv-test",
    check_attr = "multi-node-kv-test",
    nix_file = "nix/tests/multi-node-kv.nix",
    package_presets = {
      aspenNodePackage = "full-aspen-node",
      aspenCliPackage = "full-aspen-cli",
    },
    register_flake_check = true,
  },
}
