# World-merge Octet scope

This assembled workspace compiles the real world-commit, world-head, and world-merge core sources under the full reviewed Octet catalog. It includes all positive and negative unit tests.

The check does not replace shell tests, strict Clippy, dependency checks, Cairn gates, or repository release checks.

The fixture mirrors the core's pinned Kamacite canonical Preserves codec and Serde dependency because conversion and v2 plan identities cover canonical original-request, result, and conversion projections. Nix assembles the complete `d76fe4abe543724d8fc0ac4b362187caf2e27622` provider input, including its adapter profiles, and uses `kamacite-core.Cargo.toml` as a standalone build manifest; Rust implementation files remain unchanged. The fixture pins the same Preserves `5.0.0-rc.7` cohort as the product. Keep these declarations aligned with the core and regenerate this workspace's `Cargo.lock` with Cargo against the assembled provider; this is not a second implementation of the identity protocol.

```text
nix build path:$PWD#checks.x86_64-linux.world-merge-octet-deny-all -L --builders ''
```
