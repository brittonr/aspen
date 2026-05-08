# Verification Plan

Closeout for this change happens after all implementation/evidence tasks are complete:

1. Build the marker package with `nix build .#hermit-uhyve-marker --no-link -L`.
2. Run the marker fixture package/metadata contract check.
3. Run the ignored real Hermit/Uhyve product-path proof on a capable host using `.#uhyve` and the packaged marker image.
4. Run `scripts/test-harness.sh export` and `scripts/test-harness.sh check`.
5. Run `openspec validate package-hermit-uhyve-marker-fixture --strict` and `openspec validate --all --strict --json`.
6. Run `git diff --check`.
7. Archive, commit, push, and verify clean state.

The marker package build and contract check are prerequisite evidence only; runtime-host proof still requires Aspen `JobManager`/`WorkerPool` execution with marker `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED` in the product-visible receipt.
