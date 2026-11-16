# Native system-extension Octet scope

This standalone workspace compiles the real native-host functional core under the full reviewed Octet catalog. It includes positive and negative pure tests.

Nix mounts the authoritative `crates/molten-core/src/system_extension/generation/mod.rs` implementation at `src/system_extension/generation/mod.rs`. The focused lifecycle facade uses its nominal `Generation` and `Incarnation` types, preserving the production constructors, explicit `u64` projection, checked arithmetic, and full `u64` range without duplicating their implementation.

The check does not replace workspace tests, strict Clippy, separate-process tests, Cairn gates, or release checks.

Run it from the repository root:

```text
nix build path:$PWD#checks.x86_64-linux.native-system-extension-octet-deny-all -L --builders ''
```
