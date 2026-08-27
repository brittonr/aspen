# Fabric execution Octet scope

This standalone workspace compiles the real Molten fabric and bounded execution cores. The gate uses the full reviewed Octet catalog across all targets and features.

The scope includes positive and negative pure tests. It does not replace workspace tests, strict Clippy, live adapter tests, Cairn gates, or release checks.

Run it from the repository root:

```text
nix build path:$PWD#checks.x86_64-linux.fabric-execution-octet-deny-all -L --builders ''
```
