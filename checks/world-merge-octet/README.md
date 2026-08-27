# World-merge Octet scope

This assembled workspace compiles the real world-commit, world-head, and world-merge core sources under the full reviewed Octet catalog. It includes all positive and negative unit tests.

The check does not replace shell tests, strict Clippy, dependency checks, Cairn gates, or repository release checks.

```text
nix build path:$PWD#checks.x86_64-linux.world-merge-octet-deny-all -L --builders ''
```
