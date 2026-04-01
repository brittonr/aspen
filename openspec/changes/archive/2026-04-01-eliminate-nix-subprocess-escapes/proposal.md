## Why

The snix native build pipeline can evaluate and build simple derivations without any `nix` CLI subprocess. But 8 subprocess escape hatches remain in `aspen-ci-executor-nix` — calls to `nix eval`, `nix path-info`, `nix nar dump-path`, `nix-store -qR`, `nix-store --realise`, `nix flake lock`, `nix build`, and `curl`. These prevent Aspen from building itself (or any non-trivial flake) in a truly nix-free environment. Eliminating them is the path to self-hosted builds where the cluster's own PathInfoService and BlobService replace the local nix store.

## What Changes

- Replace `nix path-info --json` with snix-store PathInfoService queries for NAR size, references, and deriver lookup
- Replace `nix nar dump-path` with snix-store's `SimpleRenderer` for NAR serialization from castore
- Replace `nix-store -qR` closure computation with recursive PathInfoService reference walking
- Replace `curl` tarball fetching in `fetch.rs` with in-process HTTP via `reqwest`
- Add upstream binary cache client that queries `cache.nixos.org` narinfo API to populate PathInfoService for paths not yet in the cluster store (closure bootstrap)
- Gate all remaining `Command::new("nix*")` calls behind `nix-cli-fallback` feature flag so the zero-subprocess path is the default and subprocess usage is opt-in

## Capabilities

### New Capabilities

- `upstream-cache-client`: HTTP client for querying upstream Nix binary caches (cache.nixos.org) to resolve narinfo, fetch NARs, and populate the cluster's PathInfoService. Enables closure bootstrap without a local nix installation.
- `in-process-nar-export`: NAR serialization from castore nodes using snix-store's SimpleRenderer, replacing `nix nar dump-path` subprocess.
- `in-process-closure-walk`: Recursive reference walking via PathInfoService to compute input closures, replacing `nix-store -qR`.

### Modified Capabilities

- `snix-native-builds`: The native build path becomes the default. All subprocess calls move behind `nix-cli-fallback`. `cache.rs` uses PathInfoService instead of `nix path-info`.
- `flake-input-fetch`: `fetch.rs` replaces `curl` subprocess with `reqwest` HTTP client for tarball downloads.

## Impact

- **`crates/aspen-ci-executor-nix/src/cache.rs`**: Rewrite `check_store_path_size` and `upload_store_paths` to use PathInfoService + SimpleRenderer
- **`crates/aspen-ci-executor-nix/src/fetch.rs`**: Replace `curl` with `reqwest`, keep tarball unpacking via `flate2`/`tar`
- **`crates/aspen-ci-executor-nix/src/build_service.rs`**: Replace `compute_input_closure` (nix-store -qR) with PathInfoService reference BFS
- **`crates/aspen-ci-executor-nix/src/executor.rs`**: Gate `resolve_drv_path` (nix eval), `nix-store --realise`, and `spawn_nix_build` behind `nix-cli-fallback`
- **`crates/aspen-ci-executor-nix/src/eval.rs`**: Gate `nix eval --json` and `nix flake lock` fallbacks behind `nix-cli-fallback`
- **`crates/aspen-ci-executor-nix/Cargo.toml`**: Add `reqwest` dep (already transitive via iroh), restructure feature flags
- **NixOS VM tests**: Update `snix-pure-build-test` to verify zero subprocess calls for a flake with actual input dependencies
- **New NixOS VM test**: Cluster bootstraps nixpkgs paths from cache.nixos.org, then builds a derivation using only PathInfoService
