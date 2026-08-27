# Executable-extent Octet scope

This assembled standalone workspace compiles the product-owned executable-extent shell and a minimal facade over its real pure-core source under the full Octet catalog. It prevents inherited findings in unrelated Molten modules from hiding findings in this optional profile.

It is a check surface only. Product dependencies remain the immutable Cargo and Nix pins in the main workspace. This workspace does not own runtime behavior or release evidence.

Run it from the repository root:

```text
nix build path:$PWD#checks.x86_64-linux.executable-extent-octet-deny-all -L --builders ''
```
