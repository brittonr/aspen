# Native system-extension Octet scope

This standalone workspace compiles the real native-host functional core under the full reviewed Octet catalog. It includes positive and negative pure tests.

The check does not replace workspace tests, strict Clippy, separate-process tests, Cairn gates, or release checks.

Run it from the repository root:

```text
nix build path:$PWD#checks.x86_64-linux.native-system-extension-octet-deny-all -L --builders ''
```
