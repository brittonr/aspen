## Why

unit2nix builds individual Rust crates as separate Nix derivations, hashing each crate's source directory to determine cache keys. Files loaded via `include_str!` or `include_bytes!` (like `.nix`, `.ncl`, `.sql` files) are in the source directory and included in the hash. However, when these non-Rust files change, the Nix derivation for the crate is sometimes served from cache because `buildRustCrate`'s rebuild detection doesn't track `include_str!` dependencies the way `cargo` does (via `rerun-if-changed` in build scripts).

This blocks the snix-eval-lazy-mode change: `flake_compat_bundled.nix` was patched to remove rnix-incompatible `or` keywords, but the Nix build serves the old binary with the unpatched file baked in.

## What Changes

- Add a `build.rs` to `aspen-ci-executor-nix` that emits `cargo:rerun-if-changed=src/flake_compat_bundled.nix` so cargo tracks the dependency
- Verify the unit2nix source hash changes when `flake_compat_bundled.nix` changes (cleanSourceWith includes `.nix` files)
- If the hash does change but Nix still caches: add a `passthru.includeStrHash` to the crate override that forces rebuild

## Capabilities

### New Capabilities

- `include-str-rebuild`: Ensure `include_str!`'d non-Rust files trigger Nix derivation rebuilds when their content changes

### Modified Capabilities

## Impact

- `crates/aspen-ci-executor-nix/build.rs` — new build script
- `nix/unit2nix.nix` or flake.nix crate overrides — if source hash alone is insufficient
- Affects any crate using `include_str!` with non-Rust files: `aspen-ci-executor-nix`, `aspen-ci`, `aspen-nickel`
