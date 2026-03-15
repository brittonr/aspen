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

### CI Build Pipeline (snix-eval + snix-build)

```
git push → Forge gossip → CI trigger
  → NixBuildWorker
    1. snix-eval: evaluate flake.nix → Derivation
    2. snix-build: Derivation → BuildRequest → bubblewrap sandbox → output paths
    3. Upload outputs to PathInfoService + BlobService
    4. nar-bridge serves built paths to downstream consumers
```

When `snix-build` is unavailable, the executor falls back to the `nix build` subprocess
(gated by the `nix-cli-fallback` feature on `aspen-ci-executor-nix`).

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
| `snix-bridge-virtiofs-test` | VirtioFS mount of /nix/store backed by bridge |
| `snix-store-test` | snix-store operations against Aspen's PathInfoService |
| `snix-boot-test` | Full boot chain: snix-store virtiofs → cloud-hypervisor microVM |
| `nix-cache-gateway-test` | HTTP cache: nix-cache-info, narinfo, signing, 404/400 handling |
| `e2e-push-build-cache-test` | Full pipeline: Forge push → CI auto-trigger → build → cache gateway serves |
| `snix-daemon-test` | nix-daemon protocol: path-info, valid-path, copy via Unix socket |
