## Why

The dogfood pipeline claims to use snix for builds but actually shells out to `nix build` as a subprocess in all but one test variant. The `snix-build` feature is gated behind `#[cfg]` flags and only enabled in `full-aspen-node-plugins-snix-build`, which 4 tests use. The remaining 20+ VM tests and the `dogfood-local` app all use binaries without snix features, hitting the subprocess fallback every time. If we're going to carry 1200+ lines of snix-build integration code, it should be the default path, not an opt-in experiment.

## What Changes

- **Replace `full-aspen-node-plugins` with `full-aspen-node-plugins-snix-build`** as the standard dogfood binary. The 20 VM tests currently using `full-aspen-node-plugins` switch to the snix-build variant.
- **Enable `snix,snix-build` in the default `aspenNode` (u2n) binary** used by `nix run .#dogfood-local`, or replace it with a snix-build-enabled variant.
- **Enable `snix,snix-build` in `ci-aspen-node-plugins`** (the IFD-free CI variant) used by 4 tests.
- **Remove `full-aspen-node-plugins` and `full-aspen-node-plugins-snix`** (the intermediate snix-without-build variant). No binary should exist that has the CI executor but not snix-build. Two tiers: full snix-build for anything with CI, no snix for binaries that don't do builds.
- **Keep the `nix build` subprocess as a runtime fallback** — the executor already falls through gracefully when `NativeBuildService` fails. No code removal needed in the executor itself.

## Capabilities

### New Capabilities

- `snix-build-default`: snix-build is the default build backend for all dogfood and CI test binaries

### Modified Capabilities

- `snix-native-builds`: The subprocess fallback requirement changes from "feature flag opt-in" to "runtime fallback only" — `snix-build` feature is always compiled in for CI-capable binaries

## Impact

- `flake.nix`: Binary definitions (`bins.*`), test `aspenNodePackage` assignments, `aspenNode`/u2n feature lists, vendor dir setup (snix source + PROTO_ROOT propagation). ~20 test references change.
- `dogfood-local` app: Needs snix-aware binary, PROTO_ROOT, and busybox-sandbox-shell in PATH/closure.
- Build time: snix-build pulls in snix-castore, snix-store, snix-build, snix-eval, snix-glue, tonic, protobuf. Compile time for the default binary increases. Partially offset by removing redundant binary variants.
- Runtime: bwrap must be available in the VM test environments (already is for the 4 tests using snix-build today).
