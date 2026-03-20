## Context

The flake native build path currently uses ~2,270 lines of custom Rust across three files to replicate what Nix's flake machinery does:

```
call_flake.rs   (465 lines) — embed call-flake.nix, build override expressions
flake_lock.rs  (1017 lines) — parse flake.lock, resolve inputs, fetch via HTTP
fetch.rs        (788 lines) — FetchCache, curl subprocess, unpack tarballs, verify narHash
```

NixOS/flake-compat (`default.nix`, ~250 lines, MIT) does the same thing in pure Nix: parses flake.lock, fetches inputs via `fetchTarball`/`builtins.path`/`builtins.fetchGit`, calls the flake's outputs. snix-eval already implements the builtins flake-compat needs:

| flake-compat uses | snix-glue status |
|---|---|
| `fetchTarball { url; sha256; }` | ✅ `fetch_lazy` → lazy store path + on-demand download via `ingest_and_persist` |
| `builtins.path { path; sha256; }` | ✅ `import_helper` with hash verification |
| `builtins.fetchGit { url; rev; }` | ❌ `NotImplemented("fetchGit")` — raw git inputs fall back to subprocess |
| `builtins.fromJSON`, `import`, `readFile` | ✅ standard builtins |

snix-eval's `fetchTarball` is lazy: when `narHash` is provided (always true for locked inputs), it computes the store path and registers the fetch in `KnownPaths` without downloading. When `import` later reads from that store path, `SnixStoreIO::store_path_to_path_info` detects the pending fetch and calls `fetcher.ingest_and_persist` — downloading, unpacking, ingesting into castore, and creating a `PathInfo` entry. The content is then readable through `SnixStoreIO::open`.

## Goals / Non-Goals

**Goals:**

- Replace manual flake.lock parsing + HTTP fetching + call-flake.nix generation with embedded flake-compat
- Evaluation expression reduces to: `(import <flake-compat> { src = /path; }).outputs.<attr>.drvPath`
- GitHub, GitLab, tarball, path, and sourcehut inputs resolve via snix-eval's native builtins
- VM integration test proves zero-subprocess flake build for a tarball input
- Existing call-flake.rs/flake_lock.rs/fetch.rs code remains as fallback behind feature flag

**Non-Goals:**

- Native `git` input support (snix's `fetchGit` is unimplemented — falls back to subprocess)
- Building derivations with complex nixpkgs dependencies (requires stdenv in store)
- Replacing the `nix build` subprocess fallback entirely

## Decisions

### 1. Embed flake-compat as a string constant

Bundle `default.nix` from NixOS/flake-compat at a pinned commit as a `const &str` in a new `flake_compat.rs` module. Write it to a temp file before evaluation (snix-eval needs a file path for `import`).

**Why not fetch at runtime?** The file is 250 lines, stable, and MIT licensed. Embedding avoids network dependency during builds. Pin to a specific commit for reproducibility.

**Alternative considered:** Vendor as a file in the source tree. Rejected because a string constant is simpler to manage (no path resolution at runtime, visible in the code).

### 2. New eval method alongside existing one

Add `evaluate_flake_via_compat()` to `NixEvaluator` that:

1. Writes flake-compat's `default.nix` to a temp file
2. Constructs: `(import <temp>/default.nix { src = <flake_dir>; }).outputs.<attr>.drvPath`
3. Evaluates via `evaluate_with_store` (same snix-eval + snix-glue setup as npins)
4. Extracts `Derivation` from `KnownPaths` (identical to `evaluate_npins_derivation`)

`try_flake_eval_native` calls this instead of the current `evaluate_flake_derivation`. The current method stays as a cfg-gated fallback.

### 3. Build phase unchanged

`LocalStoreBuildService` and `resolve_build_inputs` work the same as the npins path. For trivial derivations (builder = `/bin/sh`, no nixpkgs deps), inputs are empty or local. For derivations that reference fetched sources as `input_sources`, those store paths are in `PathInfoService` (placed there by snix-eval's lazy fetch mechanism) — `resolve_build_inputs` finds them.

The LocalStoreBuildService copies inputs from `/nix/store/` on the host. For store paths that only exist in castore (fetched by snix-eval), the build will fall back to subprocess. This is the same limitation as the npins path.

### 4. VM test with local HTTP tarball

Same approach as the existing snix-native-build test: serve a synthetic tarball via `python3 -m http.server`, create a flake that depends on it as a `tarball` input, pre-compute narHash, generate flake.lock, submit ci_nix_build job, verify "zero subprocesses" in logs.

## Risks / Trade-offs

- **[Risk] snix-eval's fetchTarball has bugs for edge cases** → Mitigation: flake-compat is well-tested upstream; we're passing through to the same builtins. Test with a real tarball in the VM test.

- **[Risk] flake-compat upstream breaks API** → Mitigation: pin to a specific commit. The file has been stable since 2021 with only minor changes. Re-pin deliberately.

- **[Risk] SnixStoreIO lazy fetch doesn't trigger for some import patterns** → Mitigation: flake-compat uses standard `import (outPath + "/flake.nix")` which goes through `SnixStoreIO::open` → `store_path_to_path_info` → `ingest_and_persist`. This is snix-glue's designed-for path.

- **[Trade-off] `git` inputs still need subprocess fallback** → Acceptable. `fetchGit` is unimplemented in snix. GitHub/GitLab use tarball downloads (not git), so most real-world flake inputs work natively. Pure `git` inputs are uncommon.

- **[Trade-off] ~2,270 lines of Rust become dead code** → Keep behind a cfg flag initially. Remove in a follow-up once flake-compat path is proven stable.
