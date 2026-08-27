# World-head Octet scope

This assembled workspace compiles the real world-commit and world-head core sources under the full reviewed Octet catalog. It includes positive and negative unit tests through `--all-targets --all-features`.

The check isolates this protocol from inherited findings in unrelated Molten modules. It does not replace shell tests, strict Clippy, Cairn gates, or repository release checks.

Run it from the repository root:

```text
nix build path:$PWD#checks.x86_64-linux.world-head-octet-deny-all -L --builders ''
```
