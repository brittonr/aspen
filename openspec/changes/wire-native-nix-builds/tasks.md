## 1. Derivation Path Resolution

- [x] 1.1 Add `resolve_drv_path(&self, payload) -> Result<PathBuf>` to `NixBuildWorker` in executor.rs — runs `nix eval --raw <flake_ref>.drvPath`, parses stdout as a `/nix/store/...drv` path, returns error on non-zero exit or timeout
- [x] 1.2 Add unit test for `resolve_drv_path` with a mock/fake nix binary that prints a known drv path

## 2. Native Build Dispatch

- [x] 2.1 Add `try_native_build()` method to `NixBuildWorker` behind `#[cfg(feature = "snix-build")]` — calls `resolve_drv_path`, reads and parses the `.drv` file, calls `build_service::execute_native()`, returns `Result<NixBuildOutput>`
- [x] 2.2 Wire `try_native_build()` into `execute_build()` — when `snix-build` feature is enabled and `has_native_builds()` is true, call `try_native_build()` first; on `Ok`, use that result; on `Err`, log warning and fall through to existing `spawn_nix_build()` subprocess path
- [x] 2.3 Remove `#[allow(dead_code)]` from `build_service::execute_native()` now that it has a caller

## 3. Native Output Upload

- [x] 3.1 After successful `try_native_build()`, call `upload_native_outputs()` from build_service.rs when SNIX pathinfo service is configured — wire the NativeBuildResult outputs through to the existing upload function

## 4. Worker Startup

- [x] 4.1 Find where `NixBuildWorker` is constructed in the node binary and add `worker.init_native_build_service().await` call after construction, gated on `#[cfg(feature = "snix-build")]`

## 5. Tests

- [x] 5.1 Add test: `execute_build()` with `snix-build` disabled compiles and takes subprocess path (existing behavior unchanged)
- [x] 5.2 Add test: `try_native_build()` returns error when native service is None — verify fallback occurs
- [x] 5.3 Verify `cargo check --features snix-build` compiles cleanly with no warnings
