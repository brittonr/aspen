# Nix Integration Architecture

Aspen integrates with the Nix ecosystem through the [snix](https://git.snix.dev/snix/snix)
project — a Rust reimplementation of Nix's store, evaluator, and builder. This replaces
subprocess calls to the `nix` CLI with in-process Rust code that shares Aspen's distributed
storage directly.

## Layered Feature Flags

Each layer builds on the previous one. Enable only what you need:

```
snix            Base: content-addressed store (BlobService, DirectoryService, PathInfoService)
├── snix-http   nar-bridge HTTP server (Nix binary cache protocol)
├── snix-daemon nix-daemon Unix socket protocol (nix path-info, nix copy)
├── snix-eval   In-process Nix evaluation (snix-eval, snix-glue, snix-serde)
│   └── snix-build  Native build execution (bubblewrap/OCI sandbox)
```

The `full` workspace feature enables all of them.

## Crate Map

| Crate | Purpose |
|-------|---------|
| `aspen-snix` | Raft-backed `BlobService`, `DirectoryService`, `PathInfoService` trait impls |
| `aspen-castore` | Content-addressed store primitives shared across snix crates |
| `aspen-snix-bridge` | Standalone binary: gRPC server + nix-daemon socket backed by Aspen's store |
| `aspen-nix-cache-gateway` | Standalone binary: HTTP binary cache (nar-bridge axum router) |
| `aspen-ci-executor-nix` | Nix build worker for CI pipelines (eval, build, cache upload) |

## Data Flow

### Binary Cache (snix-http)

```
nix build --substituters http://host:8380
  → aspen-nix-cache-gateway (nar-bridge axum router)
    → PathInfoService::get()   → Raft KV lookup
    → BlobService::open_read() → iroh-blobs download
```

### nix-daemon Protocol (snix-daemon)

```
nix copy --to unix:///tmp/aspen.sock /nix/store/...
  → aspen-snix-bridge (nix-daemon listener)
    → NixDaemonIO::add_to_store_nar() → ingest NAR → BlobService + DirectoryService
    → NixDaemonIO::query_path_info()  → PathInfoService lookup
```

### CI Build Pipeline

The CI executor has three build paths, tried in priority order:

#### 1. Zero-subprocess (npins projects)

For projects with `npins/sources.json`. No `nix` binary involved at all.

```
git push → Forge gossip → CI trigger
  → NixBuildWorker::try_npins_native_build()
    1. snix-eval: import default.nix → derivationStrict → Derivation from KnownPaths
    2. LocalStoreBuildService: Derivation → BuildRequest → bubblewrap sandbox
    3. Upload outputs directly to PathInfoService + BlobService (from build result)
    4. nar-bridge serves built paths to downstream consumers
```

snix-eval resolves the `.drvPath` attribute, which triggers `derivationStrict` internally.
The `Derivation` object is extracted from snix-glue's `KnownPaths` — no `.drv` file
needs to exist on disk. The entire eval→build→upload pipeline runs in-process.

#### 2. Native build via flake-compat (flake projects, zero subprocesses)

For flake projects when `snix-build` feature is enabled. Uses embedded
[NixOS/flake-compat](https://github.com/NixOS/flake-compat) evaluated through
snix-eval — no `nix` subprocess needed for github, gitlab, tarball, path, or
sourcehut inputs.

```
git push → Forge gossip → CI trigger
  → NixBuildWorker::try_native_build()
    → try_flake_eval_native() [primary: flake-compat]
      1. evaluate_flake_via_compat(): import flake-compat { src = <dir>; }
      2. snix-eval's fetchTarball resolves inputs (HTTP download, narHash verify)
      3. derivationStrict → Derivation extracted from KnownPaths
    → materialize_store_paths(): missing inputs fetched from PathInfoService → disk
    → LocalStoreBuildService: Derivation → BuildRequest → bubblewrap sandbox
    → Upload outputs directly to PathInfoService + BlobService
    → nar-bridge serves built paths to downstream consumers
```

snix-eval's `fetchTarball` builtin handles HTTP downloads, tarball unpacking,
narHash verification, and store path computation internally. `SnixStoreIO`
triggers lazy fetches on-demand when `import` reads from a store path.

**Git inputs:** `builtins.fetchGit` is implemented via a local patch to
snix-glue (see `vendor/snix-glue/PATCHES.md`). Flakes with `type = "git"`
inputs are resolved fully in-process through both the call-flake.nix and
flake-compat evaluation paths.

Build execution uses `LocalStoreBuildService`, which copies inputs from the
local `/nix/store` into the bubblewrap sandbox via `cp -a`. This replaces
upstream snix-build's FUSE-based input mounting, which fails under systemd's
`ProtectSystem=strict`.

**Input closure materialization:** When input store paths are missing from the
local `/nix/store`, the executor materializes them from the cluster's castore
services (PathInfoService + BlobService + DirectoryService) by walking the
castore Node tree and writing files/directories/symlinks directly to disk.
This replaces the previous `nix-store --realise` subprocess fallback.

If castore cannot resolve all paths and the `nix-cli-fallback` feature is
enabled, `nix-store --realise` is used as a last-resort fallback.

#### 3. Subprocess fallback

When `snix-build` is unavailable or native builds fail.

```
git push → Forge gossip → CI trigger
  → NixBuildWorker::execute_build() (subprocess fallback)
    1. nix build <flake_ref> --no-link --print-out-paths
    2. Parse output paths from stdout
    3. Upload to PathInfoService via NAR archive (read from disk)
    4. nar-bridge serves built paths
```

Gated by the `nix-cli-fallback` feature on `aspen-ci-executor-nix`.

### snix-serde Config Parsing (snix-eval)

CI pipeline definitions can be written in Nix (`.aspen/ci.nix`) and deserialized
directly into Rust structs via `snix_serde::from_str`. Pure evaluation is enforced —
no I/O builtins are available during config parsing.

## Storage Architecture

All snix services share the same underlying Raft-replicated storage:

- **BlobService** → iroh-blobs (content-addressed by BLAKE3 hash)
- **DirectoryService** → Raft KV (directory tree nodes keyed by digest)
- **PathInfoService** → Raft KV (store path metadata: NAR hash, size, references, signatures)

The gRPC bridge (`aspen-snix-bridge`) exposes these services to external `snix-store`
and `nix` CLI tools. The HTTP gateway (`aspen-nix-cache-gateway`) serves them via the
standard Nix binary cache protocol.

## NixOS VM Tests

| Test | What it validates |
|------|-------------------|
| `snix-bridge-test` | gRPC bridge: import files/dirs via snix-store, verify store paths |
| `snix-bridge-virtiofs-test` | VirtioFS mount of /nix/store backed by bridge, file round-trip through microVM |
| `snix-store-test` | snix-store operations against Aspen's PathInfoService |
| `snix-boot-test` | Full boot chain: snix-store virtiofs → cloud-hypervisor microVM |
| `nix-cache-gateway-test` | HTTP cache: nix-cache-info, narinfo, signing, 404/400 handling |
| `e2e-push-build-cache-test` | Full pipeline: Forge push → CI auto-trigger → build → cache gateway serves |
| `snix-daemon-test` | nix-daemon protocol: path-info, valid-path, copy via Unix socket |
| `snix-native-build-test` | Native bwrap build: flake eval subprocess → LocalStoreBuildService → PathInfoService upload → cache gateway narinfo |
| `snix-flake-native-build-test` | Zero-subprocess flake build: flake-compat + snix-eval fetchTarball resolves tarball input → bwrap build → cache gateway narinfo |
| `npins-native-eval-test` | Zero-subprocess build: snix-eval resolves Derivation in-memory → bwrap build → confirms "zero subprocesses" in logs |
