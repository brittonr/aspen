## 1. Unit tests for existing infrastructure

- [x] 1.1 Add unit test for `compute_input_closure` — mock `nix-store -qR` output, verify dedup and basename extraction
- [x] 1.2 Add unit test for builder injection in `derivation_to_build_request` — verify builder store path added to `input_nodes` when missing
- [x] 1.3 Add unit test for `resolve_build_inputs` local fallback — verify placeholder Node created when PathInfoService returns None but path exists on disk
- [x] 1.4 Add unit test for output path computation — verify sandbox output path maps to correct `/nix/store/<hash>-<name>` target

## 2. Output registration via nix daemon

- [x] 2.1 Add `register_output_in_store()` function in `build_service.rs` that uses snix-store's nix daemon client to register a build output at the derivation's expected store path
- [x] 2.2 Add fallback path: if daemon protocol fails, log warning but don't fail the build (castore/PathInfoService is primary)
- [x] 2.3 Wire `register_output_in_store()` into `LocalStoreBuildService::do_build()` after output ingestion
- [x] 2.4 Add integration test (`#[ignore]`, requires bubblewrap + nix daemon): build trivial derivation, verify output at expected `/nix/store` path

## 3. Flake lock generation

- [x] 3.1 Add `ensure_flake_lock()` function in `eval.rs` — runs `nix flake lock` when `flake.lock` is missing
- [x] 3.2 Add unit test for `needs_flake_lock()` — temp dir with/without flake.lock
- [x] 3.3 Wire `ensure_flake_lock()` into `try_flake_eval_native()` before snix-eval attempt
- [x] 3.4 Add integration test (`#[ignore]`, requires nix): eval trivial flake after lock generation, verify valid .drv path

## 4. Self-build VM test

- [x] 4.1 Switch `ci-dogfood-self-build-test` to `full-aspen-node-plugins-snix-build` in flake.nix
- [x] 4.2 Add bubblewrap + `/nix/store` write access to VM configuration
- [x] 4.3 Add "native build path used" verification subtest — check node logs for bwrap execution, allow at most 1 subprocess fallback
- [x] 4.4 Update output verification — handle content-addressed path difference or use daemon-registered path
- [x] 4.5 Run full VM test end-to-end, verify binary output matches expected constants
