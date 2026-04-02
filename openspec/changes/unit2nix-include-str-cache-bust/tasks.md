## 1. Add build.rs

- [ ] 1.1 Create `crates/aspen-ci-executor-nix/build.rs` with `cargo:rerun-if-changed=src/flake_compat_bundled.nix`
- [ ] 1.2 Add `build = "build.rs"` to Cargo.toml if not auto-detected
- [ ] 1.3 Verify `cargo build -p aspen-ci-executor-nix --features snix-build` runs the build script
- [ ] 1.4 Audit other `include_str!` crates (`aspen-ci`, `aspen-nickel`) and add build.rs if missing

## 2. Verify Nix rebuild

- [ ] 2.1 Run `nix build .#checks.x86_64-linux.snix-flake-native-build-test --impure` and check derivation hash changed
- [ ] 2.2 Verify `strings` on the new binary shows `if info ? host then` (patched flake-compat)
- [ ] 2.3 Check VM test logs for "zero subprocesses" vs "falling back"

## 3. Fallback: crate override

- [ ] 3.1 If build.rs alone doesn't fix: add crate override with `extraRustcOpts` hash injection
- [ ] 3.2 Verify override triggers rebuild on `.nix` file change

## 4. Cleanup

- [ ] 4.1 Remove the dummy comment added to `flake_compat.rs` (no longer needed as cache bust)
- [ ] 4.2 Run all snix VM tests to confirm no regressions
- [ ] 4.3 Commit with verification evidence
