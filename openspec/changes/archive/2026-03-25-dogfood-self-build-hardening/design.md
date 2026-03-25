## Context

Aspen has three tiers of dogfood testing today:

1. **cowsay dogfood** (`ci-dogfood.nix`): Pushes a trivial cowsay flake to Forge, CI builds it. Proves the pipeline plumbing works. Passes reliably.
2. **aspen-constants self-build** (`ci-dogfood-self-build.nix`): Pushes a real but zero-dependency Aspen crate. Proves Rust compilation through CI. Also passes.
3. **full workspace** (`ci-dogfood-full-workspace.nix`): Pushes all 80 crates with pre-vendored deps via `builtins.storePath`. Bypasses the real flake checks — it uses a bespoke inner flake with `stdenv.mkDerivation` + raw `cargo build`, not the actual `build-node` / `clippy` / `nextest-quick` checks from `flake.nix`.

The gap: nobody has verified that the production `.aspen/ci.ncl` pipeline — which references `checks.x86_64-linux.build-node`, `checks.x86_64-linux.clippy`, `checks.x86_64-linux.nextest-quick` — actually works when the CI worker runs `nix build` against the pushed source. The flake checks depend on crane, ciSrc source assembly, snix vendoring, and dozens of crate overrides. Any of those can break without being caught.

`dogfood-local.sh` is the script designed to exercise this real path. It runs on the developer's machine (not in a NixOS VM), uses real binaries from the nix store, and pushes the actual git working tree to Forge. It's the closest thing to production, but it's never been run to completion in an automated reproducible manner.

## Goals / Non-Goals

**Goals:**

- Run `dogfood-local.sh full-loop` end-to-end successfully: start → push → CI pipeline (check, build, test stages) → deploy → verify
- Fix every concrete failure discovered during the run
- Create a VM integration test that exercises the same loop in an isolated, reproducible environment
- The VM test uses the real `.aspen/ci.ncl` pipeline config, not a simplified subset

**Non-Goals:**

- Multi-node cluster deployment (1-node is sufficient for self-build proof)
- Bit-for-bit reproducibility between CI-built and locally-built binaries (different source tree inputs make this impossible)
- WASM plugin builds (plugins-rpc requires the `aspen-wasm-plugin` sibling repo — excluded from ciSrc)
- Cross-compilation or non-x86_64 targets
- Performance optimization of build times (correctness first)

## Decisions

**1. Start with `dogfood-local.sh` on the host, then port to VM test**

Run the script directly first because it gives fast feedback and interactive debugging. VM tests take 15-30 minutes to build and provide less visibility into failures. Once the script passes, capture the working configuration as a VM test for CI regression.

Alternative: Build the VM test first and iterate inside it. Rejected — too slow for the fix-debug cycle.

**2. VM test uses ciSrc + crane checks (the real pipeline path)**

The VM test must build `aspen-node` using the same `craneLib.buildPackage` path that `checks.x86_64-linux.build-node` uses. This means the VM needs the full `ciSrc` derivation (with snix vendoring, stub handling, crate overrides) in its nix store. The inner `nix build` resolves flake checks from the pushed source's flake.nix.

Alternative: Use `fullSrc` + raw `cargo build` like `ci-dogfood-full-workspace.nix`. Rejected — that's what we already have and it doesn't test the real pipeline.

**3. Pre-populate VM nix store with cargo vendor dir and crane deps**

The full `nix build .#checks.x86_64-linux.build-node` from scratch would download ~3GB of cargo deps and ~2GB of nixpkgs toolchain inside the VM. This is slow and fragile (depends on cache.nixos.org availability). Instead, inject the pre-built crane `cargoArtifacts` derivation and the ciSrc into the VM's nix store closure, so the inner build only needs to compile Aspen's own crates.

**4. Deploy via script-level stop/restart (not DeploymentCoordinator)**

For a 1-node cluster, the in-process DeploymentCoordinator can't work (it sends NodeUpgrade to itself, killing the coordinator mid-flight). The script stops the old node, starts the CI-built binary with the same data dir, and waits for the cluster to come back. This is the approach `dogfood-local.sh` already implements.

## Risks / Trade-offs

[Risk] Inner `nix build` inside VM hits disk/memory limits on large workspace builds → Mitigation: Use `writableStoreUseTmpfs = false` + 40GB disk + 8GB RAM, as proven in `ci-dogfood-full-workspace.nix`.

[Risk] Crane IFD caching causes stale deps (napkin: 2026-03-14 entry) → Mitigation: Use `ciSrc` path which has resolved this via separate derivation paths. Monitor for the specific "old vendor dir reused" symptom.

[Risk] `nix build` inside VM fails on snix git dep vendoring → Mitigation: VM test pre-registers the `fullCargoVendorDir` (which includes `overrideVendorGitCheckout` for snix) in the VM's nix DB.

[Risk] CI worker timeout on full workspace build (aspen-node is ~80 crates, 436K lines) → Mitigation: Set generous timeouts in `.aspen/ci.ncl` (3600s for build, 1800s for tests). Monitor actual build times.

[Risk] `dogfood-local.sh` deploy race condition: new binary writes a new cluster-ticket.txt while script is still reading the old one → Mitigation: Script already deletes the old ticket file before restart and polls for the new one. Verify this works under slow restart conditions.
