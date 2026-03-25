## Context

The `ci-nix-build.nix` test validates the subprocess `nix build` path through the CI pipeline (forge push → auto-trigger → NixBuildWorker → subprocess). The `snix-store.nix` test validates SNIX storage traits. Neither exercises the native snix-build sandbox path end-to-end. The new test targets the gap: snix-build's bubblewrap sandbox producing outputs that flow into PathInfoService and out through the cache gateway.

## Goals / Non-Goals

**Goals:**

- Build a trivial flake through the full native pipeline: `nix eval .drvPath` → parse `.drv` → bubblewrap sandbox → PathInfoService → cache gateway narinfo
- Prove snix-build works inside a NixOS VM (bubblewrap available, FUSE support)
- Validate that `init_native_build_service()` detects bubblewrap at startup
- Verify the cache gateway serves narinfo for natively-built store paths

**Non-Goals:**

- Testing the CI auto-trigger pipeline (covered by `ci-nix-build.nix`)
- Testing forge/git push (covered by `e2e-push-build-cache.nix`)
- Testing CA derivations, multi-output builds, or IFD

## Decisions

### Test structure: standalone node, no forge/CI pipeline

The test doesn't need forge, git push, or CI auto-trigger. It starts a single aspen-node with snix-build enabled, submits a nix build job directly via CLI, and validates the output. This keeps the test fast (~2-3 min) and focused on the native build path.

### Node package: add snix-build to existing snix node features

Extend `full-aspen-node-plugins-snix` with `snix-build` in the cargoExtraArgs. This avoids duplicating the entire snix vendor/build setup. The feature chain `snix-build` → `snix-eval` → `snix` means all lower-level features come along.

### Bubblewrap in VM

NixOS VMs have kernel namespaces available. Add `pkgs.bubblewrap` to environment packages so `bwrap` is on PATH. `init_native_build_service()` will detect it and use the bubblewrap sandbox backend.

### Flake under test

Same pattern as `ci-nix-build.nix`: a trivial derivation that runs `/bin/sh -c "echo ... > $out"`. No network access needed, no IFD, no nixpkgs dependency.

## Risks / Trade-offs

- **[bubblewrap may not work in nested VMs]** → NixOS test VMs run in QEMU with KVM. bubblewrap uses user namespaces, not KVM, so it should work. If it doesn't, the test will show the fallback to OCI or dummy — which itself is useful diagnostic information.
- **[snix-build node package adds build time]** → The snix vendor setup is already done for `full-aspen-node-plugins-snix`. Adding `snix-build` is an incremental feature flag, not a new vendoring step.
