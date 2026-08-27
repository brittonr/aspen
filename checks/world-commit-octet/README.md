# World-commit Octet scope

This assembled standalone workspace compiles the real `molten-core` world-commit source under the full reviewed Octet catalog. It includes the module's positive and negative unit tests through the normal `--all-targets --all-features` scope.

The check isolates the deterministic protocol core from inherited findings in unrelated Molten modules. It does not replace full-workspace tests, strict Clippy, shell tests, Cairn gates, or repository release checks.

Run it from the repository root:

```text
nix build path:$PWD#checks.x86_64-linux.world-commit-octet-deny-all -L --builders ''
```
