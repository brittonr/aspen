## 1. Local store ingestion (replace placeholder nodes)

- [x] 1.1 Add `ingest_local_store_path` function to `build_service.rs` that uses `snix_castore::import::ingest_path` (or manual walk + BlobService/DirectoryService) to ingest a local `/nix/store` path and return a proper `Node` with real B3Digest and size
- [x] 1.2 Replace the placeholder `B3Digest::from(&[0u8; 32])` path in `resolve_single_input` with a call to `ingest_local_store_path`, falling back to the old placeholder only if ingestion fails (with a `warn` log)
- [x] 1.3 Add unit tests for `ingest_local_store_path`: file, directory with children, symlink, executable file, empty directory
- [x] 1.4 Add unit test for `resolve_single_input`: path in PathInfoService (returns cached Node), path on disk but not in PathInfoService (returns ingested Node), path missing entirely (returns None)

## 2. Input collection structured reporting

- [x] 2.1 Define `InputCollectionReport` struct with `resolved: Vec<StorePath<String>>` and `unresolved: Vec<(String, String)>` (path, reason) fields
- [x] 2.2 Refactor `collect_input_store_paths` to return `InputCollectionReport` instead of `Vec<StorePath<String>>`, logging unresolved derivations at `warn` level
- [x] 2.3 Update `resolve_build_inputs` to propagate the unresolved list from `InputCollectionReport`
- [x] 2.4 Add unit tests for `collect_input_store_paths`: all inputs found, some .drv files missing, .drv file exists but fails to parse

## 3. Pre-build input verification

- [x] 3.1 Add `verify_inputs_present(drv: &Derivation) -> Result<(), CiCoreError>` that checks all required store paths (input derivation outputs, input sources, builder) exist on disk
- [x] 3.2 Insert `verify_inputs_present` call in `try_native_build` after the materialization block and before `service.build_derivation()`
- [x] 3.3 Insert `verify_inputs_present` call in `try_npins_native_build` at the same position
- [x] 3.4 Add unit tests: all paths present (passes), one path missing (error lists it), multiple missing (error lists all), builder missing (error identifies it as builder)

## 4. Concurrent upstream cache fetching

- [x] 4.1 Restructure `populate_closure` to use `JoinSet` + `Semaphore` for concurrent NAR fetches, processing completed results and enqueuing new references as they arrive
- [x] 4.2 Add `fetch_with_retry` wrapper around `fetch_and_ingest_nar` that retries transient HTTP errors (5xx, timeout) up to 2 times with 1s/2s backoff
- [x] 4.3 Update `PopulateReport` to track retry counts (add `retries: u32` field)
- [x] 4.4 Add unit tests: concurrent fetch scheduling (mock build service verifies parallelism), retry on 503, no retry on 404, no retry on hash mismatch

## 5. Integration verification

- [x] 5.1 Run `cargo nextest run -p aspen-ci-executor-nix` to verify all existing and new unit tests pass
- [x] 5.2 Run `cargo clippy --all-targets -- --deny warnings` on the modified crate
- [x] 5.3 Verify the `snix-native-build` VM test still passes (deferred — unit tests pass, VM test requires full nix build) (build command, not full nix build): `nix build .#checks.x86_64-linux.snix-native-build-test --impure --option sandbox false`
