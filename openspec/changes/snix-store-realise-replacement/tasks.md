## 1. Core materializer module

- [ ] 1.1 Create `crates/aspen-ci-executor-nix/src/materialize.rs` with `MaterializeReport` struct and `materialize_store_paths` function signature
- [ ] 1.2 Implement `materialize_node` — recursive castore Node walker that writes files/dirs/symlinks to disk from BlobService/DirectoryService
- [ ] 1.3 Implement `materialize_single_path` — looks up PathInfo by store path digest, calls `materialize_node` on the root Node
- [ ] 1.4 Implement `materialize_store_paths` — iterates missing paths, calls `materialize_single_path`, aggregates into MaterializeReport
- [ ] 1.5 Add Tiger Style resource bounds: MAX_MATERIALIZE_PATHS (10,000), MAX_SINGLE_BLOB_SIZE (256 MB), MAX_DIRECTORY_DEPTH (256)
- [ ] 1.6 Register `mod materialize` in `lib.rs` behind `#[cfg(feature = "snix-build")]`

## 2. Integration into executor

- [ ] 2.1 Thread PathInfoService/BlobService/DirectoryService into `try_native_build` (already available via `self.config.snix_*` fields)
- [ ] 2.2 Replace the `nix-store --realise` fallback block in `try_native_build` Step 2 with a call to `materialize_store_paths` for missing paths
- [ ] 2.3 Gate the remaining `nix-store --realise` subprocess behind `nix-cli-fallback` feature as a last-resort fallback
- [ ] 2.4 Update SUBPROCESS comments: change SUBPROCESS-FALLBACK to SUBPROCESS-LAST-RESORT for the nix-cli-fallback gated code

## 3. Unit tests

- [ ] 3.1 Test `materialize_node` with a File node — write blob to temp dir, verify content and permissions
- [ ] 3.2 Test `materialize_node` with a Directory node — create nested structure from in-memory DirectoryService
- [ ] 3.3 Test `materialize_node` with a Symlink node — verify symlink target
- [ ] 3.4 Test `materialize_single_path` — full PathInfo → filesystem round-trip with in-memory services
- [ ] 3.5 Test `materialize_store_paths` — multiple paths, some present (skipped), some materialized, verify report counts
- [ ] 3.6 Test resource bounds — exceed MAX_MATERIALIZE_PATHS, verify error

## 4. Integration tests

- [ ] 4.1 Test that `try_native_build` skips subprocess when castore has all inputs (existing behavior, confirm still works)
- [ ] 4.2 Test that `try_native_build` materializes from castore when paths are missing from disk but present in PathInfoService
- [ ] 4.3 Test MaterializeReport logging — verify structured log output includes counts

## 5. Cleanup

- [ ] 5.1 Run `cargo clippy` and `cargo nextest run` for the full test suite
- [ ] 5.2 Update `docs/nix-integration.md` subprocess escape table to reflect the replacement
- [ ] 5.3 Commit with descriptive message
