## 1. Node Package

- [x] 1.1 Add `full-aspen-node-plugins-snix-build` package to flake.nix — extends `full-aspen-node-plugins-snix` cargoExtraArgs with `snix-build` feature flag

## 2. VM Test

- [x] 2.1 Create `nix/tests/snix-native-build.nix` — single-node VM test that boots aspen-node with snix-build, submits a nix build job for a trivial flake, waits for completion, and verifies output paths
- [x] 2.2 Add bubblewrap to VM environment packages so `init_native_build_service()` detects the sandbox
- [x] 2.3 Add cache gateway verification: start gateway after build, query narinfo for the built store path, assert valid response

## 3. Flake Integration

- [x] 3.1 Register the test in flake.nix checks as `snix-native-build-test`
