## Context

The self-hosting dogfood tests prove that code pushed to Forge compiles through Aspen's CI pipeline. Three tests exist today:

- `ci-dogfood-test`: builds cowsay (external nixpkgs package) — proves the pipeline works
- `ci-dogfood-self-build-test`: builds aspen-constants (1 crate, zero deps) — proves Rust compilation works
- `ci-dogfood-workspace-test`: builds 18 crates (32K lines, 509 packages) — proves workspace resolution works

None of them compile the full 80-crate workspace (343K lines, 658 packages). The `fullSrc` derivation in `flake.nix` already packages everything needed for a complete build: all 80 crates, vendored openraft, stubbed git deps (snix, mad-turmoil, h3-iroh), external deps (iroh-proxy-utils, aspen-wasm-plugin), and a patched Cargo.lock with git source lines stripped. This derivation drives the host `nix build .#aspen-node` that already works. The new VM test needs to get this same source tree through Forge and into the NixBuildWorker.

The challenge: the existing tests manually copy individual crate source trees into a `pkgs.runCommand` derivation. That approach doesn't scale to 80 crates with deep directory structures. Instead, this design pushes the pre-built `fullSrc` derivation (or a derivative) directly through git-remote-aspen.

## Goals / Non-Goals

**Goals:**

- Full 80-crate workspace compiles through Forge → CI → NixBuildWorker inside a NixOS VM
- The CI-built binary (`aspen-node`) runs and responds to `--version`
- Build logs stream via `ci logs --follow` during compilation
- Artifact storage verified: iroh-blobs, nix binary cache, SNIX
- Test is reproducible and runs in CI (`nix build .#checks.x86_64-linux.ci-dogfood-full-workspace-test --impure --option sandbox false`)

**Non-Goals:**

- Running the test suite inside the VM (compilation alone takes 15-30 min; tests add another 10+)
- Building with `plugins-rpc` feature (requires hyperlight WASM runtime, nested KVM)
- Building `aspen-cli` or `git-remote-aspen` — just the node binary
- Matching the exact feature set of the host build (skip `plugins-rpc` and `net` to avoid hyperlight and iroh-proxy complexity)

## Decisions

**1. Reuse `fullSrc` as the git push payload**

The `fullSrc` derivation already handles all the hard parts: stubbing git deps, rewriting Cargo.toml paths, stripping git source lines from Cargo.lock, copying iroh-proxy-utils and aspen-wasm-plugin. Rather than recreating this logic in the test, we copy `fullSrc` into a git repo and push it through `git-remote-aspen`.

Alternative considered: manually listing all 80 crate source paths like the 18-crate test does. Rejected because it's fragile (breaks when files are added/moved) and the 18-crate test already showed this approach doesn't scale.

**2. Use `craneLib.buildPackage`-style flake inside the VM**

The inner flake (pushed to Forge) uses `rustPlatform.buildRustPackage` with `cargoLock.lockFile` for dependency vendoring. This matches how `ci-dogfood-workspace-test` works — the Nix sandbox handles all crate fetching. The feature set is `ci,docs,hooks,shell-worker,automerge,secrets,forge,git-bridge,blob` (everything except `plugins-rpc`, `net`, and `snix`).

Alternative considered: crane inside the VM. Rejected because crane needs `vendorCargoDeps` which requires network access during eval, and the VM's nix sandbox would block it.

**3. VM resource allocation: 8GB RAM, 40GB disk, 4 cores**

The full Rust toolchain download is ~2GB, the cargo build peaks at ~4GB resident, and linking aspen-node needs ~2GB. 8GB RAM with `writableStoreUseTmpfs = false` (disk-backed store overlay) avoids the OOM that killed earlier tests. 40GB disk covers toolchain + 658 crate sources + build artifacts + nix store growth. 4 cores for parallel compilation.

**4. Single-stage pipeline, single job**

Unlike the 2-stage tests (check → build), this test uses a single `nix build` job. The cargo-check stage is redundant when doing a full build — it just adds 5+ minutes of duplicate type-checking. One stage, one job, one `nix build .#packages.x86_64-linux.default`.

**5. `fullSrc` subdirectory structure**

`fullSrc` places the workspace under `$out/aspen/`. The git repo pushed to Forge will have the workspace at the root, so the inner flake's `src = ./.` points at the workspace root. The `postUnpack` dance (`sourceRoot=$sourceRoot/aspen`) that crane needs on the host is unnecessary here because we flatten the structure when copying into the git repo.

**6. Timeout: 1800s (30 min)**

The 18-crate build takes ~4 min for cargo-check + ~4 min for build inside the VM. Extrapolating to 80 crates with heavier deps (ring crypto, redb, iroh QUIC, tokio, DataFusion): 15-25 min for compilation. 30 min gives comfortable margin. The checkout/prefetch phase adds another 2-5 min for rustc download.

## Risks / Trade-offs

[Risk: VM build takes >30 minutes] → Increase timeout to 2400s. Alternatively, pre-populate the VM's nix store with rustc/cargo/stdenv to skip the download phase (the `nix.registry.nixpkgs.flake` already helps here).

[Risk: Disk exhaustion during link step] → 40GB should be sufficient. The host build produces a 63MB binary from ~4GB of build artifacts. Monitor with `df -h` in test if needed.

[Risk: `fullSrc` changes break the inner flake] → The inner flake only depends on Cargo.toml, Cargo.lock, and src/ directories. It doesn't use `.cargo/config.toml` vendoring (nix handles that). Changes to fullSrc's stub strategy could affect the Cargo.lock, but the lock file is self-consistent within fullSrc.

[Risk: Memory pressure during parallel compilation] → Use `CARGO_BUILD_JOBS=2` to cap parallelism if 4-core × 4 threads overwhelms 8GB. The linker is the peak; compilation units are individually smaller.

[Trade-off: No plugins-rpc feature] → Hyperlight requires KVM, which means nested virtualization inside QEMU. This is possible but fragile and slow. The plugins-rpc feature compiles ~40 additional crates (hyperlight, wasmtime). Excluding it still proves 95%+ of the codebase compiles. A future test can add it with nested KVM support.

[Trade-off: No test execution] → `doCheck = false` in the inner flake. Running 5,700+ tests inside a 2-core VM would take 30+ minutes on top of compilation. The host CI already runs tests. This test proves the build works end-to-end through the self-hosting loop.
