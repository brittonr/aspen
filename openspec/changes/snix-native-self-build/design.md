## Context

The native build pipeline (`LocalStoreBuildService`) works for trivial derivations (`echo hello > $out`) but fails on real stdenv derivations (Rust crates with cargo, rustc, bash, glibc). Three gaps were identified during iteration on the self-build test:

1. **Output registration**: bwrap builds produce outputs in a temp dir. They get ingested into castore (PathInfoService) but not placed in `/nix/store`. The nix store overlay is read-only, `nix store add` content-addresses to a different hash, and symlinks into `/nix/store` fail.

2. **snix-eval flake support**: The in-process evaluator always falls back to `nix eval` subprocess. For flakes, it fails because `flake.lock` doesn't exist in CI checkouts (nix generates it on the fly), and `flake-compat` eval fails on stdenv derivations.

3. **Iteration speed**: The VM test takes ~7 minutes per cycle. Each fix requires a full nix rebuild of aspen-node (~5 min) plus VM boot + CI pipeline (~2 min). Need focused tests that run in seconds.

## Goals / Non-Goals

**Goals:**

- Native build outputs accessible at their expected `/nix/store/<hash>-<name>` path after build
- snix-eval handles flakes with nixpkgs inputs without falling back to subprocess
- Fast iteration: unit tests for each component run via `cargo nextest` in seconds
- Self-build VM test passes with native pipeline end-to-end

**Non-Goals:**

- Eliminating ALL nix CLI calls (input realisation via `nix-store --realise` is fine for now)
- Supporting IFD (import-from-derivation) in snix-eval
- CA derivations or recursive nix

## Decisions

### 1. Output registration via nix-daemon protocol

**Decision**: Use snix-store's `NixDaemonConnection` to add outputs to the local store with the correct derivation output path, bypassing the read-only overlay.

**Rationale**: `nix store add` content-addresses (wrong hash). `cp -a` fails on read-only overlay. The nix daemon protocol (`addToStoreNar` / `addPathToStore`) is the correct way to register a path with a specific name. snix-store already implements the daemon client protocol.

**Alternative considered**: Mounting the store read-write via systemd `ReadWritePaths`. Rejected — doesn't work reliably with the NixOS VM test overlay filesystem, and violates the principle of immutable store.

### 2. Generate flake.lock via `nix flake lock` before snix-eval

**Decision**: If `flake.lock` is missing and `flake.nix` exists, run `nix flake lock` subprocess to generate it before attempting snix-eval.

**Rationale**: snix-eval's call-flake.nix requires `flake.lock` to resolve inputs. Standard `nix build` generates this transparently. A one-time `nix flake lock` (~1-2s) is acceptable — the alternative (implementing flake resolution in snix-eval) is a much larger effort.

**Alternative considered**: Embedding a flake resolver in snix-eval. Deferred — would require reimplementing nix's flake registry, fetchTree, and lock file generation.

### 3. Focused test structure

**Decision**: Add three test levels:

- **Unit tests** in `build_service.rs` and `eval.rs`: test individual functions (closure computation, builder injection, flake.lock detection), run in `cargo nextest`
- **Integration test** with a real bwrap build of a trivial derivation: validates the output registration path end-to-end on the host
- **VM test** (`ci-dogfood-self-build`): the final gate, only run after unit + integration pass

**Rationale**: The VM test is the right final gate but wrong iteration loop. Unit tests for `compute_input_closure`, `derivation_to_build_request` builder injection, and `resolve_build_inputs` local fallback catch regressions in seconds.

## Risks / Trade-offs

- [Nix daemon protocol compatibility] snix-store's daemon client may not implement `addToStoreNar` for all nix daemon versions → Mitigation: fall back to subprocess `nix-store --import` if the protocol call fails
- [`nix flake lock` subprocess] Adds one nix CLI call to the eval path → Mitigation: only runs when `flake.lock` is missing, cached after first run
- [Store overlay variance] Different NixOS configurations may have different store mount behavior → Mitigation: test both `writableStoreUseTmpfs=true` and `false` in VM tests
