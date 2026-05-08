# Verification Plan

Closeout for this change happens after all implementation/evidence tasks are complete:

1. Build the marker package with `nix build .#hermit-uhyve-marker --no-link -L`.
2. Run the marker fixture package/metadata contract check.
3. Run the ignored real Hermit/Uhyve product-path proof on a capable host using `.#uhyve` and the packaged marker image.
4. Run `scripts/test-harness.sh export` and `scripts/test-harness.sh check`.
5. Run `openspec validate package-hermit-uhyve-marker-fixture --strict` and `openspec validate --all --strict --json`.
6. Run `git diff --check`.
7. Archive, commit, push, and verify clean state.

## Captured Evidence

- `nix build .#hermit-uhyve-marker --no-link --print-out-paths -L` passed and produced a source-built `bin/aspen-hermit-uhyve-marker` image.
- `nix build .#checks.x86_64-linux.hermit-uhyve-marker-contract --no-link --print-out-paths -L` passed; the check validates the image path, executable bit, schema, source revision, target triple, expected marker, relative image path, and `fixture-build-is-not-runtime-host-proof` boundary.
- `openspec/changes/archive/2026-05-08-package-hermit-uhyve-marker-fixture/evidence/packaged-marker-product-proof.log` records the ignored real product-path proof using `.#uhyve` and `.#hermit-uhyve-marker`: `hermit_uhyve_executes_declared_fixture_through_product_orchestration ... ok`.

The marker package build and contract check are prerequisite evidence only; runtime-host proof still requires Aspen `JobManager`/`WorkerPool` execution with marker `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED` in the product-visible receipt.
