## Context

`aspen-ci-executor-nix` has a three-tier build pipeline: npins native (zero subprocesses), flake-compat native (zero subprocesses), and `nix build` subprocess fallback. The native paths work for simple derivations, but 8 subprocess escape hatches remain. These fall into three categories:

1. **Store queries** — `nix path-info --json`, `nix-store -qR` (PathInfoService can answer these)
2. **NAR I/O** — `nix nar dump-path` (snix-store SimpleRenderer can do this)
3. **Fetching** — `curl` for tarballs, `nix-store --realise` for missing inputs (reqwest + upstream cache can replace these)
4. **Eval/build** — `nix eval`, `nix build`, `nix flake lock` (already replaced by snix-eval for supported cases; remaining calls are genuine fallbacks)

Category 4 calls are already fallbacks for when snix-eval hits unsupported features (IFD, exotic builtins). Those stay as `nix-cli-fallback` gated code. Categories 1-3 are the targets — they have pure in-process replacements.

## Goals / Non-Goals

**Goals:**

- Every `Command::new("nix*")` call in cache.rs, build_service.rs, and fetch.rs is replaced with in-process equivalents
- Every remaining nix CLI call in executor.rs and eval.rs is gated behind `#[cfg(feature = "nix-cli-fallback")]`
- A new `UpstreamCacheClient` can bootstrap a cluster's PathInfoService from cache.nixos.org
- The `snix-pure-build-test` NixOS VM test passes for a derivation with actual transitive dependencies (not just a trivial /bin/sh builder)

**Non-Goals:**

- Replacing `git` subprocess calls (git is a legitimate build dependency, not a nix dependency)
- Implementing `builtins.fetchGit` natively (already done via snix-glue vendored patch)
- Making IFD work without nix CLI (import-from-derivation is fundamentally a subprocess operation in snix-eval)
- Signature verification of upstream cache responses (trust-on-first-use is fine for now; signing verification is a separate concern)

## Decisions

### 1. UpstreamCacheClient as a standalone struct in aspen-ci-executor-nix

The client lives in a new `upstream_cache.rs` module. It holds a `reqwest::Client`, a list of cache URLs with public keys, and a reference to the cluster's PathInfoService/BlobService/DirectoryService for ingestion.

**Why not in aspen-snix?** The upstream cache client is a CI executor concern — it's about bootstrapping inputs for builds. aspen-snix provides the storage traits; the executor decides when and how to populate them.

**Alternative considered:** Shared crate used by both nix-cache-gateway and ci-executor. Rejected — the gateway *serves* narinfo, while this client *fetches* narinfo. Different concerns, no shared code beyond parsing, and nix-compat already provides narinfo parsing.

### 2. reqwest for HTTP (not hyper directly)

reqwest is already a transitive dependency via iroh. It handles TLS, redirects, timeouts, and streaming out of the box. Using hyper directly would duplicate all that plumbing.

The `curl` subprocess in `fetch.rs` is replaced with `reqwest::get()` with `.timeout()` and streaming body read into the `flate2::read::GzDecoder` + `tar::Archive` pipeline. The `FetchCache` semaphore stays unchanged.

### 3. Closure walk via BFS on PathInfoService references

`compute_input_closure_via_pathinfo` already exists and does this. The change is:

- Remove `compute_input_closure` (nix-store -qR version) from the default code path
- Move it behind `#[cfg(feature = "nix-cli-fallback")]`
- When PathInfoService can't resolve a path, try `UpstreamCacheClient::fetch_narinfo()` to populate it, then retry
- If upstream cache also can't resolve, fall back to subprocess (if enabled) or return partial closure

### 4. NAR export via SimpleRenderer

`upload_store_paths` in cache.rs currently does:

```
nix nar dump-path <store_path> | hash + upload to blob store
```

Replace with:

```rust
let renderer = SimpleRenderer::new(blob_service.clone(), directory_service.clone());
let (nar_size, nar_hash) = renderer.calculate_nar(&pathinfo.node).await?;
```

This is what `upload_native_outputs` in build_service.rs already does for native build outputs. The change extends that pattern to the legacy upload path in cache.rs.

### 5. Feature flag restructuring

Current:

- `snix-build` enables native builds
- `nix-cli-fallback` enables subprocess last resort for input materialization

After:

- `snix-build` enables native builds (unchanged)
- `nix-cli-fallback` gates ALL subprocess calls: `nix eval`, `nix build`, `nix path-info`, `nix nar dump-path`, `nix-store -qR`, `nix-store --realise`, `nix flake lock`
- Default build (no `nix-cli-fallback`) is fully in-process

## Risks / Trade-offs

**[Risk] Upstream cache latency for large closures** — Fetching hundreds of narinfos from cache.nixos.org adds network round trips. → Mitigation: batch narinfo requests where possible, cache resolved narinfos in memory for the duration of the build, and the paths persist in PathInfoService for subsequent builds.

**[Risk] NAR hash compatibility** — SimpleRenderer must produce byte-identical NAR output to `nix nar dump-path`. → Mitigation: snix-store's NAR renderer is well-tested upstream. Add a round-trip property test: ingest path → render NAR → compare hash with known-good value.

**[Risk] reqwest vs curl behavioral differences** — curl handles edge cases (HTTP/1.0 servers, weird redirects, proxy auth) that reqwest might not. → Mitigation: all current fetch targets are GitHub/GitLab archives and cache.nixos.org, which are standard HTTPS. The `nix-cli-fallback` feature remains available for exotic environments.

**[Risk] Breaking existing CI pipelines that rely on subprocess path** — Some deployments might not have all inputs in PathInfoService. → Mitigation: `nix-cli-fallback` feature is still available. The change makes zero-subprocess the *default*, not the *only* option.

## Open Questions

- Should `UpstreamCacheClient` verify narinfo signatures against configured trusted public keys? The nix-cache-gateway already does signing for *outbound* narinfo. Verifying *inbound* upstream narinfo adds security but also complexity. Could be a follow-up.
- Should reqwest share a connection pool with iroh's existing HTTP client, or use its own? Separate pool is simpler, shared pool reduces connection overhead.
