## Context

unit2nix's `fetch-source.nix` uses `pkgs.lib.cleanSourceWith` to filter each crate's source directory. This filter includes `.nix` files (only `target/` and VCS/editor files are excluded). So the Nix store hash of the source SHOULD change when `flake_compat_bundled.nix` changes.

The likely issue is that `buildRustCrate` (from nixpkgs) doesn't rebuild when only non-Rust files change in the source. `buildRustCrate` may use a secondary hash based on the crate's Rust sources (`.rs` files) or Cargo metadata, and the `.nix` file doesn't appear in that hash.

Cargo itself tracks `include_str!` dependencies only if a `build.rs` emits `cargo:rerun-if-changed`. Without it, cargo rebuilds based on `.rs` file timestamps. Nix's `buildRustCrate` may mirror this behavior.

## Goals / Non-Goals

**Goals:**

- `flake_compat_bundled.nix` changes trigger a Nix rebuild of `aspen-ci-executor-nix`
- Solution works for all `include_str!`'d files across the workspace
- No manual cache busting needed (no dummy comments in Rust files)

**Non-Goals:**

- Changing unit2nix upstream (fix locally first)
- Fixing buildRustCrate in nixpkgs (override locally)

## Decisions

### D1: Add `build.rs` with `rerun-if-changed`

Add `crates/aspen-ci-executor-nix/build.rs`:

```rust
fn main() {
    println!("cargo:rerun-if-changed=src/flake_compat_bundled.nix");
}
```

This makes cargo aware of the dependency. For Nix, `buildRustCrate` runs build scripts, so this should cause a hash change in the build output.

### D2: Verify unit2nix source hash includes `.nix` files

Check if `cleanSourceWith` produces different hashes when only `.nix` changes. If not, the filter needs adjustment (it should already work based on the code).

### D3: Crate override fallback

If D1+D2 don't fix it, add a crate override in `flake.nix` or `unit2nix.nix`:

```nix
aspen-ci-executor-nix = attrs: {
  # Force rebuild when include_str'd files change
  extraRustcOpts = ["--cfg" "include_str_hash=\"${builtins.hashFile "sha256" (src + "/crates/aspen-ci-executor-nix/src/flake_compat_bundled.nix")}\""];
};
```

## Risks / Trade-offs

**[Risk] build.rs slows incremental builds** → The build script is trivial (one println), adding <1ms. No risk.

**[Risk] crate override is fragile** → The `extraRustcOpts` approach leaks a hash into compiler flags. Prefer D1 if it works.
