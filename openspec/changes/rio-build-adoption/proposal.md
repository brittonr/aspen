## Why

Aspen's snix integration stores blobs as whole objects in iroh-blobs with no sub-blob chunking, no cross-blob deduplication, and no verified streaming. The `chunks()` method returns an empty vec. For the dogfood use case — where Aspen stores its own build artifacts — incremental rebuilds re-store nearly identical NARs without sharing common content. rio-build's chunked CAS (FastCDC + BLAKE3) and circuit breaker patterns solve problems Aspen already faces at the storage and resilience layers.

## What Changes

- Implement chunked blob storage in `aspen-snix` using FastCDC content-defined chunking with BLAKE3 hashing, so `BlobService::chunks()` returns real `ChunkMeta` and cross-blob deduplication works
- Add a generic circuit breaker to `aspen-core` (modeled on rio-build's `CacheCheckBreaker`) and wire it into snix service calls, irpc castore client, and CI worker upload paths
- Add tracey `r[impl]` / `r[verify]` annotations to existing snix crate code and Verus specs, with CI enforcement via `tracey query validate`
- Add fuzz targets for snix-facing parsers (NAR ingestion, protobuf directory deserialization, pathinfo encoding) with 30-second smoke tier in `nix flake check`
- Add backpressure hysteresis to the castore irpc server to shed load when Raft is overloaded
- Derive coupled timeouts from their base intervals in `aspen-constants`

## Capabilities

### New Capabilities

- `chunked-blob-storage`: FastCDC content-defined chunking for blobs with BLAKE3 per-chunk hashes, chunk-level deduplication across store paths, and proper `ChunkMeta` returned from `BlobService::chunks()`
- `circuit-breaker`: Generic circuit breaker primitive in `aspen-core` with consecutive-failure threshold, half-open probe, auto-close timeout — usable by any component calling a fallible external dependency
- `tracey-coverage`: Tracey `r[impl]` / `r[verify]` annotations linking spec requirements to code and tests, with CI gate on broken references
- `snix-fuzz-targets`: Fuzz targets for snix protocol parsers with smoke tier (30s) in `nix flake check` and nightly tier (10min) for deeper coverage
- `castore-backpressure`: Queue-depth hysteresis on the castore irpc server — activate rejection at 80% queue capacity, deactivate at 60% — to prevent cascading failures under CI load
- `derived-timeouts`: Derive heartbeat/staleness timeouts from base interval × missed-count so coupled constants stay in sync automatically

### Modified Capabilities

## Impact

- `aspen-snix`: Major — chunked blob service, tracey annotations, fuzz targets
- `aspen-core`: New `circuit_breaker` module
- `aspen-castore`: Backpressure on irpc server, circuit breaker on irpc client
- `aspen-constants`: Derived timeout pattern for coupled constants
- `aspen-ci-executor-nix` / `aspen-ci-executor-shell`: Circuit breaker on snix upload path
- `aspen-nix-handler`: Circuit breaker on Raft KV calls
- `flake.nix`: Fuzz smoke tier, tracey CI check
- Workspace `Cargo.toml`: `fastcdc` dependency
