# Avoid VMCI Nix Store FD Pressure

## Why

The latest `nix run .#dogfood-local-vmci-medium` proof reached the real `build-cli` command in the guest, after VMCI startup, source push, WIP overlay, CI trigger, workspace materialization, lock-original preservation, and selective `tigerstyle` lock rewrite all worked. It still failed with:

```text
copying path '/nix/store/i2gsp87gqp16whm9mw0ybk9n84zir01x-source' from 'https://cache.nixos.org'...
error: chmod ".../adoptopenjdk-icedtea-web/patches": Too many open files in system
unpacking 'github:NixOS/nixpkgs/...'
```

This means the remaining blocker is not route retention, Forge/ListRepos, source archive propagation, guest pre-transform, timeout finalization, `narHash`, or GitHub `HEAD` refresh. The root boundary is guest Nix materializing/traversing large source inputs such as `nixpkgs` through the VMCI/virtiofs/Nix-store path, which creates host/guest file-descriptor pressure. Raising limits alone already failed to remove the architectural risk; Aspen needs a bounded source-input strategy for VMCI CI jobs.

## What Changes

- **Classify the boundary**: Detect and report VMCI Nix source-materialization FD pressure as its own failure class with bounded evidence.
- **Constrain Nix input materialization**: VMCI `ci_nix_build` jobs must not rely on unbounded guest traversal/copying of large public source trees through virtiofs-backed `/nix/store`.
- **Add an explicit input strategy**: Choose and implement a VMCI-safe mode for large public flake inputs: guest-native fetching into guest-local store/cache, host-provided binary/cache proxy without virtiofs source-tree traversal, or pre-seeded guest-local store/castore import with concurrency/file-handle bounds.
- **Guard against regression**: Keep selected private/offline inputs such as `tigerstyle` rewritable, but prevent broad path rewriting or cache behavior that forces `nixpkgs`-scale trees through host `/nix/store` in the guest.
- **Prove with layered rails**: Re-run medium before clippy/full and capture receipts showing `format-check` and `build-cli` either pass or fail at a new, narrower non-FD boundary.

## Capabilities

### New Capabilities
- `vmci-nix-store-boundary`: Bounded VMCI Nix source-input/materialization behavior and diagnostics.

### Modified Capabilities
- `ci-failure-diagnostics`: Adds a deterministic diagnostic for Nix source-materialization FD pressure.
- `dogfood-evidence`: Requires medium VMCI receipts to preserve this boundary without secrets.

## Impact

- **Files**: VM worker/executor Nix command construction, workspace lock rewrite, VM provisioning/cache/store mounts, dogfood diagnostics, VMCI rail evidence, tests.
- **APIs**: No external API change expected; internal payload/config may gain a VMCI Nix input/cache strategy field.
- **Dependencies**: Prefer no new external dependency. Any Nix/cache/castore helper must be feature-gated or reuse existing Aspen/snix components.
- **Testing**: Unit tests for input classification and command/store strategy; regression tests preventing broad public input path rewrites; diagnostic tests for `Too many open files`; live `dogfood-local-vmci-medium` receipt before escalating to clippy/full.

## Non-Goals

- Replacing Nix evaluation/build semantics.
- Making GitHub/network access mandatory for private or missing inputs.
- Solving general VMCI file-descriptor exhaustion by only raising `ulimit` or host `virtiofsd` limits.
- Running the full/clippy VMCI rail before medium proves the source-input boundary is cleared.
- Logging raw tickets, raw Iroh secret keys, credentials, full environments, or unbounded argv/log output.
