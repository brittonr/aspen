## Context

The Nix CI executor has three build paths compiled behind feature flags:

1. **npins native** (`snix-build` + `snix-eval`): Zero subprocesses. snix-eval resolves the derivation, snix-build's `BuildService` executes it in a bwrap sandbox. Only works for npins-based projects.
2. **flake native** (`snix-build`): `nix eval` subprocess resolves the .drvPath, then `BuildService` executes the build. Works for any flake.
3. **subprocess** (always available): `nix build` subprocess. The fallback.

Today, 20 VM tests and the `dogfood-local` app compile without `snix-build`, so they always hit path #3. Four tests use `full-aspen-node-plugins-snix-build` with paths #1/#2.

The Nix build infrastructure in `flake.nix` has grown four binary variants that differ only in snix feature sets: `full-aspen-node-plugins`, `full-aspen-node-plugins-snix`, `full-aspen-node-plugins-snix-build`, and `full-aspen-node-e2e`. This is the root cause — we keep adding variants instead of promoting snix-build to the default.

## Goals / Non-Goals

**Goals:**

- Every CI-capable aspen-node binary compiles with `snix,snix-build` features enabled
- All dogfood VM tests exercise the native build path by default
- `dogfood-local` app uses snix-build
- Fewer binary variants in flake.nix (consolidate, don't proliferate)

**Non-Goals:**

- Removing the `nix build` subprocess fallback from the executor code — it's a safety net for environments where bwrap isn't available or snix-eval hits an unsupported builtin
- Making snix-build work without bwrap (OCI fallback is separate work)
- Changing binaries that don't have CI features (e.g., `aspen-cli`, `git-remote-aspen`)

## Decisions

### D1: Promote `full-aspen-node-plugins-snix-build` to be `full-aspen-node-plugins`

**Rationale**: Instead of changing 20 test references, absorb the snix-build config into the existing `full-aspen-node-plugins` definition. This means changing one binary definition, not 20 test files.

**Mechanics**: Take the current `full-aspen-node-plugins-snix-build` definition (src, vendor dir, PROTO_ROOT, SNIX_BUILD_SANDBOX_SHELL, features, postInstall) and make that the `full-aspen-node-plugins` definition. Delete `full-aspen-node-plugins-snix` and the old `full-aspen-node-plugins-snix-build`.

**Alternative considered**: Sed-replace all 20 test references. More churn, same result, and leaves dead binary definitions.

### D2: Same treatment for `ci-aspen-node-plugins`

The IFD-free CI variant needs the same snix-build features and build infrastructure (fullSrcWithSnix, snix vendor dir, PROTO_ROOT, SNIX_BUILD_SANDBOX_SHELL).

### D3: Enable snix,snix-build in the u2n default aspenNode

The `dogfood-local` app uses `aspenNode` which is built via unit2nix with features `ci,docs,hooks,shell-worker,automerge,secrets,git-bridge,deploy`. Add `snix,snix-build` to this feature list and propagate the build environment (PROTO_ROOT, SNIX_BUILD_SANDBOX_SHELL, snix source).

This is the riskiest change because unit2nix has a different build pipeline than crane. Need to verify it handles the snix git dependency vendoring and protobuf generation correctly.

### D4: Remove `full-aspen-node-e2e`

It has `snix` but not `snix-build` or `snix-eval` — a partial state that shouldn't exist. If anything needs it, use the consolidated `full-aspen-node-plugins` instead.

### D5: Keep `full-aspen-node-plugins-snix-build` as an alias during transition

To avoid breaking any scripts or CI references, keep it as `full-aspen-node-plugins-snix-build = bins.full-aspen-node-plugins;` temporarily.

## Risks / Trade-offs

**[Compile time increase]** → The default binary now pulls in snix-{build,eval,castore,store,glue}, tonic, protobuf codegen. Adds ~2-3 minutes to a clean build. Mitigated by cargo artifacts caching and by reducing the total number of binary variants (fewer unique builds).

**[u2n + snix vendoring]** → unit2nix may not handle the snix git dependency override the same way crane does. → Test the u2n build first in isolation before touching dogfood-local.

**[bwrap availability in test VMs]** → All NixOS VM tests already have a working `/nix/store` and busybox. The 4 existing snix-build tests prove bwrap works in this environment. Low risk.

**[Fallback masking bugs]** → If snix-build silently fails and falls back to `nix build` subprocess, we won't know the native path is broken. → Add a log-level assertion or metric in the executor that warns when fallback is triggered. Consider a `--require-native-build` flag for tests.
