## Why

A fresh clustering sweep proved Aspen's core Rust/Iroh/Raft clustering path and a fallback minimal 3-QEMU-VM NixOS cluster, but the two stock cluster VM rails did not run to completion. `microvm-cluster-test` fails before exercising the VM script because its build closure is out of sync around `iroh`/`portmapper`. `multi-node-cluster-test` fails before VM execution because its external WASM forge plugin fixture is out of sync with current `ForgeRepoInfo`/`PluginInfo` APIs.

Those failures leave operator evidence ambiguous: core clustering works, but the named stock checks that should prove it in the repository are not green.

## What Changes

- Restore the stock `microvm-cluster-test` build closure so it reaches and executes its 3-node cluster script instead of failing during dependency build.
- Restore the stock `multi-node-cluster-test` fixture boundary so optional WASM forge/plugin API drift cannot block the core cluster VM proof before boot.
- Preserve the proof boundary between core cluster formation, optional plugin/forge coverage, and microVM/AspenFs coverage.
- Capture fresh focused evidence for both stock checks after repair.

## Capabilities

### Modified Capabilities
- `test-harness-runtime`: Stock cluster VM checks remain runnable focused evidence targets and distinguish build-input drift from product clustering failures.
- `plugins`: VM fixtures that include plugin binaries track the host ABI or degrade without blocking unrelated cluster formation proof.

## Impact

- **Files**: Nix VM test definitions, flake/package inputs for the affected checks, plugin fixture/package wiring, and OpenSpec records.
- **APIs**: No intended public Rust API changes; if plugin fixture code must adapt to current APIs, it should do so in the fixture/plugin crate boundary.
- **Dependencies**: May update or patch Nix build inputs for `iroh`/`portmapper` compatibility and external plugin fixture sources.
- **Testing**: Focused `nix build --impure .#checks.x86_64-linux.microvm-cluster-test --no-link -L`, focused `nix build --impure .#checks.x86_64-linux.multi-node-cluster-test --no-link -L`, relevant Nix eval/package checks, `openspec validate --all --strict --json`, and `git diff --check`.
