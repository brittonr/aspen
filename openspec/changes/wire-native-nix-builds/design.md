## Context

`execute_build()` in `crates/aspen-ci-executor-nix/src/executor.rs` is the single entry point for all CI nix builds. It unconditionally spawns `nix build` as a subprocess. Meanwhile, `build_service.rs` has a complete `NativeBuildService` that wraps snix-build's `BuildService::do_build()` with bubblewrap/OCI sandboxing, input resolution from `PathInfoService`, and output upload. The `execute_native()` function already converts native build results into the same `NixBuildOutput` format used by the subprocess path. All that's missing is the dispatch logic in `execute_build()` and the eval→derivation bridge.

The gap between eval and build: snix-eval evaluates Nix expressions to `Value` objects, but `NativeBuildService::build_derivation()` takes a `Derivation` struct. A flake attribute like `packages.x86_64-linux.default` evaluates to a derivation value whose `.drvPath` points to a `.drv` file in `/nix/store/`. We need to get that `.drv` path and parse it.

## Goals / Non-Goals

**Goals:**

- `execute_build()` tries the native snix-build path first when the `snix-build` feature is enabled and `NativeBuildService` is initialized
- Falls back to subprocess `nix build` on any native-path failure
- The fallback is silent from the caller's perspective — same `NixBuildOutput` either way
- Worker startup calls `init_native_build_service()` so the service is ready before the first job
- Zero behavior change when `snix-build` feature is off

**Non-Goals:**

- Eliminating the `nix` CLI dependency entirely (we still use `nix eval .drvPath` for the eval→drv bridge)
- CA derivation support (separate effort)
- Replacing the SNIX upload path in `snix.rs` — native builds use `upload_native_outputs()` from build_service.rs, subprocess builds continue using `upload_store_paths_snix()`

## Decisions

### 1. Hybrid eval→drv bridge via `nix eval --raw`

**Choice**: Use `nix eval --raw .#attr.drvPath` subprocess to resolve the drv path, then `parse_derivation()` on the `.drv` file from disk.

**Alternatives considered**:

- *Pure snix-eval*: snix-eval's `derivationStrict` builtin produces a derivation value, but extracting the `.drvPath` string from a `Value::Attrs` requires threading through snix-glue internals that aren't public API. The value is an attrset with a `drvPath` key, but getting from there to a `Derivation` struct means either reading the `.drv` from the store (same as our approach) or duplicating snix-glue's derivation-to-store-path logic.
- *Instantiate subprocess*: `nix-instantiate` or `nix eval` both work; `nix eval --raw .#attr.drvPath` is the modern equivalent and works with flake refs directly.

**Rationale**: The hybrid approach keeps the actual build in-process (the expensive part) while using the Nix CLI for the cheap eval step (~100ms). This avoids coupling to snix-eval internals that may change. When snix-eval matures to expose a direct eval→Derivation path, we can swap the bridge without changing the rest of the pipeline.

### 2. Native-first with subprocess fallback

**Choice**: Try native path first, catch all errors, fall back to subprocess.

```
execute_build()
  ├── [snix-build enabled + native service initialized]
  │   ├── resolve_drv_path(payload)        // nix eval --raw .drvPath
  │   ├── parse_derivation(drv_bytes)      // read .drv from /nix/store
  │   ├── execute_native(service, drv, tx) // snix-build sandbox
  │   ├── upload_native_outputs(...)       // PathInfoService
  │   └── on error → fall through to subprocess
  │
  └── spawn_nix_build(payload, flake_ref)  // existing subprocess path
```

**Rationale**: The subprocess path is battle-tested. Native builds are new. Fallback guarantees no regression for existing users while gaining native builds where the sandbox is available.

### 3. drv path resolution as a separate method

**Choice**: Add `resolve_drv_path(&self, payload) -> Result<PathBuf>` to `NixBuildWorker` that runs `nix eval --raw` and returns the `/nix/store/...drv` path.

**Rationale**: Isolates the eval→drv bridge so it can be swapped to pure snix-eval later without touching the dispatch logic. Also testable independently.

### 4. Worker startup initialization

**Choice**: Call `worker.init_native_build_service().await` in the node binary after constructing the worker, before registering it with the job system.

**Rationale**: `init_native_build_service()` already exists and handles sandbox detection (bubblewrap → OCI → dummy). If it fails, the worker still functions via subprocess. No retry needed — if bwrap isn't available at startup, it won't appear later.

## Risks / Trade-offs

- **[eval subprocess adds latency]** → ~100ms per build for `nix eval --raw`. Negligible compared to build times (minutes). Eliminable when snix-eval exposes a direct Derivation API.
- **[.drv file must exist on local disk]** → `nix eval` writes the `.drv` to the local store. For remote flakes, Nix fetches and evaluates. This is the same behavior as the current subprocess path.
- **[native build may produce different output hash than subprocess]** → Both use the same derivation, so outputs should be identical. The `test_native_vs_subprocess_parity` integration test (already scaffolded) validates this. If hashes diverge, fallback to subprocess ensures correctness.
- **[bubblewrap not available in all environments]** → `init_native_build_service()` already handles this: bwrap → OCI → dummy. When dummy, `has_native_builds()` returns false and we skip straight to subprocess.
