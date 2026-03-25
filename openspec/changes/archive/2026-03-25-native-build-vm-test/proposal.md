## Why

The native snix-build path was just wired into `execute_build()` but is only validated by unit tests (mock nix binary, no-service fallback). There's no proof it works end-to-end under real conditions: real Nix evaluation, real bubblewrap sandbox, real PathInfoService upload, real cache gateway serving the result. A NixOS VM integration test closes this gap.

## What Changes

- New NixOS VM test `nix/tests/snix-native-build.nix` that builds a trivial flake entirely through the snix-build path (bubblewrap sandbox), verifies outputs land in PathInfoService, and confirms the cache gateway can serve the resulting narinfo
- New node package variant `full-aspen-node-plugins-snix-build` in flake.nix that enables `snix-build` feature (extends the existing `full-aspen-node-plugins-snix` with the extra feature flag)
- Test registered in flake.nix checks

## Capabilities

### New Capabilities

- `native-build-e2e-test`: The VM test that validates eval→drv→bubblewrap→PathInfoService→cache gateway

### Modified Capabilities

## Impact

- `nix/tests/snix-native-build.nix`: New file
- `flake.nix`: New node package + new check entry
- No changes to application code — this is a test-only change
